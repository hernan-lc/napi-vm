//! `Object` static methods.

use super::nf;
use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::Value;

pub(super) fn install(e: &mut Environment) {
    if let Some(o) = e.get("Object") {
        o.set_prop("keys".to_string(), nf("keys", object_keys));
        o.set_prop("values".to_string(), nf("values", object_values));
        o.set_prop("entries".to_string(), nf("entries", object_entries));
        o.set_prop("assign".to_string(), nf("assign", object_assign));
    }
}

fn object_keys(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let v = a.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::array(
        interp.keys(&v).into_iter().map(Value::String).collect(),
    ))
}
fn object_values(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    match a.first() {
        Some(Value::Object { props, .. }) => Ok(Value::array(props.borrow().values())),
        _ => Ok(Value::array(vec![])),
    }
}
fn object_entries(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    match a.first() {
        Some(Value::Object { props, .. }) => {
            let entries = props
                .borrow()
                .entries()
                .into_iter()
                .map(|(k, v)| Value::array(vec![Value::String(k), v]))
                .collect();
            Ok(Value::array(entries))
        }
        _ => Ok(Value::array(vec![])),
    }
}
fn object_assign(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let target = a.first().cloned().unwrap_or_else(|| Value::object(vec![]));
    for src in a.iter().skip(1) {
        if let Value::Object { props, .. } = src {
            for (k, v) in props.borrow().entries() {
                target.set_prop(k, v);
            }
        }
    }
    Ok(target)
}
