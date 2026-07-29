use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use crate::error::VmErr;
use crate::interpreter::{Env, Interpreter};
use crate::parser::Statement;

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
        name: Option<String>,
        // Shared (`Rc`) so closures created in hot loops reference the same AST
        // instead of deep-cloning the parameter list and body on every creation.
        params: Rc<Vec<String>>,
        body: Rc<Vec<Statement>>,
        closure: Option<Env>,
        is_arrow: bool,
        is_async: bool,
        is_generator: bool,
    },
    NativeFunction {
        name: String,
        callable: fn(&mut Interpreter, Value, Vec<Value>) -> Result<Value, VmErr>,
    },
    /// A function implemented on the host (Node.js) side, reachable from the VM.
    /// Calling it dispatches through the interpreter's `HostBridge` using `id`,
    /// which the bridge maps to a persisted JavaScript function reference.
    HostFunction {
        name: String,
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
}
