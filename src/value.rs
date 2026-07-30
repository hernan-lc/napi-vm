use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use crate::error::VmErr;
use crate::interpreter::{Env, Interpreter};
use crate::parser::Statement;

/// Hard cap on array length. Guest code that grows an array past this gets a
/// catchable `RangeError` instead of exhausting host memory (which would
/// abort the process — Rust's allocator does not return errors, it dies).
/// `Value` is a fat enum (~120 bytes), and arrays of arrays multiply that:
/// 262k slots of 8-element inner arrays is already ~290MB, so the cap is
/// sized to keep worst-case guest allocations survivable for the host.
pub const MAX_ARRAY_LEN: usize = 262_144;

/// Hard cap (bytes) on any string the VM produces — concatenation, `repeat`,
/// `join`, `replaceAll`, `JSON.stringify`. Same rationale as `MAX_ARRAY_LEN`.
pub const MAX_STRING_LEN: usize = 16 * 1024 * 1024;

/// Convenience constructor for the guest-visible limit errors.
pub fn limit_err(msg: &str) -> VmErr {
    VmErr::Msg(format!("RangeError: {}", msg))
}

#[derive(Debug, Clone)]
pub enum Value {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Object {
        props: Rc<RefCell<Vec<(String, Value)>>>,
        proto: Option<Rc<Value>>,
    },
    Array(Rc<RefCell<Vec<Value>>>),
    Function {
        name: Option<Rc<str>>,
        // Shared (`Rc`) so closures created in hot loops reference the same AST
        // instead of deep-cloning the parameter list and body on every creation.
        params: Rc<Vec<String>>,
        body: Rc<Vec<Statement>>,
        closure: Option<Env>,
        is_arrow: bool,
        is_async: bool,
        is_generator: bool,
        /// Whether the body references `arguments`. Frames for functions that
        /// never read it skip building the (detached) arguments object.
        uses_arguments: bool,
    },
    NativeFunction {
        name: Rc<str>,
        callable: fn(&mut Interpreter, Value, Vec<Value>) -> Result<Value, VmErr>,
    },
    /// A function implemented on the host (Node.js) side, reachable from the VM.
    /// Calling it dispatches through the interpreter's `HostBridge` using `id`,
    /// which the bridge maps to a persisted JavaScript function reference.
    HostFunction {
        name: Rc<str>,
        id: usize,
    },
    /// A handle to the global scope itself. Bound to `globalThis`, `self` and
    /// `window`; member access on it reads and writes real globals (handled in
    /// `Interpreter::prop` / `assign_member`, which have scope access).
    GlobalObject,
    Class {
        name: String,
        constructor: Box<Value>,
        // Shared so every instance references the same prototype object (cheap
        // `Rc` clone, and identity-comparable for `instanceof`).
        prototype: Rc<Value>,
        statics: Rc<RefCell<Vec<(String, Value)>>>,
        superclass: Option<Box<Value>>,
    },
    Promise {
        state: PromiseState,
        value: Option<Box<Value>>,
    },
    Generator {
        inner: Rc<RefCell<GeneratorInner>>,
    },
    Symbol(String),
    Error {
        message: String,
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromiseState {
    Pending,
    Fulfilled,
    Rejected,
}

/// Messages sent from the main thread to the generator thread.
pub enum GenResume {
    /// Resume execution, optionally passing a value into the `yield` expression.
    Next(Option<Value>),
}

/// Messages sent from the generator thread back to the main thread.
pub enum GenYield {
    /// The generator yielded a value and is now suspended.
    Yielded(Value),
    /// The generator returned (body finished). Carries the return value.
    Returned(Value),
    /// The generator threw an uncaught error.
    Threw(String),
}

// Safety: the channel protocol guarantees mutual exclusion between the generator
// thread and the main thread — only one is ever active at a time. Values sent
// across the channel are not concurrently accessed.
unsafe impl Send for GenResume {}
unsafe impl Send for GenYield {}

/// Mutable state shared across a generator's `next()` calls (behind an `Rc` so
/// clones of the `Value::Generator` observe the same progress).
///
/// True mid-body suspension is implemented via a dedicated OS thread: the
/// generator body runs in its own thread and blocks at each `yield`, waiting
/// for a resume signal over a channel. This correctly handles infinite
/// generators, yields inside loops/conditionals, and `try/finally` around
/// yields.
pub struct GeneratorInner {
    pub body: Rc<Vec<Statement>>,
    pub closure: Option<Env>,
    pub params: Rc<Vec<String>>,
    pub args: Vec<Value>,
    /// Sender to the generator thread (resume signals). `None` once done.
    pub to_gen: Option<mpsc::Sender<GenResume>>,
    /// Receiver from the generator thread (yielded/returned values).
    pub from_gen: Option<mpsc::Receiver<GenYield>>,
    pub started: bool,
    pub done: bool,
    pub return_value: Option<Value>,
}

impl std::fmt::Debug for GeneratorInner {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "GeneratorInner {{ started: {}, done: {} }}",
            self.started, self.done
        )
    }
}

