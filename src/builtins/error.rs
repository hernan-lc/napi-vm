//! Constructible `Error` types (`Error`, `TypeError`, `RangeError`,
//! `SyntaxError`, `ReferenceError`). Each is a real class whose instances carry
//! `name` and `message` properties, so `throw new Error("x")` can be caught and
//! inspected as an object (`e.message`, `e.name`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::Value;

const ERROR_TYPES: &[&str] = &[
    "Error",
    "TypeError",
    "RangeError",
    "SyntaxError",
    "ReferenceError",
];

pub(super) fn install(e: &mut Environment) {
    for name in ERROR_TYPES {
        e.set(name, make_error_class(name));
    }
}

fn make_error_class(name: &str) -> Value {
    let constructor = Value::NativeFunction {
        name: name.to_string(),
        callable: error_ctor,
    };
    let prototype = Value::object(vec![
        ("name".to_string(), Value::String(name.to_string())),
        ("message".to_string(), Value::String(String::new())),
    ]);
    prototype.set_prop("constructor".to_string(), constructor.clone());
    Value::Class {
        name: name.to_string(),
        constructor: Box::new(constructor),
        prototype: Rc::new(prototype),
        statics: Rc::new(RefCell::new(vec![(
            "name".to_string(),
            Value::String(name.to_string()),
        )])),
        superclass: None,
    }
}

/// Shared constructor for every error type. The concrete type name is read from
/// the instance's prototype (set per-class above), so one native function
/// serves all five.
fn error_ctor(interp: &mut Interpreter, this: Value, args: Vec<Value>) -> Result<Value, VmErr> {
    let name = match this.get_prop("name") {
        Some(Value::String(s)) => s,
        _ => "Error".to_string(),
    };
    let msg = match args.first() {
        None | Some(Value::Undefined) => String::new(),
        Some(v) => interp.vs(v),
    };
    this.set_prop("message".to_string(), Value::String(msg));
    this.set_prop("name".to_string(), Value::String(name));
    Ok(Value::Undefined)
}
