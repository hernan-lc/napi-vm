//! `Array` statics and `Array.prototype` methods.

use std::cmp::Ordering;

use super::{NativeFn, arr_items, join_str, nf};
use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::Value;

pub(super) fn install(e: &mut Environment) {
    if let Some(a) = e.get("Array") {
        a.set_prop("isArray".to_string(), nf("isArray", array_is_array))
            .expect("built-in Array property");
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
    Value::checked_array(out)
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
    Value::checked_array(out)
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
        Some(v) => interp.vs(v)?,
    };
    let mut out = String::new();
    for (index, value) in items.iter().enumerate() {
        if index > 0 {
            if out.len().saturating_add(sep.len()) > crate::value::MAX_STRING_LEN {
                return Err(crate::value::limit_err("Maximum string length exceeded"));
            }
            out.push_str(&sep);
        }
        let part = join_str(interp, value)?;
        if out.len().saturating_add(part.len()) > crate::value::MAX_STRING_LEN {
            return Err(crate::value::limit_err("Maximum string length exceeded"));
        }
        out.push_str(&part);
    }
    Value::checked_string(out)
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
    Value::checked_array(items[start as usize..end as usize].to_vec())
}

fn array_concat(_: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let mut out = arr_items(&this);
    for v in a {
        match &v {
            Value::Array(items) => {
                let items = items.borrow();
                if out.len().saturating_add(items.len()) > crate::value::MAX_ARRAY_LEN {
                    return Err(crate::value::limit_err("Maximum array length exceeded"));
                }
                out.extend(items.iter().cloned());
            }
            _ => {
                if out.len() >= crate::value::MAX_ARRAY_LEN {
                    return Err(crate::value::limit_err("Maximum array length exceeded"));
                }
                out.push(v);
            }
        }
        if out.len() > crate::value::MAX_ARRAY_LEN {
            return Err(crate::value::limit_err("Maximum array length exceeded"));
        }
    }
    Value::checked_array(out)
}

fn array_reverse(_: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    if let Value::Array(items) = &this {
        items.borrow_mut().reverse();
        return Ok(this);
    }
    Ok(Value::Undefined)
}

fn compare_sort_values(
    interp: &mut Interpreter,
    comparator: &Value,
    left: &Value,
    right: &Value,
) -> Result<Ordering, VmErr> {
    let number = interp
        .call_this(
            comparator,
            Value::Undefined,
            vec![left.clone(), right.clone()],
        )?
        .to_number();
    Ok(number.partial_cmp(&0.0).unwrap_or(Ordering::Equal))
}

fn merge_sort_values(
    interp: &mut Interpreter,
    values: &mut [Value],
    comparator: &Value,
) -> Result<(), VmErr> {
    if values.len() < 2 {
        return Ok(());
    }
    let midpoint = values.len() / 2;
    merge_sort_values(interp, &mut values[..midpoint], comparator)?;
    merge_sort_values(interp, &mut values[midpoint..], comparator)?;

    // Clone the halves before invoking guest code. The comparator can mutate
    // the original array, but it must not encounter a Rust borrow of this
    // temporary sort buffer.
    let left = values[..midpoint].to_vec();
    let right = values[midpoint..].to_vec();
    let mut merged = Vec::with_capacity(values.len());
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        let order =
            compare_sort_values(interp, comparator, &left[left_index], &right[right_index])?;
        if order == Ordering::Greater {
            merged.push(right[right_index].clone());
            right_index += 1;
        } else {
            // Taking the left value for equality preserves sort stability.
            merged.push(left[left_index].clone());
            left_index += 1;
        }
    }
    merged.extend(left[left_index..].iter().cloned());
    merged.extend(right[right_index..].iter().cloned());
    values.clone_from_slice(&merged);
    Ok(())
}

