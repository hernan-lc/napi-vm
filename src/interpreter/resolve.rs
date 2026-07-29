//! Property resolution: direct lookup, prototype-chain walk, and getter
//! invocation.

use super::Interpreter;
use crate::error::VmErr;
use crate::value::Value;

impl Interpreter {
    /// Resolve a property value, invoking it if it is a getter.
    pub(super) fn get_prop_value(&mut self, o: &Value, p: &Value) -> Result<Value, VmErr> {
        let v = self.prop(o, p)?;
        if let Value::Function {
            name: Some(n),
            is_arrow: false,
            ..
        } = &v
            && n.starts_with("get ")
        {
            return self.call_this(&v, o.clone(), vec![]);
        }
        Ok(v)
    }

    pub(super) fn prop(&self, o: &Value, p: &Value) -> Result<Value, VmErr> {
        match (o, p) {
            // `window.x` / `globalThis.x` / `self.x` read a real global.
            (Value::GlobalObject, Value::String(k)) => {
                Ok(self.global.borrow().get(k).unwrap_or(Value::Undefined))
            }
            (Value::Object { props, proto }, Value::String(k)) => {
                if let Some(v) = props.borrow().iter().find(|(xk, _)| xk == k) {
                    return Ok(v.1.clone());
                }
                if let Some(proto) = proto {
                    return self.prop(proto, p);
                }
                Ok(Value::Undefined)
            }
            (Value::Array(items), Value::Number(i)) => {
                let items = items.borrow();
                let idx = *i as usize;
                if idx < items.len() {
                    Ok(items[idx].clone())
                } else {
                    Ok(Value::Undefined)
                }
            }
            (Value::Array(items), Value::String(k)) => {
                if k == "length" {
                    Ok(Value::Number(items.borrow().len() as f64))
                } else if let Ok(idx) = k.parse::<usize>() {
                    let items = items.borrow();
                    if idx < items.len() {
                        Ok(items[idx].clone())
                    } else {
                        Ok(Value::Undefined)
                    }
                } else if let Some(m) = crate::builtins::array_method(k) {
                    Ok(m)
                } else {
                    Ok(Value::Undefined)
                }
            }
            (Value::String(s), Value::String(k)) => {
                if k == "length" {
                    Ok(Value::Number(s.chars().count() as f64))
                } else if let Ok(idx) = k.parse::<usize>() {
                    Ok(s.chars()
                        .nth(idx)
                        .map(|c| Value::String(c.to_string()))
                        .unwrap_or(Value::Undefined))
                } else if let Some(m) = crate::builtins::string_method(k) {
                    Ok(m)
                } else {
                    Ok(Value::Undefined)
                }
            }
            (Value::Number(_), Value::String(k)) => {
                if let Some(m) = crate::builtins::number_method(k) {
                    Ok(m)
                } else {
                    Ok(Value::Undefined)
                }
            }
            (Value::String(s), Value::Number(i)) => {
                let idx = *i as usize;
                Ok(s.chars()
                    .nth(idx)
                    .map(|c| Value::String(c.to_string()))
                    .unwrap_or(Value::Undefined))
            }
            (
                Value::Class {
                    statics,
                    prototype,
                    name,
                    ..
                },
                Value::String(k),
            ) => {
                if k == "prototype" {
                    return Ok(prototype.as_ref().clone());
                }
                if k == "name" {
                    return Ok(Value::String(name.clone()));
                }
                if let Some(v) = statics.borrow().iter().find(|(xk, _)| xk == k) {
                    return Ok(v.1.clone());
                }
                Ok(Value::Undefined)
            }
            (Value::Generator { .. }, Value::String(k)) => {
                if k == "next" {
                    Ok(Value::NativeFunction {
                        name: "next".to_string(),
                        callable: super::call::generator_next,
                    })
                } else {
                    Ok(Value::Undefined)
                }
            }
            (Value::NativeFunction { name, .. }, Value::String(k)) => {
                // The well-known `Symbol.iterator` member. A native function
                // cannot carry properties, so it is resolved here by name.
                if name == "Symbol" && k == "iterator" {
                    Ok(Value::Symbol("Symbol.iterator".to_string()))
                } else {
                    Ok(Value::Undefined)
                }
            }
            (Value::HostFunction { name, .. }, Value::String(k)) => {
                if k == "name" {
                    Ok(Value::String(name.clone()))
                } else {
                    Ok(Value::Undefined)
                }
            }
            _ => Ok(Value::Undefined),
        }
    }
}
