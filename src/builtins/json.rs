//! `JSON.stringify` / `JSON.parse`

use super::nf;
use crate::error::{VmErr, vm_err};
use crate::interpreter::{Environment, Interpreter};
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
    let mut out = String::new();
    json_serialize(&v, &mut out);
    Ok(Value::String(out))
}

fn json_serialize(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Undefined => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if n.is_nan() || n.is_infinite() {
                out.push_str("null");
            } else if n.fract() == 0.0 && n.abs() < 1e15 {
                use std::fmt::Write;
                let _ = write!(out, "{:.0}", n);
            } else {
                out.push_str(&n.to_string());
            }
        }
        Value::String(s) => {
            out.push('"');
            escape_json(s, out);
            out.push('"');
        }
        Value::Array(items) => {
            out.push('[');
            let items = items.borrow();
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                json_serialize(it, out);
            }
            out.push(']');
        }
        Value::Object { props, .. } => {
            out.push('{');
            let props = props.borrow();
            let mut first = true;
            for (k, v) in props.iter() {
                if matches!(v, Value::Undefined) {
                    continue;
                }
                if !first {
                    out.push(',');
                }
                first = false;
                out.push('"');
                escape_json(k, out);
                out.push_str("\":");
                json_serialize(v, out);
            }
            out.push('}');
        }
        _ => out.push_str("null"),
    }
}

fn escape_json(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
}

fn json_parse(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = match a.first() {
        Some(Value::String(s)) => s,
        _ => return vm_err("JSON.parse requires a string argument"),
    };
    JsonParser::new(s).parse()
}

/// A small recursive-descent JSON parser producing `Value`s directly, with no
/// token or AST allocation. Accepts strict JSON only (quoted keys, no trailing
/// commas), matching the semantics the previous lexer/parser-reuse approach
/// provided for well-formed JSON input.
struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }

    fn parse(mut self) -> Result<Value, VmErr> {
        let v = self.value()?;
        self.skip_ws();
        if self.pos != self.bytes.len() {
            return vm_err("Invalid JSON");
        }
        Ok(v)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), VmErr> {
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(VmErr::Msg("Invalid JSON".to_string()))
        }
    }

    fn value(&mut self) -> Result<Value, VmErr> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b't') => self.literal(b"true", Value::Bool(true)),
            Some(b'f') => self.literal(b"false", Value::Bool(false)),
            Some(b'n') => self.literal(b"null", Value::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            _ => vm_err("Invalid JSON"),
        }
    }

    fn literal(&mut self, lit: &[u8], v: Value) -> Result<Value, VmErr> {
        if self.bytes.get(self.pos..self.pos + lit.len()) == Some(lit) {
            self.pos += lit.len();
            Ok(v)
        } else {
            vm_err("Invalid JSON")
        }
    }

    fn number(&mut self) -> Result<Value, VmErr> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        // The scanned range is ASCII-only (digits and punctuation), hence a
        // valid UTF-8 slice of the original input.
        let s = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| VmErr::Msg("Invalid JSON".to_string()))?;
        s.parse::<f64>()
            .map(Value::Number)
            .map_err(|_| VmErr::Msg("Invalid JSON".to_string()))
    }

    fn string(&mut self) -> Result<String, VmErr> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err(VmErr::Msg("Invalid JSON".to_string())),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'"') => out.push('"'),
                        Some(b'\\') => out.push('\\'),
                        Some(b'/') => out.push('/'),
                        Some(b'b') => out.push('\u{08}'),
                        Some(b'f') => out.push('\u{0C}'),
                        Some(b'n') => out.push('\n'),
                        Some(b'r') => out.push('\r'),
                        Some(b't') => out.push('\t'),
                        Some(b'u') => {
                            self.pos += 1;
                            let hi = self.hex4()?;
                            // Combine a UTF-16 surrogate pair into one scalar.
                            if (0xD800..0xDC00).contains(&hi)
                                && self.peek() == Some(b'\\')
                                && self.bytes.get(self.pos + 1) == Some(&b'u')
                            {
                                self.pos += 2;
                                let lo = self.hex4()?;
                                if (0xDC00..0xE000).contains(&lo) {
                                    let cp = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                                    out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                                } else {
                                    out.push('\u{FFFD}');
                                    out.push(char::from_u32(lo).unwrap_or('\u{FFFD}'));
                                }
                            } else {
                                out.push(char::from_u32(hi).unwrap_or('\u{FFFD}'));
                            }
                            continue;
                        }
                        _ => return Err(VmErr::Msg("Invalid JSON".to_string())),
                    }
                    self.pos += 1;
                }
                Some(_) => {
                    // Fast path: copy a run of bytes with no quote/backslash.
                    // UTF-8 continuation bytes are >= 0x80 and can never be
                    // 0x22 or 0x5C, so the run ends on a char boundary.
                    let start = self.pos;
                    while matches!(self.peek(), Some(c) if c != b'"' && c != b'\\') {
                        self.pos += 1;
                    }
                    let run = std::str::from_utf8(&self.bytes[start..self.pos])
                        .map_err(|_| VmErr::Msg("Invalid JSON".to_string()))?;
                    out.push_str(run);
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, VmErr> {
        let mut v = 0u32;
        for _ in 0..4 {
            let c = self
                .peek()
                .ok_or_else(|| VmErr::Msg("Invalid JSON".to_string()))?;
            self.pos += 1;
            v = v * 16
                + match c {
                    b'0'..=b'9' => (c - b'0') as u32,
                    b'a'..=b'f' => (c - b'a' + 10) as u32,
                    b'A'..=b'F' => (c - b'A' + 10) as u32,
                    _ => return Err(VmErr::Msg("Invalid JSON".to_string())),
                };
        }
        Ok(v)
    }

    fn array(&mut self) -> Result<Value, VmErr> {
        self.pos += 1; // [
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::array(items));
                }
                _ => return vm_err("Invalid JSON"),
            }
        }
    }

    fn object(&mut self) -> Result<Value, VmErr> {
        self.pos += 1; // {
        let mut props = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::object(props));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return vm_err("Invalid JSON");
            }
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            let v = self.value()?;
            props.push((key, v));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::object(props));
                }
                _ => return vm_err("Invalid JSON"),
            }
        }
    }
}
