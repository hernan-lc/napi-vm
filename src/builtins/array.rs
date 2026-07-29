//! `Array` statics and `Array.prototype` methods.

use super::{NativeFn, arr_items, join_str, nf};
use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::Value;

pub(super) fn install(e: &mut Environment) {
    if let Some(a) = e.get("Array") {
        a.set_prop("isArray".to_string(), nf("isArray", array_is_array));
    }
}

// --- Array statics ----------------------------------------------------------

fn array_is_array(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Bool(matches!(a.get(0), Some(Value::Array(_)))))
}

// --- Array prototype --------------------------------------------------------

/// Dispatch table for `Array.prototype` methods, looked up by `prop()`.
pub fn array_method(name: &str) -> Option<Value> {
    let f: NativeFn = match name {
        "map" => array_map,
        "filter" => array_filter,
        "reduce" => array_reduce,
        "forEach" => array_for_each,
        "find" => array_find,
        "some" => array_some,
        "every" => array_every,
        "push" => array_push,
        "pop" => array_pop,
        "join" => array_join,
        "indexOf" => array_index_of,
        "includes" => array_includes,
        "slice" => array_slice,
        "concat" => array_concat,
        "reverse" => array_reverse,
        _ => return None,
    };
    Some(nf(name, f))
}

fn array_map(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    let mut out = Vec::with_capacity(items.len());
    for (i, it) in items.iter().enumerate() {
        let r = interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
        out.push(r);
    }
    Ok(Value::array(out))
}

fn array_filter(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    let mut out = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let keep = interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
        if keep.is_truthy() {
            out.push(it.clone());
        }
    }
    Ok(Value::array(out))
}

fn array_reduce(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    let (mut acc, start) = if a.len() >= 2 {
        (a[1].clone(), 0)
    } else {
        (items.get(0).cloned().unwrap_or(Value::Undefined), 1)
    };
    for i in start..items.len() {
        acc = interp.call_this(
            &cb,
            Value::Undefined,
            vec![acc, items[i].clone(), Value::Number(i as f64), this.clone()],
        )?;
    }
    Ok(acc)
}

fn array_for_each(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    for (i, it) in items.iter().enumerate() {
        interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
    }
    Ok(Value::Undefined)
}

fn array_find(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    for (i, it) in items.iter().enumerate() {
        let hit = interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
        if hit.is_truthy() {
            return Ok(it.clone());
        }
    }
    Ok(Value::Undefined)
}

fn array_some(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    for (i, it) in items.iter().enumerate() {
        let hit = interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
        if hit.is_truthy() {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

fn array_every(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    for (i, it) in items.iter().enumerate() {
        let hit = interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
        if !hit.is_truthy() {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn array_push(_: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    if let Value::Array(items) = &this {
        let mut b = items.borrow_mut();
        for x in a {
            b.push(x);
        }
        return Ok(Value::Number(b.len() as f64));
    }
    Ok(Value::Undefined)
}

fn array_pop(_: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    if let Value::Array(items) = &this {
        return Ok(items.borrow_mut().pop().unwrap_or(Value::Undefined));
    }
    Ok(Value::Undefined)
}

fn array_join(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let sep = match a.get(0) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Undefined) | None => ",".to_string(),
        Some(v) => interp.vs(v),
    };
    let parts: Vec<String> = items.iter().map(|v| join_str(interp, v)).collect();
    Ok(Value::String(parts.join(&sep)))
}

fn array_index_of(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let target = a.get(0).cloned().unwrap_or(Value::Undefined);
    for (i, it) in items.iter().enumerate() {
        if interp.seq(it, &target) {
            return Ok(Value::Number(i as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

fn array_includes(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let target = a.get(0).cloned().unwrap_or(Value::Undefined);
    for it in &items {
        if interp.seq(it, &target) {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

fn array_slice(_: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let len = items.len() as i64;
    let norm = |v: f64| -> i64 {
        if v.is_nan() {
            return 0;
        }
        let i = v as i64;
        if i < 0 { (len + i).max(0) } else { i.min(len) }
    };
    let start = norm(a.get(0).map(|v| v.to_number()).unwrap_or(0.0));
    let end = match a.get(1) {
        Some(v) => norm(v.to_number()),
        None => len,
    };
    if start >= end {
        return Ok(Value::array(vec![]));
    }
    Ok(Value::array(items[start as usize..end as usize].to_vec()))
}

fn array_concat(_: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let mut out = arr_items(&this);
    for v in a {
        match v {
            Value::Array(items) => out.extend(items.borrow().iter().cloned()),
            other => out.push(other),
        }
    }
    Ok(Value::array(out))
}

fn array_reverse(_: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    if let Value::Array(items) = &this {
        items.borrow_mut().reverse();
        return Ok(this);
    }
    Ok(Value::Undefined)
}
