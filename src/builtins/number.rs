//! `Number` statics, `Number.prototype` methods, and the global
//! `parseInt` / `parseFloat` implementations (shared with the `Number` statics).

use super::{NativeFn, nf};
use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::Value;

pub(super) fn install(e: &mut Environment) {
    if let Some(n) = e.get("Number") {
        n.set_prop("isNaN".to_string(), nf("isNaN", number_is_nan))
            .expect("built-in Number property");
        n.set_prop("isFinite".to_string(), nf("isFinite", number_is_finite))
            .expect("built-in Number property");
        n.set_prop("parseInt".to_string(), nf("parseInt", parse_int))
            .expect("built-in Number property");
        n.set_prop("parseFloat".to_string(), nf("parseFloat", parse_float))
            .expect("built-in Number property");
    }
}

// --- Number prototype -------------------------------------------------------

pub fn number_method(name: &str) -> Option<Value> {
    let f: NativeFn = match name {
        "toFixed" => number_to_fixed,
        _ => return None,
    };
    Some(nf(name, f))
}

fn number_to_fixed(_: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let n = this.to_number();
    let digits = a.first().map(|v| v.to_number() as usize).unwrap_or(0);
    if digits > crate::value::MAX_STRING_LEN {
        return Err(crate::value::limit_err("Maximum string length exceeded"));
    }
    Value::checked_string(format!("{:.*}", digits, n))
}

// --- Number statics ---------------------------------------------------------

fn number_is_nan(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Bool(
        matches!(a.first(), Some(Value::Number(n)) if n.is_nan()),
    ))
}
fn number_is_finite(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Bool(
        matches!(a.first(), Some(Value::Number(n)) if n.is_finite()),
    ))
}

// --- parseInt / parseFloat (global and Number.* share these) ----------------

pub(super) fn parse_int(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = match a.first() {
        Some(Value::String(s)) => s.clone(),
        Some(v) => interp.vs(v),
        None => return Ok(Value::Number(f64::NAN)),
    };
    let mut radix = match a.get(1) {
        Some(v) => v.to_number() as u32,
        None => 0,
    };
    let t = s.trim();
    let mut chars = t.chars();
    let mut first = chars.next();
    let mut neg = false;
    if first == Some('+') {
        first = chars.next();
    } else if first == Some('-') {
        neg = true;
        first = chars.next();
    }
    // Infer radix from a 0x/0X prefix when unspecified.
    if radix == 0 {
        if first == Some('0') {
            let mut peek = chars.clone();
            if matches!(peek.next(), Some('x') | Some('X')) {
                radix = 16;
                chars.next();
                first = chars.next();
            } else {
                radix = 10;
            }
        } else {
            radix = 10;
        }
    }
    let mut val: i64 = 0;
    let mut any = false;
    let mut cur = first;
    while let Some(c) = cur {
        match c.to_digit(radix) {
            Some(d) => {
                val = val.saturating_mul(radix as i64).saturating_add(d as i64);
                any = true;
                cur = chars.next();
            }
            None => break,
        }
    }
    if !any {
        return Ok(Value::Number(f64::NAN));
    }
    Ok(Value::Number((if neg { -val } else { val }) as f64))
}

pub(super) fn parse_float(
    interp: &mut Interpreter,
    _: Value,
    a: Vec<Value>,
) -> Result<Value, VmErr> {
    let s = match a.first() {
        Some(Value::String(s)) => s.clone(),
        Some(v) => interp.vs(v),
        None => return Ok(Value::Number(f64::NAN)),
    };
    let t = s.trim();
    let mut end = 0usize;
    let mut seen_digit = false;
    for (i, c) in t.char_indices() {
        let ok = c.is_ascii_digit()
            || (c == '.' && seen_digit)
            || ((c == '+' || c == '-') && i == 0)
            || ((c == 'e' || c == 'E') && seen_digit);
        if ok {
            if c.is_ascii_digit() {
                seen_digit = true;
            }
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    match t[..end].parse::<f64>() {
        Ok(n) => Ok(Value::Number(n)),
        Err(_) => Ok(Value::Number(f64::NAN)),
    }
}
