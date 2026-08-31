//! Stress tests for the generator thread-transfer boundary.
//!
//! Generators run their body on a dedicated OS thread and move `Rc`-bearing VM
//! state across it under `unsafe impl Send` (see `napi_vm::generator_transfer`
//! for the proof). The proof's third invariant -- that a generator thread's
//! exit is *joined*, never concurrent with the main thread -- is the one that
//! ordinary channel discipline does not provide, and it is what these tests
//! exercise: completion, abandonment mid-suspension, nesting, and cloning.
//!
//! A data race on a non-atomic refcount is not reliably observable by running
//! the program, so these are not proof of soundness on their own. They are
//! there to (a) make the paths the proof depends on actually execute, in bulk,
//! and (b) give a sanitizer or Miri run something to chew on. Under ordinary
//! `cargo test` a regression here shows up as a use-after-free, a double free,
//! a hang in `join`, or a leaked thread.
//!
//! **These tests pass, but the boundary is not sound.** Under ThreadSanitizer
//! this file still reports roughly one data race per full run, down from ~7.6
//! before the joins in `GeneratorInner::join_thread` were added. See
//! `napi_vm::generator_transfer` for the measurement, the surviving race's
//! shape, and the command to reproduce it. Do not read a green `cargo test`
//! here as evidence that generators are thread-safe.

#![cfg(not(target_arch = "wasm32"))]

use napi_vm::interpreter::Interpreter;
use napi_vm::lexer::Lexer;
use napi_vm::parser::Parser;

/// Evaluate `source` in a fresh interpreter and return the result as a string.
fn run(source: &str) -> String {
    let mut interp = Interpreter::with_builtins();
    let toks = Lexer::new(source).tokenize_with_spans();
    let stmts = Parser::new_with_spans(toks).parse();
    let value = interp.run(&stmts).expect("execution failed");
    interp.vs(&value).unwrap_or_default()
}

/// Evaluate `source`, expecting it to fail.
fn run_expecting_error(source: &str) {
    let mut interp = Interpreter::with_builtins();
    let toks = Lexer::new(source).tokenize_with_spans();
    let stmts = Parser::new_with_spans(toks).parse();
    assert!(interp.run(&stmts).is_err(), "expected an error");
}

// ── completion ───────────────────────────────────────────────────────

