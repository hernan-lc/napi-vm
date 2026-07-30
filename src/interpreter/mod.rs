mod call;
mod env;
mod eval;
mod ops;
mod resolve;

pub use env::{Env, Environment, Module};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::error::{StackFrame, VmErr};
use crate::host::HostBridge;
use crate::parser::Statement;
use crate::span::Span;
use crate::value::Value;

/// Maximum number of VM call frames (guest-visible recursion depth). Each VM
/// call maps onto several native Rust frames in the tree-walker, so unbounded
/// guest recursion would overflow the *native* stack — a SIGSEGV that no
/// try/catch can intercept. Checking the depth here turns that into a
/// catchable `RangeError`, the way V8 does. 256 keeps a wide margin under the
/// native limit on both the main thread (8MB typical) and generator threads
/// (8MB, see `spawn_generator_thread`).
pub const MAX_CALL_DEPTH: usize = 256;

/// Default cap on loop iterations per top-level execution (`vm.run`,
/// `registerModule`, `callFunction`). The interpreter is synchronous and has
/// no preemption, so `while (true) {}` would otherwise freeze the host event
/// loop forever; this budget turns it into a catchable `RangeError`.
/// 100M iterations is far above any legitimate computation (the benchmark
/// workloads stay under a few million) while still stopping an empty
/// infinite loop within a couple of seconds.
pub const DEFAULT_LOOP_BUDGET: u64 = 100_000_000;

pub struct Interpreter {
    pub global: Env,
    pub modules: HashMap<String, Module>,
    /// Optional bridge for calling host (Node.js) functions from inside the VM.
    /// Attached by the N-API layer when functions are exposed via
    /// `Vm.exposeFunction`; `None` for a standalone interpreter.
    pub host: Option<Rc<dyn HostBridge>>,
    pub cur_mod: Option<String>,
    pub is_main: bool,
    /// Label applied to the loop currently being entered, if any. A loop takes
    /// this on entry so nested unlabeled loops do not consume its signals.
    active_label: Option<String>,
    /// When executing inside a generator thread, this holds the channel
    /// endpoints used to communicate yield/resume with the main thread.
    /// `None` when not inside a generator body.
    pub(crate) gen_channel: Option<GenChannel>,
    /// Call stack for error reporting. Pushed on function entry, popped on exit.
    call_stack: Vec<StackFrame>,
    /// The source code for the current module/script, used to extract
    /// source lines for error context. Stored as lines for efficient lookup.
    source_lines: Vec<String>,
    /// Configured per-execution loop-iteration cap.
    loop_budget: u64,
    /// Remaining loop iterations in the current execution. Refilled by
    /// `begin_execution()` at each NAPI entry point; decremented by
    /// `consume_loop()` on every loop iteration.
    loops_remaining: u64,
}

