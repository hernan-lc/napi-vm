//! `JSON.stringify` / `JSON.parse`.

use super::nf;
use crate::error::{VmErr, vm_err};
use crate::interpreter::{Environment, Interpreter};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::Value;

pub(super) fn install(e: &mut Environment) {
    if let Some(j) = e.get("JSON") {
        j.set_prop("stringify".to_string(), nf("stringify", json_stringify));
        j.set_prop("parse".to_string(), nf("parse", json_parse));
    }
}

fn json_stringify(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let v = a.first().cloned().unwrap_or(Value::Undefined);
    if matches!(v, Value::Undefined) {
        return Ok(Value::Undefined);
    }
    Ok(Value::String(json_serialize(&v)))
}

fn json_serialize(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Undefined => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if n.is_nan() || n.is_infinite() {
                "null".to_string()
            } else if n.fract() == 0.0 && n.abs() < 1e15 {
                format!("{:.0}", n)
            } else {
                n.to_string()
            }
        }
        Value::String(s) => format!("\"{}\"", escape_json(s)),
        Value::Array(items) => {
            let parts: Vec<String> = items.borrow().iter().map(json_serialize).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object { props, .. } => {
            let parts: Vec<String> = props
                .borrow()
                .entries()
                .into_iter()
                .filter(|(_, v)| !matches!(v, Value::Undefined))
                .map(|(k, v)| format!("\"{}\":{}", escape_json(&k), json_serialize(&v)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        _ => "null".to_string(),
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn json_parse(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = match a.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return vm_err("JSON.parse requires a string argument"),
    };
    let mut lex = Lexer::new(&s);
    let toks = lex.tokenize();
    let mut parser = Parser::new(toks);
    let expr = parser
        .expr()
        .ok_or_else(|| VmErr::Msg("Invalid JSON".to_string()))?;
    interp.eval_expr(&expr)
}
