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
    Ok(Value::Bool(matches!(a.first(), Some(Value::Array(_)))))
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
        "sort" => array_sort,
        "flat" => array_flat,
        "flatMap" => array_flat_map,
        "reduceRight" => array_reduce_right,
        _ => return None,
    };
    Some(nf(name, f))
}

fn array_map(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.first().cloned().unwrap_or(Value::Undefined);
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
    let cb = a.first().cloned().unwrap_or(Value::Undefined);
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
    let cb = a.first().cloned().unwrap_or(Value::Undefined);
    let (mut acc, start) = if a.len() >= 2 {
        (a[1].clone(), 0)
    } else {
        (items.first().cloned().unwrap_or(Value::Undefined), 1)
    };
    for (i, item) in items.iter().enumerate().skip(start) {
        acc = interp.call_this(
            &cb,
            Value::Undefined,
            vec![acc, item.clone(), Value::Number(i as f64), this.clone()],
        )?;
    }
    Ok(acc)
}

fn array_for_each(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.first().cloned().unwrap_or(Value::Undefined);
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
    let cb = a.first().cloned().unwrap_or(Value::Undefined);
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
    let cb = a.first().cloned().unwrap_or(Value::Undefined);
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
    let cb = a.first().cloned().unwrap_or(Value::Undefined);
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
        if b.len().saturating_add(a.len()) > crate::value::MAX_ARRAY_LEN {
            return Err(crate::value::limit_err("Maximum array length exceeded"));
        }
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
    let sep = match a.first() {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Undefined) | None => ",".to_string(),
        Some(v) => interp.vs(v),
    };
    let parts: Vec<String> = items.iter().map(|v| join_str(interp, v)).collect();
    // Pre-check the joined size: joining a million long strings could
    // allocate far past the string cap before `join` returns.
    let total: usize = parts
        .iter()
        .map(|p| p.len())
        .sum::<usize>()
        .saturating_add(sep.len().saturating_mul(parts.len().saturating_sub(1)));
    if total > crate::value::MAX_STRING_LEN {
        return Err(crate::value::limit_err("Maximum string length exceeded"));
    }
    Ok(Value::String(parts.join(&sep)))
}

fn array_index_of(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let target = a.first().cloned().unwrap_or(Value::Undefined);
    for (i, it) in items.iter().enumerate() {
        if interp.seq(it, &target) {
            return Ok(Value::Number(i as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

fn array_includes(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let target = a.first().cloned().unwrap_or(Value::Undefined);
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
    let start = norm(a.first().map(|v| v.to_number()).unwrap_or(0.0));
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
        match &v {
            Value::Array(items) => out.extend(items.borrow().iter().cloned()),
            _ => out.push(v),
        }
        if out.len() > crate::value::MAX_ARRAY_LEN {
            return Err(crate::value::limit_err("Maximum array length exceeded"));
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

fn array_sort(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    if let Value::Array(items) = &this {
        let cmp = a.first().cloned().unwrap_or(Value::Undefined);
        if matches!(
            cmp,
            Value::Function(_) | Value::NativeFunction { .. } | Value::HostFunction { .. }
        ) {
            // Comparator callback: negative/positive/zero ordering.
            items.borrow_mut().sort_by(|x, y| {
                let n = interp
                    .call_this(&cmp, Value::Undefined, vec![x.clone(), y.clone()])
                    .map(|v| v.to_number())
                    .unwrap_or(0.0);
                n.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            // Default: lexicographic comparison of the stringified elements.
            items.borrow_mut().sort_by_key(|x| interp.vs(x));
        }
    }
    Ok(this)
}

fn array_flat(_: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let depth = a.first().map(|v| v.to_number()).unwrap_or(1.0);
    let depth = if depth.is_nan() { 0.0 } else { depth };

    fn flatten(items: &[Value], depth: f64, out: &mut Vec<Value>) -> Result<(), VmErr> {
        for it in items {
            if depth > 0.0
                && let Value::Array(inner) = it
            {
                flatten(&inner.borrow(), depth - 1.0, out)?;
            } else {
                out.push(it.clone());
            }
            if out.len() > crate::value::MAX_ARRAY_LEN {
                return Err(crate::value::limit_err("Maximum array length exceeded"));
            }
        }
        Ok(())
    }

    let mut out = Vec::new();
    flatten(&items, depth, &mut out)?;
    Ok(Value::array(out))
}

fn array_flat_map(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.first().cloned().unwrap_or(Value::Undefined);
    let mut out = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let r = interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
        match &r {
            Value::Array(inner) => out.extend(inner.borrow().iter().cloned()),
            _ => out.push(r),
        }
        if out.len() > crate::value::MAX_ARRAY_LEN {
            return Err(crate::value::limit_err("Maximum array length exceeded"));
        }
    }
    Ok(Value::array(out))
}

fn array_reduce_right(
    interp: &mut Interpreter,
    this: Value,
    a: Vec<Value>,
) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.first().cloned().unwrap_or(Value::Undefined);
    let len = items.len();
    let (mut acc, mut i) = if a.len() >= 2 {
        (a[1].clone(), len as i64 - 1)
    } else if len == 0 {
        return Ok(Value::Undefined);
    } else {
        (items[len - 1].clone(), len as i64 - 2)
    };
    while i >= 0 {
        acc = interp.call_this(
            &cb,
            Value::Undefined,
            vec![
                acc,
                items[i as usize].clone(),
                Value::Number(i as f64),
                this.clone(),
            ],
        )?;
        i -= 1;
    }
    Ok(acc)
}