/// A wrapper asserting that a `Value` can be sent across threads.
///
/// # Safety
/// The generator thread and the main thread never access shared `Rc<RefCell<_>>`
/// state concurrently: the channel protocol guarantees mutual exclusion (the
/// main thread blocks on `recv()` while the generator runs, and vice versa).
pub struct SendValue(pub Value);
unsafe impl Send for SendValue {}

/// A wrapper asserting that the generator's initial state can be sent to its
/// thread. Same safety argument as `SendValue`.
pub struct SendGenInit {
    pub body: Rc<Vec<Statement>>,
    pub closure: Option<Env>,
    pub params: Rc<Vec<String>>,
    pub args: Vec<Value>,
    pub to_gen_rx: mpsc::Receiver<GenResume>,
    pub from_gen_tx: mpsc::Sender<GenYield>,
    pub builtins_env: Option<Env>,
}
unsafe impl Send for SendGenInit {}

impl Value {
    pub fn object(props: Vec<(String, Value)>) -> Self {
        Value::Object {
            props: Rc::new(RefCell::new(props)),
            proto: None,
        }
    }

    pub fn object_with_proto(props: Vec<(String, Value)>, proto: Option<Rc<Value>>) -> Self {
        Value::Object {
            props: Rc::new(RefCell::new(props)),
            proto,
        }
    }

    pub fn array(items: Vec<Value>) -> Self {
        Value::Array(Rc::new(RefCell::new(items)))
    }

