//! `Math` methods. The constants (`PI`, `E`, ...) are installed as plain
//! properties by `setup_builtins`; this module supplies the callable methods.

use super::{NativeFn, arg_num, nf};
use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::Value;

pub(super) fn install(e: &mut Environment) {
    if let Some(math) = e.get("Math") {
        for (name, f) in math_methods() {
            math.set_prop(name, f);
        }
    }
}

fn math_methods() -> Vec<(String, Value)> {
    let table: Vec<(&str, NativeFn)> = vec![
        ("abs", math_abs),
        ("floor", math_floor),
        ("ceil", math_ceil),
        ("round", math_round),
        ("sqrt", math_sqrt),
        ("cbrt", math_cbrt),
        ("pow", math_pow),
        ("min", math_min),
        ("max", math_max),
        ("random", math_random),
        ("trunc", math_trunc),
        ("sign", math_sign),
        ("log", math_log),
        ("log2", math_log2),
        ("log10", math_log10),
        ("exp", math_exp),
        ("sin", math_sin),
        ("cos", math_cos),
        ("tan", math_tan),
        ("hypot", math_hypot),
    ];
    table
        .into_iter()
        .map(|(n, f)| (n.to_string(), nf(n, f)))
        .collect()
}

fn math_abs(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).abs()))
}
fn math_floor(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).floor()))
}
fn math_ceil(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).ceil()))
}
fn math_round(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let x = arg_num(&a, 0);
    // JS rounds halves toward +Infinity.
    Ok(Value::Number((x + 0.5).floor()))
}
fn math_sqrt(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).sqrt()))
}
fn math_cbrt(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).cbrt()))
}
fn math_pow(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).powf(arg_num(&a, 1))))
}
fn math_trunc(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).trunc()))
}
fn math_sign(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let x = arg_num(&a, 0);
    let r = if x.is_nan() {
        f64::NAN
    } else if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    };
    Ok(Value::Number(r))
}
fn math_log(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).ln()))
}
fn math_log2(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).log2()))
}
fn math_log10(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).log10()))
}
fn math_exp(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).exp()))
}
fn math_sin(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).sin()))
}
fn math_cos(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).cos()))
}
fn math_tan(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).tan()))
}
fn math_min(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    if a.is_empty() {
        return Ok(Value::Number(f64::INFINITY));
    }
    let mut m = f64::INFINITY;
    for v in &a {
        let n = v.to_number();
        if n.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        if n < m {
            m = n;
        }
    }
    Ok(Value::Number(m))
}
fn math_max(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    if a.is_empty() {
        return Ok(Value::Number(f64::NEG_INFINITY));
    }
    let mut m = f64::NEG_INFINITY;
    for v in &a {
        let n = v.to_number();
        if n.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        if n > m {
            m = n;
        }
    }
    Ok(Value::Number(m))
}
fn math_hypot(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let mut sum = 0.0;
    for v in &a {
        let n = v.to_number();
        sum += n * n;
    }
    Ok(Value::Number(sum.sqrt()))
}
fn math_random(_: &mut Interpreter, _: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);
    let mut x = SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    Ok(Value::Number((x >> 11) as f64 / (1u64 << 53) as f64))
}
