use std::rc::Rc;

use super::Interpreter;
use crate::error::VmErr;
use crate::parser::{BinOp, UnOp};
use crate::value::Value;

impl Interpreter {
    pub fn bin_op(&self, op: BinOp, l: &Value, r: &Value) -> Result<Value, VmErr> {
        // Fast path: when both operands are already numbers, the arithmetic and
        // comparison operators need no coercion and `+` cannot be string
        // concatenation. This skips the `to_number` dispatch and the string
        // check on the hottest interpreter path.
        if let (Value::Number(a), Value::Number(b)) = (l, r) {
            let fast = match op {
                BinOp::Add => Some(Value::Number(a + b)),
                BinOp::Sub => Some(Value::Number(a - b)),
                BinOp::Mul => Some(Value::Number(a * b)),
                BinOp::Div => Some(Value::Number(a / b)),
                BinOp::Mod => Some(Value::Number(a % b)),
                BinOp::Pow => Some(Value::Number(a.powf(*b))),
                BinOp::BitAnd => Some(Value::Number(((*a as i32) & (*b as i32)) as f64)),
                BinOp::BitOr => Some(Value::Number(((*a as i32) | (*b as i32)) as f64)),
                BinOp::BitXor => Some(Value::Number(((*a as i32) ^ (*b as i32)) as f64)),
                BinOp::Shl => Some(Value::Number(((*a as i32) << ((*b as i32) & 31)) as f64)),
                BinOp::Shr => Some(Value::Number(((*a as i32) >> ((*b as i32) & 31)) as f64)),
                BinOp::UShr => Some(Value::Number(
                    ((*a as i32 as u32) >> (*b as i32 as u32 & 31)) as f64,
                )),
                BinOp::Lt => Some(Value::Bool(a < b)),
                BinOp::Gt => Some(Value::Bool(a > b)),
                BinOp::Le => Some(Value::Bool(a <= b)),
                BinOp::Ge => Some(Value::Bool(a >= b)),
                BinOp::Eq | BinOp::Seq => Some(Value::Bool(a == b)),
                BinOp::Neq | BinOp::Sneq => Some(Value::Bool(a != b)),
                _ => None,
            };
            if let Some(v) = fast {
                return Ok(v);
            }
        }
        Ok(match op {
            BinOp::Add => {
                // String concatenation if either side is a string; otherwise
                // numeric addition (booleans/null/etc. coerce via to_number).
                if matches!(l, Value::String(_)) || matches!(r, Value::String(_)) {
                    Value::String(format!("{}{}", self.vs(l), self.vs(r)))
                } else {
                    Value::Number(self.tn(l) + self.tn(r))
                }
            }
            BinOp::Sub => Value::Number(self.tn(l) - self.tn(r)),
            BinOp::Mul => Value::Number(self.tn(l) * self.tn(r)),
            BinOp::Div => Value::Number(self.tn(l) / self.tn(r)),
            BinOp::Mod => Value::Number(self.tn(l) % self.tn(r)),
            BinOp::Pow => Value::Number(self.tn(l).powf(self.tn(r))),
            BinOp::BitAnd => Value::Number(((self.tn(l) as i32) & (self.tn(r) as i32)) as f64),
            BinOp::BitOr => Value::Number(((self.tn(l) as i32) | (self.tn(r) as i32)) as f64),
            BinOp::BitXor => Value::Number(((self.tn(l) as i32) ^ (self.tn(r) as i32)) as f64),
            BinOp::Shl => Value::Number(((self.tn(l) as i32) << ((self.tn(r) as i32) & 31)) as f64),
            BinOp::Shr => Value::Number(((self.tn(l) as i32) >> ((self.tn(r) as i32) & 31)) as f64),
            BinOp::UShr => {
                let a = (self.tn(l) as i32) as u32;
                let b = (self.tn(r) as i32) as u32 & 31;
                Value::Number((a >> b) as f64)
            }
            BinOp::Eq => Value::Bool(self.leq(l, r)),
            BinOp::Neq => Value::Bool(!self.leq(l, r)),
            BinOp::Seq => Value::Bool(self.seq(l, r)),
            BinOp::Sneq => Value::Bool(!self.seq(l, r)),
            BinOp::Lt => Value::Bool(self.tn(l) < self.tn(r)),
            BinOp::Gt => Value::Bool(self.tn(l) > self.tn(r)),
            BinOp::Le => Value::Bool(self.tn(l) <= self.tn(r)),
            BinOp::Ge => Value::Bool(self.tn(l) >= self.tn(r)),
            BinOp::And => {
                if self.truthy(l) {
                    r.clone()
                } else {
                    l.clone()
                }
            }
            BinOp::Or => {
                if self.truthy(l) {
                    l.clone()
                } else {
                    r.clone()
                }
            }
            BinOp::Nullish => {
                if matches!(l, Value::Null | Value::Undefined) {
                    r.clone()
                } else {
                    l.clone()
                }
            }
            BinOp::Comma => r.clone(),
            BinOp::Instanceof => {
                // `l instanceof r`: walk l's prototype chain looking for r's
                // prototype object (compared by shared Rc identity).
                let target_proto = match r {
                    Value::Class { prototype, .. } => match prototype.as_ref() {
                        Value::Object { props, .. } => Some(props.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                let mut result = false;
                if let Some(tp) = target_proto {
                    let mut cur = match l {
                        Value::Object { proto, .. } => proto.clone(),
                        _ => None,
                    };
                    while let Some(p) = cur {
                        if let Value::Object { props, proto } = p.as_ref() {
                            if Rc::ptr_eq(props, &tp) {
                                result = true;
                                break;
                            }
                            cur = proto.clone();
                        } else {
                            break;
                        }
                    }
                }
                Value::Bool(result)
            }
            BinOp::In => {
                if let (Value::String(k), Value::Object { props, .. }) = (l, r) {
                    Value::Bool(props.borrow().iter().any(|(x, _)| x == k))
                } else {
                    Value::Bool(false)
                }
            }
        })
    }

    pub fn un_op(&self, op: UnOp, v: &Value) -> Result<Value, VmErr> {
        Ok(match op {
            UnOp::Not => Value::Bool(!self.truthy(v)),
            UnOp::Neg => Value::Number(-self.tn(v)),
            UnOp::Pos => Value::Number(self.tn(v)),
            UnOp::BitNot => Value::Number(!(self.tn(v) as i32) as f64),
            UnOp::Typeof => Value::String(
                match v {
                    Value::Undefined => "undefined",
                    Value::Null => "object",
                    Value::Bool(_) => "boolean",
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Object { .. } | Value::Array(_) | Value::GlobalObject => "object",
                    Value::Function { .. }
                    | Value::NativeFunction { .. }
                    | Value::HostFunction { .. }
                    | Value::Class { .. } => "function",
                    Value::Promise { .. } => "object",
                    Value::Generator { .. } => "object",
                    Value::Symbol(_) => "symbol",
                    Value::Error { .. } => "object",
                }
                .to_string(),
            ),
            UnOp::Void => Value::Undefined,
            UnOp::Delete => Value::Bool(true),
            UnOp::Inc => Value::Number(self.tn(v) + 1.0),
            UnOp::Dec => Value::Number(self.tn(v) - 1.0),
        })
    }

    pub fn keys(&self, o: &Value) -> Vec<String> {
        match o {
            Value::Object { props, .. } => props.borrow().iter().map(|(k, _)| k.clone()).collect(),
            Value::Array(i) => (0..i.borrow().len()).map(|x| x.to_string()).collect(),
            _ => vec![],
        }
    }

    pub fn truthy(&self, v: &Value) -> bool {
        v.is_truthy()
    }

    pub fn tn(&self, v: &Value) -> f64 {
        v.to_number()
    }

    pub fn leq(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Undefined) | (Value::Undefined, Value::Null) => true,
            (Value::GlobalObject, Value::GlobalObject) => true,
            (Value::Number(a), Value::String(b)) => {
                if let Ok(parsed) = b.parse::<f64>() {
                    *a == parsed
                } else {
                    false
                }
            }
            (Value::String(a), Value::Number(b)) => {
                if let Ok(parsed) = a.parse::<f64>() {
                    parsed == *b
                } else {
                    false
                }
            }
            (Value::Bool(a), Value::Number(b)) => {
                let num = if *a { 1.0 } else { 0.0 };
                num == *b
            }
            (Value::Number(a), Value::Bool(b)) => {
                let num = if *b { 1.0 } else { 0.0 };
                *a == num
            }
            (Value::Bool(a), Value::String(b)) => {
                let s = if *a { "true" } else { "false" };
                s == b
            }
            (Value::String(a), Value::Bool(b)) => {
                let s = if *b { "true" } else { "false" };
                a == s
            }
            _ => false,
        }
    }

    pub fn seq(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) | (Value::Undefined, Value::Undefined) => true,
            // The global aliases all denote the one global scope.
            (Value::GlobalObject, Value::GlobalObject) => true,
            _ => false,
        }
    }

    pub fn vs(&self, v: &Value) -> String {
        match v {
            Value::Undefined => "undefined".to_string(),
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{:.0}", n)
                } else {
                    n.to_string()
                }
            }
            Value::String(s) => s.clone(),
            Value::Object { .. } => "[object Object]".to_string(),
            Value::GlobalObject => "[object global]".to_string(),
            Value::Array(i) => i
                .borrow()
                .iter()
                .map(|x| self.vs(x))
                .collect::<Vec<_>>()
                .join(","),
            Value::Function { name, .. } => format!("function {}", name.as_deref().unwrap_or("")),
            Value::NativeFunction { name, .. } => format!("function {} [native]", name),
            Value::HostFunction { name, .. } => format!("function {} [native]", name),
            Value::Class { name, .. } => format!("class {}", name),
            Value::Promise { .. } => "[object Promise]".to_string(),
            Value::Generator { .. } => "[object Generator]".to_string(),
            Value::Symbol(s) => format!("Symbol({})", s),
            Value::Error { message, .. } => message.clone(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn sv(&self, s: &str) -> Value {
        if s == "undefined" {
            Value::Undefined
        } else if s == "null" {
            Value::Null
        } else if s == "true" {
            Value::Bool(true)
        } else if s == "false" {
            Value::Bool(false)
        } else if let Ok(n) = s.parse::<f64>() {
            Value::Number(n)
        } else {
            Value::String(s.to_string())
        }
    }
}
