use std::cell::RefCell;
use std::rc::Rc;

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
        proto: Option<Box<Value>>,
    },
    Array(Rc<RefCell<Vec<Value>>>),
    Function {
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Statement>,
        closure: Option<Env>,
        is_arrow: bool,
    },
    NativeFunction {
        name: String,
        callable: fn(&mut Interpreter, Value, Vec<Value>) -> Result<Value, VmErr>,
    },
    Class {
        name: String,
        constructor: Box<Value>,
        prototype: Box<Value>,
        statics: Rc<RefCell<Vec<(String, Value)>>>,
        superclass: Option<Box<Value>>,
    },
    Promise {
        state: PromiseState,
        value: Option<Box<Value>>,
    },
    Generator {
        state: GeneratorState,
        body: Vec<Statement>,
        closure: Option<Env>,
        yielded: Option<Box<Value>>,
        sent: Option<Box<Value>>,
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

#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorState {
    SuspendedStart,
    SuspendedYield,
    Completed,
}

impl Value {
    pub fn object(props: Vec<(String, Value)>) -> Self {
        Value::Object {
            props: Rc::new(RefCell::new(props)),
            proto: None,
        }
    }

    pub fn object_with_proto(props: Vec<(String, Value)>, proto: Option<Box<Value>>) -> Self {
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
                if let Ok(idx) = key.parse::<usize>() {
                    if idx < items.len() {
                        return Some(items[idx].clone());
                    }
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
        match self {
            Value::Object { props, .. } => {
                let mut props = props.borrow_mut();
                for (k, v) in props.iter_mut() {
                    if k == &key {
                        *v = val;
                        return;
                    }
                }
                props.push((key, val));
            }
            _ => {}
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
                key == "length" || key.parse::<usize>().map(|i| i < items.len()).unwrap_or(false)
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
