//! A minimal `Symbol` global: callable to mint symbol values, with the
//! well-known `Symbol.iterator` member (resolved specially in `prop()`, since a
//! native function cannot carry its own properties).

use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::Value;

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
