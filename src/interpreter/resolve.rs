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
                } else if k == "__symbol_iterator__" {
                    Ok(Value::NativeFunction {
                        name: "[Symbol.iterator]".to_string(),
                        callable: array_iter,
                    })
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
                } else if k == "__symbol_iterator__" {
                    Ok(Value::NativeFunction {
                        name: "[Symbol.iterator]".to_string(),
                        callable: string_iter,
                    })
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
                } else if k == "__symbol_iterator__" {
                    // Generators are their own iterators: [Symbol.iterator]()
                    // returns `this`.
                    Ok(Value::NativeFunction {
                        name: "[Symbol.iterator]".to_string(),
                        callable: generator_iter_self,
                    })
                } else {
                    Ok(Value::Undefined)
                }
            }
            (Value::NativeFunction { name, .. }, Value::String(k)) => {
                // Well-known symbols and static methods on `Symbol`. A native
                // function cannot carry properties, so they are resolved here.
                if name == "Symbol" {
                    match k.as_str() {
                        "iterator" => Ok(Value::Symbol("Symbol.iterator".to_string())),
                        "toStringTag" => Ok(Value::Symbol("Symbol.toStringTag".to_string())),
                        "hasInstance" => Ok(Value::Symbol("Symbol.hasInstance".to_string())),
                        "toPrimitive" => Ok(Value::Symbol("Symbol.toPrimitive".to_string())),
                        "species" => Ok(Value::Symbol("Symbol.species".to_string())),
                        "asyncIterator" => Ok(Value::Symbol("Symbol.asyncIterator".to_string())),
                        "for" => Ok(Value::NativeFunction {
                            name: "for".to_string(),
                            callable: crate::builtins::symbol_for,
                        }),
                        "keyFor" => Ok(Value::NativeFunction {
                            name: "keyFor".to_string(),
                            callable: crate::builtins::symbol_key_for,
                        }),
                        _ => Ok(Value::Undefined),
                    }
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
            // Symbol-keyed property access: `arr[Symbol.iterator]`,
            // `str[Symbol.iterator]`, `gen[Symbol.iterator]`.
            (Value::Array(_), Value::Symbol(desc)) if desc == "Symbol.iterator" => {
                Ok(Value::NativeFunction {
                    name: "[Symbol.iterator]".to_string(),
                    callable: array_iter,
                })
            }
            (Value::String(_), Value::Symbol(desc)) if desc == "Symbol.iterator" => {
                Ok(Value::NativeFunction {
                    name: "[Symbol.iterator]".to_string(),
                    callable: string_iter,
                })
            }
            (Value::Generator { .. }, Value::Symbol(desc)) if desc == "Symbol.iterator" => {
                Ok(Value::NativeFunction {
                    name: "[Symbol.iterator]".to_string(),
                    callable: generator_iter_self,
                })
            }
            // Object symbol-keyed lookup: `obj[Symbol.iterator]` resolves the
            // internal `__symbol_iterator__` property.
            (Value::Object { props, proto }, Value::Symbol(desc)) => {
                let internal_key = if desc == "Symbol.iterator" {
                    "__symbol_iterator__".to_string()
                } else {
                    format!("__symbol:{}__", desc)
                };
                if let Some(v) = props.borrow().iter().find(|(k, _)| *k == internal_key) {
                    return Ok(v.1.clone());
                }
                if let Some(proto) = proto {
                    return self.prop(proto, p);
                }
                Ok(Value::Undefined)
            }
            _ => Ok(Value::Undefined),
        }
    }
}

// --- Iterator protocol native functions -------------------------------------

/// `[Symbol.iterator]()` on a generator returns the generator itself (generators
/// are their own iterators).
fn generator_iter_self(
    _interp: &mut super::Interpreter,
    this: super::Value,
    _args: Vec<super::Value>,
) -> Result<super::Value, crate::error::VmErr> {
    Ok(this)
}

/// `[Symbol.iterator]()` on an array returns a new array iterator object with a
/// `next()` method that walks the elements.
fn array_iter(
    _interp: &mut super::Interpreter,
    this: super::Value,
    _args: Vec<super::Value>,
) -> Result<super::Value, crate::error::VmErr> {
    use std::cell::RefCell;
    use std::rc::Rc;

    let items = match &this {
        super::Value::Array(a) => a.borrow().clone(),
        _ => vec![],
    };
    let cursor = Rc::new(RefCell::new(0usize));
    let items_rc = Rc::new(items);

    // Build an iterator object with a `next` method implemented as a closure
    // captured in a NativeFunction. Since NativeFunction takes a plain fn
    // pointer, we store the state in the object's properties and use a
    // stateful approach via a shared counter.
    let cursor_clone = cursor.clone();
    let items_clone = items_rc.clone();

    // We store the iterator state in the object itself and use a native
    // function that reads it back. The trick: store items and cursor index
    // as hidden properties on the iterator object.
    let iter_obj = super::Value::object(vec![
        ("__items__".to_string(), super::Value::Array(Rc::new(RefCell::new((*items_rc).clone())))),
        ("__cursor__".to_string(), super::Value::Number(0.0)),
        (
            "next".to_string(),
            super::Value::NativeFunction {
                name: "next".to_string(),
                callable: array_iter_next,
            },
        ),
    ]);

    // Suppress unused variable warnings for the closure-based approach we
    // didn't end up using.
    let _ = (cursor_clone, items_clone);

    Ok(iter_obj)
}

/// `next()` for an array iterator: reads `__items__` and `__cursor__` from
/// `this`, advances the cursor, and returns `{value, done}`.
fn array_iter_next(
    _interp: &mut super::Interpreter,
    this: super::Value,
    _args: Vec<super::Value>,
) -> Result<super::Value, crate::error::VmErr> {
    let items = match this.get_prop("__items__") {
        Some(super::Value::Array(a)) => a.borrow().clone(),
        _ => vec![],
    };
    let cursor = match this.get_prop("__cursor__") {
        Some(super::Value::Number(n)) => n as usize,
        _ => 0,
    };

    if cursor < items.len() {
        let val = items[cursor].clone();
        this.set_prop("__cursor__".to_string(), super::Value::Number((cursor + 1) as f64));
        Ok(super::call::iter_result(val, false))
    } else {
        Ok(super::call::iter_result(super::Value::Undefined, true))
    }
}

/// `[Symbol.iterator]()` on a string returns a character iterator.
fn string_iter(
    interp: &mut super::Interpreter,
    this: super::Value,
    _args: Vec<super::Value>,
) -> Result<super::Value, crate::error::VmErr> {
    use std::cell::RefCell;
    use std::rc::Rc;

    let chars: Vec<super::Value> = match &this {
        super::Value::String(s) => s.chars().map(|c| super::Value::String(c.to_string())).collect(),
        _ => vec![],
    };

    let iter_obj = super::Value::object(vec![
        ("__items__".to_string(), super::Value::Array(Rc::new(RefCell::new(chars)))),
        ("__cursor__".to_string(), super::Value::Number(0.0)),
        (
            "next".to_string(),
            super::Value::NativeFunction {
                name: "next".to_string(),
                callable: array_iter_next, // same logic: walk __items__ by __cursor__
            },
        ),
    ]);

    let _ = interp;
    Ok(iter_obj)
}
