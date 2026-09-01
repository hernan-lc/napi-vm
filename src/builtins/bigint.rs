//! The `BigInt` global: conversion and the fixed-width wrapping helpers.

use std::rc::Rc;

use crate::bigint::BigInt;
use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::Value;

pub(super) fn install(e: &mut Environment) {
    let Some(namespace) = e.get("BigInt") else {
        return;
    };
    namespace
        .set_prop("asIntN".to_string(), super::nf("asIntN", as_int_n))
        .expect("built-in BigInt property");
    namespace
        .set_prop("asUintN".to_string(), super::nf("asUintN", as_uint_n))
        .expect("built-in BigInt property");
    super::make_callable(&namespace, bigint_convert, None);
}

/// `BigInt(value)`. A number must be an exact integer — there is no rounding,
/// since a silent one would defeat the point of the type.
fn bigint_convert(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let value = match a.first() {
        Some(Value::BigInt(existing)) => return Ok(Value::BigInt(existing.clone())),
        Some(Value::Number(n)) => BigInt::from_f64(*n),
        Some(Value::Bool(b)) => Ok(BigInt::from_i64(if *b { 1 } else { 0 })),
        Some(Value::String(s)) => BigInt::parse(s),
        None | Some(Value::Undefined) => {
            Err("TypeError: Cannot convert undefined to a BigInt".into())
        }
        Some(Value::Null) => Err("TypeError: Cannot convert null to a BigInt".to_string()),
        Some(other) => BigInt::parse(&interp.vs(other)?),
    };
    value.map(|v| Value::BigInt(Rc::new(v))).map_err(VmErr::Msg)
}

fn wrap(a: &[Value], signed: bool) -> Result<Value, VmErr> {
    let bits = a.first().map(|v| v.to_number()).unwrap_or(0.0);
    if !bits.is_finite() || bits < 0.0 {
        return Err(VmErr::Msg("RangeError: Invalid bit width".to_string()));
    }
    let Some(value) = a.get(1).and_then(|v| v.as_bigint()) else {
        return Err(VmErr::Msg(
            "TypeError: Cannot convert a non-BigInt value".to_string(),
        ));
    };
    value
        .as_n_bit(bits as usize, signed)
        .map(|v| Value::BigInt(Rc::new(v)))
        .map_err(VmErr::Msg)
}

fn as_int_n(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    wrap(&a, true)
}

fn as_uint_n(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    wrap(&a, false)
}

/// Methods readable on a `BigInt` value.
pub fn bigint_method(key: &str) -> Option<Value> {
    match key {
        "toString" | "toLocaleString" => Some(super::nf("toString", bigint_to_string)),
        "valueOf" => Some(super::nf("valueOf", bigint_value_of)),
        _ => None,
    }
}

fn bigint_to_string(_: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::String(match this.as_bigint() {
        Some(value) => value.to_decimal(),
        None => "0".to_string(),
    }))
}

fn bigint_value_of(_: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    Ok(this)
}
