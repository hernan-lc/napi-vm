//! `Promise` static methods and eager instance chaining methods.
//!
//! The VM's promise model is eager and synchronous — an async function runs its
//! body immediately and yields an already-settled promise — so these combinators
//! produce settled promises directly rather than scheduling microtasks.

use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::{PromiseState, Value};

pub(crate) fn promise_method(name: &str) -> Option<Value> {
    let callable = match name {
        "then" => promise_then,
        "catch" => promise_catch,
        "finally" => promise_finally,
        _ => return None,
    };
    Some(Value::NativeFunction {
        name: name.into(),
        callable,
    })
}

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
        match &it {
            // The first rejection short-circuits the whole combinator.
            Value::Promise {
                state: PromiseState::Rejected,
                value,
            } => {
                return Ok(Value::Promise {
                    state: PromiseState::Rejected,
                    value: value.clone(),
                });
            }
            Value::Promise { value, .. } => {
                out.push(
                    value
                        .as_ref()
                        .map(|b| (**b).clone())
                        .unwrap_or(Value::Undefined),
                );
            }
            _ => out.push(it),
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

fn is_callable(value: &Value) -> bool {
    matches!(
        value,
        Value::Function(_) | Value::NativeFunction { .. } | Value::HostFunction { .. }
    )
}

fn fulfilled(value: Value) -> Value {
    Value::Promise {
        state: PromiseState::Fulfilled,
        value: Some(Box::new(value)),
    }
}

fn rejected(value: Value) -> Value {
    Value::Promise {
        state: PromiseState::Rejected,
        value: Some(Box::new(value)),
    }
}

fn chain_callback(
    interpreter: &mut Interpreter,
    callback: &Value,
    value: Value,
) -> Result<Value, VmErr> {
    if !is_callable(callback) {
        return Ok(fulfilled(value));
    }

    match interpreter.call_this(callback, Value::Undefined, vec![value]) {
        Ok(value @ Value::Promise { .. }) => Ok(value),
        Ok(value) => Ok(fulfilled(value)),
        Err(VmErr::Throw(value)) => Ok(rejected(value)),
        Err(error) => Err(error),
    }
}

fn promise_then(
    interpreter: &mut Interpreter,
    this: Value,
    args: Vec<Value>,
) -> Result<Value, VmErr> {
    let (state, value) = match &this {
        Value::Promise { state, value } => (
            state.clone(),
            value
                .as_ref()
                .map(|boxed| (**boxed).clone())
                .unwrap_or(Value::Undefined),
        ),
        _ => return Ok(rejected(Value::Undefined)),
    };
    let on_fulfilled = args.first().cloned().unwrap_or(Value::Undefined);
    let on_rejected = args.get(1).cloned().unwrap_or(Value::Undefined);

    match state {
        PromiseState::Fulfilled => chain_callback(interpreter, &on_fulfilled, value),
        PromiseState::Rejected => {
            if is_callable(&on_rejected) {
                chain_callback(interpreter, &on_rejected, value)
            } else {
                Ok(rejected(value))
            }
        }
        PromiseState::Pending => Ok(rejected(value)),
    }
}

fn promise_catch(
    interpreter: &mut Interpreter,
    this: Value,
    args: Vec<Value>,
) -> Result<Value, VmErr> {
    promise_then(
        interpreter,
        this,
        vec![Value::Undefined, args.first().cloned().unwrap_or(Value::Undefined)],
    )
}

fn promise_finally(
    interpreter: &mut Interpreter,
    this: Value,
    args: Vec<Value>,
) -> Result<Value, VmErr> {
    let callback = args.first().cloned().unwrap_or(Value::Undefined);
    if is_callable(&callback) {
        match interpreter.call_this(&callback, Value::Undefined, vec![]) {
            Ok(_) => {}
            Err(VmErr::Throw(value)) => return Ok(rejected(value)),
            Err(error) => return Err(error),
        }
    }
    Ok(this)
}
