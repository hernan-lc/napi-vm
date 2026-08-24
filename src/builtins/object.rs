//! `Object` static methods.

use super::nf;
use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::Value;

pub(super) fn install(e: &mut Environment) {
    if let Some(o) = e.get("Object") {
        o.set_prop("keys".to_string(), nf("keys", object_keys))
            .expect("built-in Object property");
        o.set_prop("values".to_string(), nf("values", object_values))
            .expect("built-in Object property");
        o.set_prop("entries".to_string(), nf("entries", object_entries))
            .expect("built-in Object property");
        o.set_prop("assign".to_string(), nf("assign", object_assign))
            .expect("built-in Object property");
        o.set_prop(
            "getOwnPropertyNames".to_string(),
            nf("getOwnPropertyNames", object_get_own_property_names),
        )
        .expect("built-in Object property");
    }
}

fn object_keys(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let v = a.first().cloned().unwrap_or(Value::Undefined);
    Value::checked_array(interp.keys(&v).into_iter().map(Value::String).collect())
}
fn object_values(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    match a.first() {
        Some(Value::Object { props, .. }) => {
            Value::checked_array(props.borrow().iter().map(|(_, v)| v.clone()).collect())
        }
        _ => Value::checked_array(vec![]),
    }
}
fn object_entries(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    match a.first() {
        Some(Value::Object { props, .. }) => {
            let entries = props
                .borrow()
                .iter()
                .map(|(k, v)| Value::array(vec![Value::String(k.clone()), v.clone()]))
                .collect();
            Value::checked_array(entries)
        }
        _ => Value::checked_array(vec![]),
    }
}
fn object_assign(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let target = a.first().cloned().unwrap_or_else(|| Value::object(vec![]));
    for src in a.iter().skip(1) {
        if let Value::Object { props, .. } = src {
            // Clone before writing so Object.assign(o, o) does not hold a
            // RefCell borrow while set_prop tries to mutate the same object.
            let entries = props.borrow().clone();
            for (k, v) in entries {
                target.set_prop(k, v)?;
            }
        }
    }
    Ok(target)
}
fn object_get_own_property_names(
    interp: &mut Interpreter,
    _: Value,
    a: Vec<Value>,
) -> Result<Value, VmErr> {
    let v = a.first().cloned().unwrap_or(Value::Undefined);
    match v {
        Value::GlobalObject => {
            let names = interp.global_keys();
            Value::checked_array(names.into_iter().map(Value::String).collect())
        }
        _ => Value::checked_array(interp.keys(&v).into_iter().map(Value::String).collect()),
    }
}