#[test]
fn many_generators_run_to_completion() {
    // Each generator spawns and joins a thread. Running a few hundred in one
    // interpreter catches handle leaks and joins that never complete.
    let out = run(r#"
        function* range(n) { let i = 0; while (i < n) { yield i; i = i + 1; } }
        let total = 0;
        let g = 0;
        while (g < 200) {
            for (const v of range(5)) { total = total + v; }
            g = g + 1;
        }
        total;
    "#);
    assert_eq!(out, "2000");
}

#[test]
fn generators_sharing_one_closure_environment() {
    // The closure `Env` is an `Rc<RefCell<_>>` held by the main thread *and*
    // every generator thread. This is the refcount most likely to be raced on
    // teardown, so make many threads release it while the main thread keeps
    // reading and writing the same scope.
    let out = run(r#"
        let shared = 0;
        function* bump() { shared = shared + 1; yield shared; shared = shared + 1; yield shared; }
        let seen = 0;
        let i = 0;
        while (i < 200) {
            const g = bump();
            seen = seen + g.next().value;
            seen = seen + g.next().value;
            shared = shared + 1;
            i = i + 1;
        }
        shared;
    "#);
    assert_eq!(out, "600");
}

// ── abandonment ──────────────────────────────────────────────────────

#[test]
fn generators_abandoned_while_suspended() {
    // Dropped mid-body: the thread is parked in `recv()` and only unwinds when
    // the sender goes away. Without a join, that unwind -- and its `Rc`
    // releases -- runs concurrently with the loop below.
    let out = run(r#"
        function* forever() { let i = 0; while (true) { yield i; i = i + 1; } }
        let i = 0;
        let sum = 0;
        while (i < 200) {
            const g = forever();
            sum = sum + g.next().value;
            sum = sum + g.next().value;
            i = i + 1;
        }
        sum;
    "#);
    assert_eq!(out, "200");
}

#[test]
fn generators_abandoned_by_breaking_out_of_for_of() {
    let out = run(r#"
        function* forever() { let i = 0; while (true) { yield i; i = i + 1; } }
        let sum = 0;
        let i = 0;
        while (i < 200) {
            for (const v of forever()) { if (v > 2) { break; } sum = sum + v; }
            i = i + 1;
        }
        sum;
    "#);
    assert_eq!(out, "600");
}

#[test]
fn generators_never_started_are_dropped_cleanly() {
    // No `next()` at all: the thread is never spawned, so `join_thread` must
    // cope with a `None` handle.
    let out = run(r#"
        function* g() { yield 1; }
        let i = 0;
        while (i < 500) { const unused = g(); i = i + 1; }
        i;
    "#);
    assert_eq!(out, "500");
}

// ── nesting ──────────────────────────────────────────────────────────

#[test]
fn nested_generators_drive_one_another() {
    // An outer generator thread drives an inner one, so a generator thread is
    // itself the "main thread" of another transfer boundary.
    let out = run(r#"
        function* inner(n) { let i = 0; while (i < n) { yield i; i = i + 1; } }
        function* outer(n) {
            for (const v of inner(n)) { yield v * 2; }
        }
        let sum = 0;
        let r = 0;
        while (r < 50) {
            for (const v of outer(4)) { sum = sum + v; }
            r = r + 1;
        }
        sum;
    "#);
    assert_eq!(out, "600");
}

#[test]
fn deeply_nested_generators() {
    let out = run(r#"
        function* a() { yield 1; yield 2; }
        function* b() { for (const v of a()) { yield v + 1; } }
        function* c() { for (const v of b()) { yield v + 1; } }
        function* d() { for (const v of c()) { yield v + 1; } }
        let sum = 0;
        let i = 0;
        while (i < 50) { for (const v of d()) { sum = sum + v; } i = i + 1; }
        sum;
    "#);
    assert_eq!(out, "450");
}

#[test]
fn nested_generators_abandoned_mid_flight() {
    // Breaking out of the outer loop abandons both threads at once.
    let out = run(r#"
        function* inner() { let i = 0; while (true) { yield i; i = i + 1; } }
        function* outer() { for (const v of inner()) { yield v; } }
        let sum = 0;
        let i = 0;
        while (i < 100) {
            for (const v of outer()) { if (v > 1) { break; } sum = sum + v; }
            i = i + 1;
        }
        sum;
    "#);
    assert_eq!(out, "100");
}

// ── cloning ──────────────────────────────────────────────────────────

#[test]
fn cloned_generator_values_share_one_thread() {
    // `Value::Generator` is `Rc<RefCell<GeneratorInner>>`; clones must observe
    // the same progress and must not join the same handle twice.
    let out = run(r#"
        function* counter() { yield 1; yield 2; yield 3; }
        let sum = 0;
        let i = 0;
        while (i < 100) {
            const g = counter();
            const alias = g;
            sum = sum + g.next().value;
            sum = sum + alias.next().value;
            sum = sum + g.next().value;
            i = i + 1;
        }
        sum;
    "#);
    assert_eq!(out, "600");
}

#[test]
fn generators_stored_in_structures_outlive_their_scope() {
    let out = run(r#"
        function* g(n) { yield n; yield n + 1; }
        const held = [];
        let i = 0;
        while (i < 100) { held.push(g(i)); i = i + 1; }
        let sum = 0;
        let j = 0;
        while (j < 100) { sum = sum + held[j].next().value; j = j + 1; }
        sum;
    "#);
    assert_eq!(out, "4950");
}

// ── failure paths ────────────────────────────────────────────────────

#[test]
fn a_throwing_generator_body_still_joins() {
    // The thread sends `Threw` and then unwinds; the join has to happen on the
    // error path too, not just on clean return.
    let mut i = 0;
    while i < 100 {
        run_expecting_error(
            r#"
            function* boom() { yield 1; throw new Error("boom"); }
            const g = boom();
            g.next();
            g.next();
        "#,
        );
        i += 1;
    }
}

#[test]
fn a_generator_exhausting_the_loop_budget_still_joins() {
    let mut interp = Interpreter::with_builtins();
    interp.set_loop_budget(10_000);
    let source = r#"
        function* forever() { while (true) { yield 1; } }
        const g = forever();
        let n = 0;
        while (true) { g.next(); n = n + 1; }
    "#;
    let toks = Lexer::new(source).tokenize_with_spans();
    let stmts = Parser::new_with_spans(toks).parse();
    assert!(interp.run(&stmts).is_err(), "expected the budget to trip");
}