    pub fn get_prop(&self, key: &str) -> Option<Value> {
        match self {
            Value::Object { props, proto } => {
                let props = props.borrow();
                for (k, v) in props.iter() {
                    if k == key {
                        return Some(v.clone());
                    }
                }
                if let Some(p) = proto {
                    p.get_prop(key)
                } else {
                    None
                }
            }
            Value::Array(items) => {
                let items = items.borrow();
                if key == "length" {
                    return Some(Value::Number(items.len() as f64));
                }
                if let Ok(idx) = key.parse::<usize>()
                    && idx < items.len()
                {
                    return Some(items[idx].clone());
                }
                None
            }
            Value::String(s) => {
                if key == "length" {
                    return Some(Value::Number(s.chars().count() as f64));
                }
                if let Ok(idx) = key.parse::<usize>() {
                    return s.chars().nth(idx).map(|c| Value::String(c.to_string()));
                }
                None
            }
            Value::Error { message, name } => match key {
                "message" => Some(Value::String(message.clone())),
                "name" => Some(Value::String(name.clone())),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn set_prop(&self, key: String, val: Value) {
        if let Value::Object { props, .. } = self {
            let mut props = props.borrow_mut();
            for (k, v) in props.iter_mut() {
                if k == &key {
                    *v = val;
                    return;
                }
            }
            props.push((key, val));
        }
    }

    pub fn has_prop(&self, key: &str) -> bool {
        match self {
            Value::Object { props, proto } => {
                let props = props.borrow();
                props.iter().any(|(k, _)| k == key)
                    || proto.as_ref().map(|p| p.has_prop(key)).unwrap_or(false)
            }
            Value::Array(items) => {
                let items = items.borrow();
                key == "length"
                    || key
                        .parse::<usize>()
                        .map(|i| i < items.len())
                        .unwrap_or(false)
            }
            Value::String(_) => key == "length",
            Value::Error { .. } => key == "message" || key == "name",
            _ => false,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            Value::Null | Value::Undefined => false,
            _ => true,
        }
    }

    pub fn to_number(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            Value::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Value::String(s) => s.parse().unwrap_or(0.0),
            Value::Null => 0.0,
            Value::Undefined => f64::NAN,
            _ => 0.0,
        }
    }

    /// Move the direct child `Value`s out of `self` into `work`, leaving
    /// shallow placeholders behind. See the `Drop` impl for why.
    fn take_children(&mut self, work: &mut Vec<Value>) {
        match self {
            Value::Array(items) => {
                // Only drain when we own the buffer outright; a shared Rc
                // keeps its contents until the last reference drops.
                if Rc::strong_count(items) == 1
                    && let Some(cell) = Rc::get_mut(items)
                {
                    work.append(cell.get_mut());
                }
            }
            Value::Object { props, proto } => {
                if Rc::strong_count(props) == 1
                    && let Some(cell) = Rc::get_mut(props)
                {
                    work.extend(cell.get_mut().drain(..).map(|(_, v)| v));
                }
                if let Some(p) = proto.take()
                    && let Ok(inner) = Rc::try_unwrap(p)
                {
                    work.push(inner);
                }
            }
            Value::Function { closure, .. } => {
                if let Some(env) = closure.take() {
                    crate::interpreter::Environment::drain_chain(env, work);
                }
            }
            Value::Class {
                constructor,
                prototype,
                statics,
                superclass,
                ..
            } => {
                work.push(std::mem::replace(constructor.as_mut(), Value::Undefined));
                if let Some(s) = superclass.take() {
                    work.push(*s);
                }
                if Rc::strong_count(prototype) == 1
                    && let Some(p) = Rc::get_mut(prototype)
                {
                    p.take_children(work);
                }
                if Rc::strong_count(statics) == 1
                    && let Some(cell) = Rc::get_mut(statics)
                {
                    work.extend(cell.get_mut().drain(..).map(|(_, v)| v));
                }
            }
            Value::Promise { value, .. } => {
                if let Some(v) = value.take() {
                    work.push(*v);
                }
            }
            Value::Generator { inner } => {
                if Rc::strong_count(inner) == 1
                    && let Some(cell) = Rc::get_mut(inner)
                {
                    let g = cell.get_mut();
                    work.append(&mut g.args);
                    if let Some(v) = g.return_value.take() {
                        work.push(v);
                    }
                    if let Some(env) = g.closure.take() {
                        crate::interpreter::Environment::drain_chain(env, work);
                    }
                }
            }
            // All remaining variants hold no heap-nested `Value`s.
            _ => {}
        }
    }
}

/// Dropping a deeply nested value (guest code can build `a = [a]` a million
/// times in a loop) would recurse once per nesting level in the derived
/// `Drop` glue and overflow the native stack *during teardown* — after the
/// guest code already finished successfully. This iterative implementation
/// drains children onto an explicit work stack instead, so teardown depth is
/// always O(1) regardless of structure depth. Cyclic `Rc` graphs (which Rust
/// would simply leak) are skipped via the strong-count checks, so they can
/// neither recurse infinitely nor crash.
impl Drop for Value {
    fn drop(&mut self) {
        let mut work: Vec<Value> = Vec::new();
        self.take_children(&mut work);
        while let Some(mut v) = work.pop() {
            v.take_children(&mut work);
            // `v` drops here with its children already removed: a shallow,
            // non-recursive drop.
        }
    }
}
