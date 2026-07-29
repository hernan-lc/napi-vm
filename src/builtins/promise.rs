//! `Promise` static methods: `resolve`, `reject`, `all`, and `race`.
//!
//! The VM's promise model is eager and synchronous — an async function runs its
//! body immediately and yields an already-settled promise — so these combinators
//! produce settled promises directly rather than scheduling microtasks.

use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::{PromiseState, Value};

pub(super) fn install(e: &mut Environment) {
    if let Some(p) = e.get("Promise") {
        p.set_prop("resolve".to_string(), super::nf("resolve", promise_resolve));
        p.set_prop("reject".to_string(), super::nf("reject", promise_reject));
        p.set_prop("all".to_string(), super::nf("all", promise_all));
        p.set_prop("race".to_string(), super::nf("race", promise_race));
    }
}

fn promise_resolve(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let v = a.first().cloned().unwrap_or(Value::Undefined);
    // Resolving with a promise returns that promise unchanged (flattening).
    if matches!(v, Value::Promise { .. }) {
        return Ok(v);
    }
    Ok(Value::Promise {
        state: PromiseState::Fulfilled,
        value: Some(Box::new(v)),
    })
}

fn promise_reject(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let v = a.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Promise {
        state: PromiseState::Rejected,
        value: Some(Box::new(v)),
    })
}

fn promise_all(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = match a.first() {
        Some(Value::Array(i)) => i.borrow().clone(),
        _ => vec![],
    };
    let mut out = Vec::with_capacity(items.len());
    for it in items {
        match it {
            // The first rejection short-circuits the whole combinator.
            Value::Promise {
                state: PromiseState::Rejected,
                value,
            } => {
                return Ok(Value::Promise {
                    state: PromiseState::Rejected,
                    value,
                });
            }
            Value::Promise { value, .. } => {
                out.push(value.map(|b| *b).unwrap_or(Value::Undefined));
            }
            other => out.push(other),
        }
    }
    Ok(Value::Promise {
        state: PromiseState::Fulfilled,
        value: Some(Box::new(Value::array(out))),
    })
}

fn promise_race(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = match a.first() {
        Some(Value::Array(i)) => i.borrow().clone(),
        _ => vec![],
    };
    // In the eager model the first element is already settled, so it wins.
    let first = items.into_iter().next().unwrap_or(Value::Undefined);
    Ok(match first {
        Value::Promise { .. } => first,
        other => Value::Promise {
            state: PromiseState::Fulfilled,
            value: Some(Box::new(other)),
        },
    })
}
