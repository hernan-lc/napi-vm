//! `Symbol` global: callable to mint unique symbol values, with well-known
//! symbols (`Symbol.iterator`, `Symbol.toStringTag`, etc.) and registry
//! methods (`Symbol.for`, `Symbol.keyFor`).

use std::cell::RefCell;
use std::collections::HashMap;

use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::Value;

// Global symbol registry for `Symbol.for` / `Symbol.keyFor`.
thread_local! {
    static SYMBOL_REGISTRY: RefCell<HashMap<String, Value>> = RefCell::new(HashMap::new());
}

pub(super) fn install(e: &mut Environment) {
    e.set("Symbol", super::nf("Symbol", symbol_call));
}

fn symbol_call(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let desc = match a.first() {
        None | Some(Value::Undefined) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => interp.vs(v),
    };
    Ok(Value::Symbol(desc))
}

/// `Symbol.for(key)`: returns the shared symbol for `key`, creating it if new.
pub(crate) fn symbol_for(
    interp: &mut Interpreter,
    _: Value,
    a: Vec<Value>,
) -> Result<Value, VmErr> {
    let key = match a.first() {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Undefined) | None => "undefined".to_string(),
        Some(v) => interp.vs(v),
    };
    let sym = SYMBOL_REGISTRY.with(|reg| {
        let mut reg = reg.borrow_mut();
        reg.entry(key.clone())
            .or_insert_with(|| Value::Symbol(key.clone()))
            .clone()
    });
    Ok(sym)
}

/// `Symbol.keyFor(sym)`: returns the registry key for a shared symbol.
pub(crate) fn symbol_key_for(
    _interp: &mut Interpreter,
    _: Value,
    a: Vec<Value>,
) -> Result<Value, VmErr> {
    let desc = match a.first() {
        Some(Value::Symbol(d)) => d.clone(),
        _ => return Ok(Value::Undefined),
    };
    SYMBOL_REGISTRY.with(|reg| {
        let reg = reg.borrow();
        for (key, val) in reg.iter() {
            if let Value::Symbol(d) = val
                && *d == desc
            {
                return Ok(Value::String(key.clone()));
            }
        }
        Ok(Value::Undefined)
    })
}
