//! Backend smoke tests for the coroutines that generators and async bodies
//! run on.
//!
//! These drive `corosensei` *directly*, with no interpreter in the way. That
//! is the point: `generator_stress.rs` exercises the same machinery through
//! several thousand lines of evaluator, so when it faults there is no way to
//! tell a bug in the stack-switching backend from a bug in this crate's
//! teardown. These tests fault only if the backend does.
//!
//! They are ordered `t1`..`t6` by how much of the backend they need, because
//! a fault here is a *process* fault — it kills the harness, so the log shows
//! which stage was reached and nothing after it. Run single-threaded so the
//! name is flushed before the crash:
//!
//! ```text
//! cargo test --release --test coroutine_backend -- --test-threads=1 --nocapture
//! ```
//!
//! The interesting stages are `t3` onward: dropping a coroutine that is still
//! suspended cannot just free its stack, because that stack holds live locals.
//! `Coroutine::drop` instead resumes the body with a forced-unwind panic and
//! lets it unwind, running destructors on the way out. On Windows that unwind
//! cannot cross the stack-switch trampoline (`corosensei`'s own
//! `arch/x86_64_windows.rs`: *"the unwinder will not update the TEB fields
//! when switching stacks"*), so it is caught at the coroutine root and the
//! four TEB stack fields are restored by hand around every switch. That is
//! the path `generator_stress::cloned_generator_values_share_one_coroutine`
//! dies on with `STATUS_ACCESS_VIOLATION`.

#![cfg(stackful_coroutines)]

use std::cell::Cell;
use std::rc::Rc;

use corosensei::stack::DefaultStack;
use corosensei::{Coroutine, CoroutineResult, Yielder};

/// The stack size `interpreter::call::GENERATOR_STACK_SIZE` uses, so these
/// exercise the same allocation shape as a real generator.
const STACK_SIZE: usize = 8 * 1024 * 1024;

/// How many lifecycles each stage runs. A single forced unwind survives on
/// every platform; the fault in the full suite only appears in bulk, so a
/// count in this range is what makes the difference visible.
const ROUNDS: usize = 200;

fn spawn<F>(body: F) -> Coroutine<(), u32, u32>
where
    F: FnOnce(&Yielder<(), u32>, ()) -> u32 + 'static,
{
    let stack = DefaultStack::new(STACK_SIZE).expect("failed to allocate a coroutine stack");
    Coroutine::with_stack(stack, body)
}

/// Increments a counter when dropped, so a test can prove the forced unwind
/// actually ran destructors rather than silently leaking the frame.
struct DropCount(Rc<Cell<usize>>);

