//! `String.prototype` methods.

use super::{nf, str_this, NativeFn};
use crate::error::VmErr;
use crate::interpreter::Interpreter;
use crate::value::Value;

/// Dispatch table for `String.prototype` methods, looked up by `prop()`.
pub fn string_method(name: &str) -> Option<Value> {
    let f: NativeFn = match name {
        "toUpperCase" => string_to_upper,
        "toLowerCase" => string_to_lower,
        "trim" => string_trim,
        "slice" => string_slice,
        "substring" => string_slice,
        "split" => string_split,
        "includes" => string_includes,
        "indexOf" => string_index_of,
        "charAt" => string_char_at,
        "startsWith" => string_starts_with,
        "endsWith" => string_ends_with,
        "repeat" => string_repeat,
        "replace" => string_replace,
        _ => return None,
    };
    Some(nf(name, f))
}

fn string_to_upper(interp: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::String(str_this(interp, &this).to_uppercase()))
}
fn string_to_lower(interp: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::String(str_this(interp, &this).to_lowercase()))
}
fn string_trim(interp: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::String(str_this(interp, &this).trim().to_string()))
}
fn string_slice(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len() as i64;
    let norm = |v: f64| -> i64 {
        if v.is_nan() {
            return 0;
        }
        let i = v as i64;
        if i < 0 {
            (len + i).max(0)
        } else {
            i.min(len)
        }
    };
    let start = norm(a.get(0).map(|v| v.to_number()).unwrap_or(0.0));
    let end = match a.get(1) {
        Some(v) => norm(v.to_number()),
        None => len,
    };
    if start >= end {
        return Ok(Value::String(String::new()));
    }
    Ok(Value::String(chars[start as usize..end as usize].iter().collect()))
}
fn string_split(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    match a.get(0) {
        Some(Value::String(sep)) => {
            let mut parts: Vec<Value> = s
                .split(sep.as_str())
                .map(|p| Value::String(p.to_string()))
                .collect();
            if let Some(l) = a.get(1) {
                parts.truncate(l.to_number().max(0.0) as usize);
            }
            Ok(Value::array(parts))
        }
        _ => Ok(Value::array(vec![Value::String(s)])),
    }
}
fn string_includes(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let needle = match a.get(0) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => String::new(),
    };
    Ok(Value::Bool(s.contains(&needle)))
}
fn string_index_of(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let needle = match a.get(0) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => String::new(),
    };
    Ok(Value::Number(s.find(&needle).map(|i| i as f64).unwrap_or(-1.0)))
}
fn string_char_at(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let idx = a.get(0).map(|v| v.to_number() as usize).unwrap_or(0);
    Ok(Value::String(
        s.chars().nth(idx).map(|c| c.to_string()).unwrap_or_default(),
    ))
}
fn string_starts_with(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let needle = match a.get(0) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => String::new(),
    };
    Ok(Value::Bool(s.starts_with(&needle)))
}
fn string_ends_with(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let needle = match a.get(0) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => String::new(),
    };
    Ok(Value::Bool(s.ends_with(&needle)))
}
fn string_repeat(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let n = a.get(0).map(|v| v.to_number() as usize).unwrap_or(0);
    Ok(Value::String(s.repeat(n)))
}
fn string_replace(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let from = match a.get(0) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => return Ok(Value::String(s)),
    };
    let to = match a.get(1) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => String::new(),
    };
    Ok(Value::String(s.replacen(&from, &to, 1)))
}