fn array_sort(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    if let Value::Array(items) = &this {
        let cmp = a.first().cloned().unwrap_or(Value::Undefined);
        // Never hold the array's RefCell borrow while invoking guest code.
        // Comparators are allowed to re-enter the array (for example by
        // calling `push`), and keeping the RefMut across the callback used to
        // turn that normal JavaScript re-entry into a Rust RefCell panic.
        let mut sorted = items.borrow().clone();
        let original_len = sorted.len();
        if matches!(
            cmp,
            Value::Function(_) | Value::NativeFunction { .. } | Value::HostFunction { .. }
        ) {
            // Comparator errors must escape sort. Converting them to zero
            // silently changes program behavior and leaves callers unable to
            // catch the original exception.
            merge_sort_values(interp, &mut sorted, &cmp)?;
        } else {
            // Default: lexicographic comparison of the stringified elements.
            let mut format_error = None;
            sorted.sort_by(|left, right| {
                if format_error.is_some() {
                    return Ordering::Equal;
                }
                match (interp.vs(left), interp.vs(right)) {
                    (Ok(left), Ok(right)) => left.cmp(&right),
                    (Err(error), _) | (_, Err(error)) => {
                        format_error = Some(error);
                        Ordering::Equal
                    }
                }
            });
            if let Some(error) = format_error {
                return Err(error);
            }
        }
        let mut current = items.borrow_mut();
        // Sorting uses the array length captured on entry. If a comparator
        // shrinks the array, writing the sorted range extends it again; if it
        // appends values, those values remain after the sorted range.
        if current.len() < original_len {
            current.resize(original_len, Value::Undefined);
        }
        for (index, value) in sorted.into_iter().enumerate() {
            current[index] = value;
        }
    }
    Ok(this)
}

fn array_flat(_: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let depth = a.first().map(|v| v.to_number()).unwrap_or(1.0);
    let depth = if depth.is_nan() { 0.0 } else { depth };

    // Use an explicit work stack. Guest code can construct deeply nested
    // arrays dynamically, so parser and call-depth limits do not protect the
    // recursive implementation that used to live here.
    enum Work {
        Value(Value, f64),
        Leave(usize),
    }

    let mut work = Vec::with_capacity(items.len());
    for item in items.into_iter().rev() {
        work.push(Work::Value(item, depth));
    }
    let mut active = std::collections::HashSet::new();
    let mut out = Vec::new();
    while let Some(entry) = work.pop() {
        match entry {
            Work::Leave(identity) => {
                active.remove(&identity);
            }
            Work::Value(value, remaining) => {
                if remaining > 0.0
                    && let Value::Array(inner) = &value
                {
                    let identity = std::rc::Rc::as_ptr(inner) as usize;
                    // A cyclic array cannot be expanded forever. Treat the
                    // back-edge as a leaf, matching the boundary's other
                    // cycle-safe representations.
                    if active.insert(identity) {
                        work.push(Work::Leave(identity));
                        let children = inner.borrow().clone();
                        for child in children.into_iter().rev() {
                            work.push(Work::Value(child, remaining - 1.0));
                        }
                        continue;
                    }
                }
                out.push(value);
                if out.len() > crate::value::MAX_ARRAY_LEN {
                    return Err(crate::value::limit_err("Maximum array length exceeded"));
                }
            }
        }
    }
    Value::checked_array(out)
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
            Value::Array(inner) => {
                let inner = inner.borrow();
                if out.len().saturating_add(inner.len()) > crate::value::MAX_ARRAY_LEN {
                    return Err(crate::value::limit_err("Maximum array length exceeded"));
                }
                out.extend(inner.iter().cloned());
            }
            _ => {
                if out.len() >= crate::value::MAX_ARRAY_LEN {
                    return Err(crate::value::limit_err("Maximum array length exceeded"));
                }
                out.push(r);
            }
        }
        if out.len() > crate::value::MAX_ARRAY_LEN {
            return Err(crate::value::limit_err("Maximum array length exceeded"));
        }
    }
    Value::checked_array(out)
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