/// Channel endpoints available to a generator body during execution.
pub(crate) struct GenChannel {
    /// Send yielded values back to the main thread.
    pub to_main: std::sync::mpsc::Sender<crate::value::GenYield>,
    /// Receive resume signals from the main thread.
    pub from_main: std::sync::mpsc::Receiver<crate::value::GenResume>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            global: Rc::new(RefCell::new(Environment::new())),
            modules: HashMap::new(),
            host: None,
            cur_mod: None,
            is_main: false,
            active_label: None,
            gen_channel: None,
            call_stack: Vec::new(),
            source_lines: Vec::new(),
            loop_budget: DEFAULT_LOOP_BUDGET,
            loops_remaining: DEFAULT_LOOP_BUDGET,
        }
    }

    /// Create an interpreter whose global scope is a fresh *user* frame chained
    /// to a shared builtins frame. User declarations land in the small user
    /// frame, so hot-path variable lookups hit immediately instead of scanning
    /// the large builtins table; builtins still resolve via the parent chain.
    pub fn with_builtins() -> Self {
        let mut interp = Self::new();
        let builtins = Rc::new(RefCell::new(Environment::new()));
        crate::builtins::setup_builtins(&builtins);
        interp.global = Rc::new(RefCell::new(Environment::child(builtins)));
        interp
    }

    pub fn run(&mut self, stmts: &[Statement]) -> Result<Value, VmErr> {
        let mut r = Value::Undefined;
        for s in stmts {
            r = self.eval_stmt(s)?;
        }
        Ok(r)
    }

    /// Refill the loop budget. Called at each NAPI entry point (`run`,
    /// `registerModule`, `callFunction`) so every top-level execution gets a
    /// full budget. Not called from `run` itself: block bodies and loop
    /// bodies re-enter it recursively and must not refill mid-execution.
    pub fn begin_execution(&mut self) {
        self.loops_remaining = self.loop_budget;
    }

    /// Change the loop-iteration cap (exposed to Node as `setLoopLimit`).
    pub fn set_loop_budget(&mut self, n: u64) {
        self.loop_budget = n;
        self.loops_remaining = n;
    }

    /// Account one loop iteration against the budget. Every loop construct
    /// calls this per iteration, so guest code can never spin forever.
    pub(crate) fn consume_loop(&mut self) -> Result<(), VmErr> {
        if self.loops_remaining == 0 {
            return Err(VmErr::Msg(
                "RangeError: Maximum loop iterations exceeded".to_string(),
            ));
        }
        self.loops_remaining -= 1;
        Ok(())
    }

    /// Set the source code for the current script/module. Used to extract
    /// source lines for error context.
    pub fn set_source(&mut self, source: &str) {
        self.source_lines = source.lines().map(String::from).collect();
    }

    /// Get a source line by 1-based line number, if available.
    pub fn get_source_line(&self, line: usize) -> Option<&str> {
        self.source_lines.get(line - 1).map(|s| s.as_str())
    }

    /// Push a frame onto the call stack. The name is shared (`Rc<str>`), so
    /// pushing a frame for a function call is a refcount bump, not a string
    /// allocation.
    pub(crate) fn push_frame(&mut self, name: std::rc::Rc<str>, span: Span) {
        self.call_stack.push(StackFrame { name, span });
    }

    /// Pop a frame from the call stack.
    pub(crate) fn pop_frame(&mut self) {
        self.call_stack.pop();
    }

    /// Get a snapshot of the current call stack.
    pub(crate) fn get_stack(&self) -> &[StackFrame] {
        &self.call_stack
    }

    /// Attach the current call stack and last span to an error.
    pub(crate) fn enrich_error(&self, err: VmErr, span: Option<Span>) -> VmErr {
        err.with_context(span, &self.call_stack)
    }

    /// Return all global variable names (user-defined + builtins). Used by
    /// `Object.getOwnPropertyNames(window)`.
    pub fn global_keys(&self) -> Vec<String> {
        self.global.borrow().all_keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn eval(src: &str) -> Result<Value, VmErr> {
        let mut interp = Interpreter::with_builtins();
        let mut lex = Lexer::new(src);
        let toks = lex.tokenize_with_spans();
        let mut parser = Parser::new_with_spans(toks);
        let stmts = parser.parse();
        interp.run(&stmts)
    }

    fn eval_str(src: &str) -> String {
        let interp = Interpreter::new();
        match eval(src) {
            Ok(v) => interp.vs(&v),
            Err(e) => format!("ERROR: {}", e),
        }
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(eval_str("2 + 2;"), "4");
        assert_eq!(eval_str("10 - 3;"), "7");
        assert_eq!(eval_str("4 * 5;"), "20");
        assert_eq!(eval_str("15 / 3;"), "5");
        assert_eq!(eval_str("10 % 3;"), "1");
    }

    #[test]
    fn test_variables() {
        assert_eq!(eval_str("const x = 42; x;"), "42");
        assert_eq!(eval_str("let x = 1; x = 2; x;"), "2");
    }

    #[test]
    fn test_functions() {
        assert_eq!(
            eval_str("function add(a, b) { return a + b; } add(3, 4);"),
            "7"
        );
        assert_eq!(eval_str("const f = (x) => x * x; f(5);"), "25");
    }

    #[test]
    fn test_closures() {
        assert_eq!(
            eval_str(
                "function counter() { let n = 0; return () => ++n; } const c = counter(); c(); c(); c();"
            ),
            "3"
        );
    }

    #[test]
    fn test_recursion() {
        assert_eq!(
            eval_str("function fib(n) { return n <= 1 ? n : fib(n-1) + fib(n-2); } fib(10);"),
            "55"
        );
    }

    #[test]
    fn test_strings() {
        assert_eq!(eval_str("'hello' + ' ' + 'world';"), "hello world");
        assert_eq!(eval_str("'hello'.length;"), "5");
    }

    #[test]
    fn test_arrays() {
        assert_eq!(eval_str("const a = [1,2,3]; a.length;"), "3");
        assert_eq!(eval_str("const a = [10,20,30]; a[1];"), "20");
    }

    #[test]
    fn test_objects() {
        assert_eq!(eval_str("const o = {x: 1}; o.x;"), "1");
        assert_eq!(eval_str("const o = {x: 1}; o['x'];"), "1");
    }

    #[test]
    fn test_loops() {
        assert_eq!(
            eval_str("let s = 0; for (let i = 0; i < 10; i++) { s += i; } s;"),
            "45"
        );
        assert_eq!(eval_str("let i = 0; while (i < 5) { i++; } i;"), "5");
    }

    #[test]
    fn test_try_catch() {
        assert_eq!(
            eval_str("try { throw 'oops'; } catch(e) { 'caught: ' + e; }"),
            "caught: oops"
        );
    }

    #[test]
    fn test_typeof() {
        assert_eq!(eval_str("typeof 42;"), "number");
        assert_eq!(eval_str("typeof 'hi';"), "string");
        assert_eq!(eval_str("typeof true;"), "boolean");
        assert_eq!(eval_str("typeof undefined;"), "undefined");
        assert_eq!(eval_str("typeof null;"), "object");
    }

    #[test]
    fn test_comparison() {
        assert_eq!(eval_str("5 === 5;"), "true");
        assert_eq!(eval_str("5 !== 3;"), "true");
        assert_eq!(eval_str("5 == 5;"), "true");
        assert_eq!(eval_str("'5' === 5;"), "false");
    }

    #[test]
    fn test_logical() {
        assert_eq!(eval_str("true && false;"), "false");
        assert_eq!(eval_str("true || false;"), "true");
        assert_eq!(eval_str("!true;"), "false");
    }

    #[test]
    fn test_ternary() {
        assert_eq!(eval_str("true ? 'yes' : 'no';"), "yes");
        assert_eq!(eval_str("false ? 'yes' : 'no';"), "no");
    }

    #[test]
    fn test_increment() {
        assert_eq!(eval_str("let i = 0; i++;"), "0");
        assert_eq!(eval_str("let i = 0; ++i;"), "1");
    }

    #[test]
    fn test_compound_assign() {
        assert_eq!(eval_str("let x = 5; x += 3; x;"), "8");
        assert_eq!(eval_str("let x = 10; x -= 4; x;"), "6");
        assert_eq!(eval_str("let x = 3; x *= 2; x;"), "6");
    }

    #[test]
    fn test_for_of() {
        assert_eq!(
            eval_str("let s = 0; for (const x of [1,2,3]) { s += x; } s;"),
            "6"
        );
    }

    #[test]
    fn test_for_in() {
        assert_eq!(
            eval_str("let r = ''; for (const k in {a: 1, b: 2}) { r += k; } r;"),
            "ab"
        );
    }

    #[test]
    fn test_switch() {
        assert_eq!(
            eval_str(
                "let r = ''; switch (2) { case 1: r = 'one'; break; case 2: r = 'two'; break; default: r = 'other'; } r;"
            ),
            "two"
        );
    }

    #[test]
    fn test_nested_functions() {
        assert_eq!(
            eval_str(
                "function outer() { function inner() { return 42; } return inner(); } outer();"
            ),
            "42"
        );
    }

    #[test]
    fn test_math_constants() {
        assert_eq!(eval_str("Math.PI;"), "3.141592653589793");
        assert_eq!(eval_str("Math.E;"), "2.718281828459045");
    }

    #[test]
    fn test_do_while() {
        assert_eq!(eval_str("let i = 0; do { i++; } while (i < 5); i;"), "5");
    }

    #[test]
    fn test_break_in_loops() {
        assert_eq!(
            eval_str("let i = 0; while (true) { if (i >= 3) { break; } i++; } i;"),
            "3"
        );
        assert_eq!(
            eval_str("let n = 0; for (let i = 0; i < 10; i++) { if (i === 4) { break; } n++; } n;"),
            "4"
        );
        assert_eq!(
            eval_str("let i = 0; do { if (i >= 2) { break; } i++; } while (true); i;"),
            "2"
        );
    }

    #[test]
    fn test_continue_in_loops() {
        assert_eq!(
            eval_str(
                "let s = 0; for (let i = 0; i < 5; i++) { if (i % 2) { continue; } s += i; } s;"
            ),
            "6"
        );
        assert_eq!(
            eval_str(
                "let s = 0; let i = 0; while (i < 5) { i++; if (i === 3) { continue; } s += i; } s;"
            ),
            "12"
        );
    }
}