impl Drop for DropCount {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

// ── stages ───────────────────────────────────────────────────────────

#[test]
fn t1_coroutines_run_to_completion() {
    for _ in 0..ROUNDS {
        let mut c = spawn(|y, _| {
            y.suspend(1);
            y.suspend(2);
            7
        });
        assert_eq!(c.resume(()), CoroutineResult::Yield(1));
        assert_eq!(c.resume(()), CoroutineResult::Yield(2));
        assert_eq!(c.resume(()), CoroutineResult::Return(7));
    }
}

#[test]
fn t2_coroutines_dropped_before_starting() {
    // Never resumed: `force_unwind` only has to drop the initial closure,
    // which is a different path from unwinding a live stack.
    for _ in 0..ROUNDS {
        drop(spawn(|y, _| {
            y.suspend(1);
            0
        }));
    }
}

#[test]
fn t3_coroutines_dropped_while_suspended() {
    // The shape of `cloned_generator_values_share_one_coroutine`: suspend
    // three times, then drop without letting the body finish.
    for _ in 0..ROUNDS {
        let mut c = spawn(|y, _| {
            y.suspend(1);
            y.suspend(2);
            y.suspend(3);
            0
        });
        c.resume(());
        c.resume(());
        c.resume(());
        drop(c);
    }
}

#[test]
fn t4_dropped_coroutines_run_their_destructors() {
    // The forced unwind has to actually unwind, not just discard the stack:
    // a generator body owns `Value`s and environments whose destructors
    // release `Rc`s the driver still holds.
    let drops = Rc::new(Cell::new(0));
    for _ in 0..ROUNDS {
        let counter = drops.clone();
        let mut c = spawn(move |y, _| {
            let _guard = DropCount(counter);
            // A heap allocation the unwind has to free, not just a `Copy`
            // local the stack reclaims for free.
            let mut held = Vec::with_capacity(4);
            held.push(String::from("live across the suspension point"));
            y.suspend(1);
            0
        });
        c.resume(());
        drop(c);
    }
    assert_eq!(
        drops.get(),
        ROUNDS,
        "the forced unwind skipped destructors on the coroutine stack"
    );
}

#[test]
fn t5_coroutines_dropped_while_suspended_deep() {
    // Suspending from far down the coroutine's own stack makes the forced
    // unwind walk many frames, as it does when a generator yields from inside
    // nested evaluator calls.
    fn descend(y: &Yielder<(), u32>, depth: u32) -> u32 {
        let _guard = [depth; 16];
        if depth == 0 {
            y.suspend(1);
            return 0;
        }
        descend(y, depth - 1)
    }

    for _ in 0..ROUNDS {
        let mut c = spawn(|y, _| descend(y, 64));
        c.resume(());
        drop(c);
    }
}

#[test]
fn t7_dropped_from_deep_in_the_callers_stack() {
    // The variable stages 1-6 never move: *who is on the main stack* when the
    // drop happens. Those all drop from a shallow test frame; the captured
    // Windows fault drops from `eval_stmt -> run_block -> Environment`
    // teardown, with the driver's evaluator frames live beneath it.
    //
    // Windows unwinding consults the thread's TEB stack bounds, which the
    // switch swaps. If a deep, live main stack changes what the unwinder sees
    // while it walks the coroutine stack, it shows up here and nowhere else.
    fn drop_at_depth(depth: u32) {
        let _frame = [depth; 32];
        if depth != 0 {
            return drop_at_depth(depth - 1);
        }
        let mut c = spawn(|y, _| {
            y.suspend(1);
            0
        });
        c.resume(());
        drop(c);
    }

    for _ in 0..ROUNDS {
        drop_at_depth(64);
    }
}

#[test]
fn t8_deep_coroutine_dropped_from_deep_in_the_callers_stack() {
    // Both stacks deep at once, which is the shape a real generator has: a
    // body suspended inside nested evaluator calls, abandoned by a driver
    // that is itself nested.
    fn suspend_at_depth(y: &Yielder<(), u32>, depth: u32) -> u32 {
        let _frame = [depth; 32];
        if depth == 0 {
            y.suspend(1);
            return 0;
        }
        suspend_at_depth(y, depth - 1)
    }

    fn drop_at_depth(depth: u32) {
        let _frame = [depth; 32];
        if depth != 0 {
            return drop_at_depth(depth - 1);
        }
        let mut c = spawn(|y, _| suspend_at_depth(y, 32));
        c.resume(());
        drop(c);
    }

    for _ in 0..ROUNDS {
        drop_at_depth(32);
    }
}

#[test]
fn t6_nested_coroutines_dropped_while_both_suspended() {
    // `nested_generators_abandoned_mid_flight`: an outer coroutine is itself
    // the caller resuming an inner one, so dropping the outer force-unwinds a
    // stack that owns another suspended stack.
    for _ in 0..ROUNDS {
        let mut outer = spawn(|y, _| {
            let mut inner = spawn(|iy, _| {
                iy.suspend(10);
                iy.suspend(20);
                0
            });
            inner.resume(());
            y.suspend(1);
            // Never reached: the outer is dropped while suspended here, which
            // drops `inner` — still suspended — during the unwind.
            drop(inner);
            0
        });
        outer.resume(());
        drop(outer);
    }
}
