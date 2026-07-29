use super::Interpreter;
use crate::error::{VmErr, vm_err};
use crate::value::Value;

impl Interpreter {
    pub fn bin_op(&self, op: &str, l: &Value, r: &Value) -> Result<Value, VmErr> {
        Ok(match op {
            "+" => match (l, r) {
                (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
                (Value::String(a), _) => Value::String(format!("{}{}", a, self.vs(r))),
                (_, Value::String(b)) => Value::String(format!("{}{}", self.vs(l), b)),
                _ => Value::String(format!("{}{}", self.vs(l), self.vs(r))),
            },
            "-" => Value::Number(self.tn(l) - self.tn(r)),
            "*" => Value::Number(self.tn(l) * self.tn(r)),
            "/" => Value::Number(self.tn(l) / self.tn(r)),
            "%" => Value::Number(self.tn(l) % self.tn(r)),
            "==" => Value::Bool(self.leq(l, r)),
            "!=" => Value::Bool(!self.leq(l, r)),
            "===" => Value::Bool(self.seq(l, r)),
            "!==" => Value::Bool(!self.seq(l, r)),
            "<" => Value::Bool(self.tn(l) < self.tn(r)),
            ">" => Value::Bool(self.tn(l) > self.tn(r)),
            "<=" => Value::Bool(self.tn(l) <= self.tn(r)),
            ">=" => Value::Bool(self.tn(l) >= self.tn(r)),
            "&&" => {
                if self.truthy(l) {
                    r.clone()
                } else {
                    l.clone()
                }
            }
            "||" => {
                if self.truthy(l) {
                    l.clone()
                } else {
                    r.clone()
                }
            }
            "instanceof" => Value::Bool(false),
            "in" => {
                if let (Value::String(k), Value::Object(p)) = (l, r) {
                    Value::Bool(p.iter().any(|(x, _)| x == k))
                } else {
                    Value::Bool(false)
                }
            }
            _ => return vm_err(format!("Unknown op: {}", op)),
        })
    }

    pub fn un_op(&self, op: &str, v: &Value) -> Result<Value, VmErr> {
        Ok(match op {
            "!" => Value::Bool(!self.truthy(v)),
            "-" => Value::Number(-self.tn(v)),
            "+" => Value::Number(self.tn(v)),
            "typeof" => Value::String(
                match v {
                    Value::Undefined => "undefined",
                    Value::Null => "object",
                    Value::Bool(_) => "boolean",
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Object(_) | Value::Array(_) => "object",
                    Value::Function { .. } | Value::NativeFunction { .. } => "function",
                }
                .to_string(),
            ),
            "void" => Value::Undefined,
            "delete" => Value::Bool(true),
            "++" => Value::Number(self.tn(v) + 1.0),
            "--" => Value::Number(self.tn(v) - 1.0),
            _ => return vm_err(format!("Unknown unary: {}", op)),
        })
    }

    pub fn keys(&self, o: &Value) -> Vec<String> {
        match o {
            Value::Object(p) => p.iter().map(|(k, _)| k.clone()).collect(),
            Value::Array(i) => (0..i.len()).map(|x| x.to_string()).collect(),
            _ => vec![],
        }
    }

    pub fn truthy(&self, v: &Value) -> bool {
        match v {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            Value::Null | Value::Undefined => false,
            _ => true,
        }
    }

    pub fn tn(&self, v: &Value) -> f64 {
        match v {
            Value::Number(n) => *n,
            Value::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Value::String(s) => s.parse().unwrap_or(0.0),
            Value::Null => 0.0,
            Value::Undefined => f64::NAN,
            _ => 0.0,
        }
    }

    pub fn leq(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Undefined) | (Value::Undefined, Value::Null) => true,
            _ => false,
        }
    }

    pub fn seq(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) | (Value::Undefined, Value::Undefined) => true,
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
            Value::Object(_) => "[object Object]".to_string(),
            Value::Array(i) => i.iter().map(|x| self.vs(x)).collect::<Vec<_>>().join(","),
            Value::Function { name, .. } => format!("function {}", name.as_deref().unwrap_or("")),
            Value::NativeFunction { name, .. } => format!("function {} [native]", name),
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
