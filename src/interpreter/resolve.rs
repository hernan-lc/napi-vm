//! Property resolution: direct lookup, prototype-chain walk, and getter
//! invocation.

use super::Interpreter;
use crate::error::VmErr;
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
use crate::lang::CompletionKind;
use crate::value::Value;

impl Interpreter {
    /// Enumerate properties visible on a simple runtime receiver such as
    /// `store` or `user.profile`. This only reads existing values and walks
    /// their prototype objects; it never evaluates guest source.
    #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
    pub(crate) fn completion_property_members(
        &self,
        receiver: &str,
    ) -> Vec<(String, CompletionKind)> {
        let Some(value) = self.completion_receiver_value(receiver) else {
            return Vec::new();
        };
        let mut members = Vec::new();
        self.collect_completion_members(&value, &mut members, 0);
        members.sort_by(|a, b| a.0.cmp(&b.0));
        members.dedup_by(|a, b| a.0 == b.0);
        members
    }

    #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
    fn completion_receiver_value(&self, receiver: &str) -> Option<Value> {
        let mut parts = receiver.split('.');
        let first = parts.next()?;
        if !is_completion_identifier(first) {
            return None;
        }
        let mut value = self.global.borrow().get(first)?;
        for part in parts {
            if !is_completion_identifier(part) {
                return None;
            }
            value = self.prop(&value, &Value::String(part.to_string())).ok()?;
        }
        Some(value)
    }

    #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
    fn collect_completion_members(
        &self,
        value: &Value,
        members: &mut Vec<(String, CompletionKind)>,
        depth: usize,
    ) {
        if depth > 32 {
            return;
        }
        let mut add = |name: &str, kind: CompletionKind| {
            if is_completion_identifier(name) && !name.starts_with("__") {
                members.push((name.to_string(), kind));
            }
        };

        match value {
            Value::Object { props, proto } => {
                for (name, property) in props.borrow().iter() {
                    add(name, completion_kind(property));
                }
                if let Some(proto) = proto {
                    self.collect_completion_members(proto, members, depth + 1);
                }
            }
            Value::Array(_) => {
                for name in
                    crate::lang::catalog::prototype_members(crate::lang::catalog::ProtoKind::Array)
                {
                    add(name, CompletionKind::Method);
                }
            }
            Value::String(_) => {
                for name in
                    crate::lang::catalog::prototype_members(crate::lang::catalog::ProtoKind::String)
                {
                    add(name, CompletionKind::Method);
                }
            }
            Value::Number(_) => {
                for name in
                    crate::lang::catalog::prototype_members(crate::lang::catalog::ProtoKind::Number)
                {
                    add(name, CompletionKind::Method);
                }
            }
            Value::Promise { .. } => {
                for name in crate::lang::catalog::prototype_members(
                    crate::lang::catalog::ProtoKind::Promise,
                ) {
                    add(name, CompletionKind::Method);
                }
            }
            Value::Class(class) => {
                for (name, property) in class.statics.borrow().iter() {
                    add(name, completion_kind(property));
                }
                self.collect_completion_members(&class.prototype, members, depth + 1);
            }
            Value::GlobalObject => {
                for name in self.global.borrow().all_keys() {
                    add(&name, CompletionKind::Global);
                }
            }
            Value::HostFunction { .. }
            | Value::Function(_)
            | Value::NativeFunction { .. }
            | Value::Undefined
            | Value::Null
            | Value::Bool(_)
            | Value::Error(_)
            | Value::Generator { .. }
            | Value::StringIterator { .. }
            | Value::HostPending { .. }
            | Value::Symbol(_) => {}
        }
    }

    /// Resolve a property value, invoking it if it is a getter.
    pub(super) fn get_prop_value(&mut self, o: &Value, p: &Value) -> Result<Value, VmErr> {
        let v = self.prop(o, p)?;
        if let Value::Function(f) = &v
            && !f.is_arrow
            && f.name.as_ref().is_some_and(|n| n.starts_with("get "))
        {
            return self.call_this(&v, o.clone(), vec![]);
        }
        Ok(v)
    }

    pub(super) fn prop(&self, o: &Value, p: &Value) -> Result<Value, VmErr> {
        match (o, p) {
            // `window.x` / `globalThis.x` / `self.x` read a real global.
            (Value::GlobalObject, Value::String(k)) => Ok(self
                .persistent_global
                .borrow()
                .get(k)
                .unwrap_or(Value::Undefined)),
            (Value::Object { .. }, Value::String(k)) => {
                let mut current = o;
                for _ in 0..=crate::value::MAX_PROTOTYPE_DEPTH {
                    match current {
                        Value::Object { props, proto } => {
                            if let Some((_, value)) = props.borrow().iter().find(|(xk, _)| xk == k)
                            {
                                return Ok(value.clone());
                            }
                            let Some(next) = proto.as_deref() else {
                                return Ok(Value::Undefined);
                            };
                            current = next;
                        }
                        _ => return Ok(Value::Undefined),
                    }
                }
                Err(crate::value::limit_err(
                    "Maximum prototype chain depth exceeded",
                ))
            }
            (Value::Array(items), Value::Number(i)) => {
                let items = items.borrow();
                if !i.is_finite() || *i < 0.0 || i.fract() != 0.0 {
                    Ok(Value::Undefined)
                } else {
                    let idx = *i as usize;
                    if idx < items.len() {
                        Ok(items[idx].clone())
                    } else {
                        Ok(Value::Undefined)
                    }
                }
            }
            (Value::Array(items), Value::String(k)) => {
                if k == "length" {
                    Ok(Value::Number(items.borrow().len() as f64))
                } else if k == "__symbol_iterator__" {
                    Ok(Value::NativeFunction {
                        name: "[Symbol.iterator]".into(),
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
                        name: "[Symbol.iterator]".into(),
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
            (Value::Promise { .. }, Value::String(k)) => {
                Ok(crate::builtins::promise_method(k).unwrap_or(Value::Undefined))
            }
            (Value::String(s), Value::Number(i)) => {
                let idx = *i as usize;
                Ok(s.chars()
                    .nth(idx)
                    .map(|c| Value::String(c.to_string()))
                    .unwrap_or(Value::Undefined))
            }
            (Value::Class(c), Value::String(k)) => {
                if k == "prototype" {
                    return Ok(c.prototype.as_ref().clone());
                }
                if k == "name" {
                    return Ok(Value::String(c.name.clone()));
                }
                if let Some(v) = c.statics.borrow().iter().find(|(xk, _)| xk == k) {
                    return Ok(v.1.clone());
                }
                Ok(Value::Undefined)
            }
            (Value::Generator { .. }, Value::String(k)) => {
                if k == "next" {
                    Ok(Value::NativeFunction {
                        name: "next".into(),
                        callable: super::call::generator_next,
                    })
                } else if k == "__symbol_iterator__" {
                    // Generators are their own iterators: [Symbol.iterator]()
                    // returns `this`.
                    Ok(Value::NativeFunction {
                        name: "[Symbol.iterator]".into(),
                        callable: generator_iter_self,
                    })
                } else {
                    Ok(Value::Undefined)
                }
            }
            (Value::StringIterator { .. }, Value::String(k)) if k == "next" => {
                Ok(Value::NativeFunction {
                    name: "next".into(),
                    callable: string_iter_next,
                })
            }
            (Value::NativeFunction { name, .. }, Value::String(k)) => {
                // Well-known symbols and static methods on `Symbol`. A native
                // function cannot carry properties, so they are resolved here.
                if name.as_ref() == "Symbol" {
                    match k.as_str() {
                        "iterator" => Ok(Value::Symbol("Symbol.iterator".to_string())),
                        "toStringTag" => Ok(Value::Symbol("Symbol.toStringTag".to_string())),
                        "hasInstance" => Ok(Value::Symbol("Symbol.hasInstance".to_string())),
                        "toPrimitive" => Ok(Value::Symbol("Symbol.toPrimitive".to_string())),
                        "species" => Ok(Value::Symbol("Symbol.species".to_string())),
                        "asyncIterator" => Ok(Value::Symbol("Symbol.asyncIterator".to_string())),
                        "for" => Ok(Value::NativeFunction {
                            name: "for".into(),
                            callable: crate::builtins::symbol_for,
                        }),
                        "keyFor" => Ok(Value::NativeFunction {
                            name: "keyFor".into(),
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
                    Ok(Value::String(name.to_string()))
                } else {
                    Ok(Value::Undefined)
                }
            }
            // Symbol-keyed property access: `arr[Symbol.iterator]`,
            // `str[Symbol.iterator]`, `gen[Symbol.iterator]`.
            (Value::Array(_), Value::Symbol(desc)) if desc == "Symbol.iterator" => {
                Ok(Value::NativeFunction {
                    name: "[Symbol.iterator]".into(),
                    callable: array_iter,
                })
            }
            (Value::String(_), Value::Symbol(desc)) if desc == "Symbol.iterator" => {
                Ok(Value::NativeFunction {
                    name: "[Symbol.iterator]".into(),
                    callable: string_iter,
                })
            }
            (Value::Generator { .. }, Value::Symbol(desc)) if desc == "Symbol.iterator" => {
                Ok(Value::NativeFunction {
                    name: "[Symbol.iterator]".into(),
                    callable: generator_iter_self,
                })
            }
            (Value::StringIterator { .. }, Value::Symbol(desc)) if desc == "Symbol.iterator" => {
                Ok(Value::NativeFunction {
                    name: "[Symbol.iterator]".into(),
                    callable: string_iter_self,
                })
            }
            // Object symbol-keyed lookup: `obj[Symbol.iterator]` resolves the
            // internal `__symbol_iterator__` property.
            (Value::Object { .. }, Value::Symbol(desc)) => {
                let internal_key = if desc == "Symbol.iterator" {
                    "__symbol_iterator__".to_string()
                } else {
                    format!("__symbol:{}__", desc)
                };
                let mut current = o;
                for _ in 0..=crate::value::MAX_PROTOTYPE_DEPTH {
                    match current {
                        Value::Object { props, proto } => {
                            if let Some((_, value)) =
                                props.borrow().iter().find(|(key, _)| *key == internal_key)
                            {
                                return Ok(value.clone());
                            }
                            let Some(next) = proto.as_deref() else {
                                return Ok(Value::Undefined);
                            };
                            current = next;
                        }
                        _ => return Ok(Value::Undefined),
                    }
                }
                Err(crate::value::limit_err(
                    "Maximum prototype chain depth exceeded",
                ))
            }
            // Internal errors surface to guest `catch` blocks as error objects
            // with readable `name`/`message` properties.
            (Value::Error(e), Value::String(k)) => match k.as_str() {
                "message" => Ok(Value::String(e.message.clone())),
                "name" => Ok(Value::String(e.name.clone())),
                _ => Ok(Value::Undefined),
            },
            _ => Ok(Value::Undefined),
        }
    }
}

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
fn is_completion_identifier(name: &str) -> bool {
    !name.is_empty()
        && name.chars().enumerate().all(|(index, c)| {
            if index == 0 {
                c.is_ascii_alphabetic() || c == '_' || c == '$'
            } else {
                c.is_ascii_alphanumeric() || c == '_' || c == '$'
            }
        })
}

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
fn completion_kind(value: &Value) -> CompletionKind {
    match value {
        Value::Function(_)
        | Value::NativeFunction { .. }
        | Value::HostFunction { .. }
        | Value::Class(_) => CompletionKind::Method,
        _ => CompletionKind::Property,
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
        (
            "__items__".to_string(),
            super::Value::Array(Rc::new(RefCell::new((*items_rc).clone()))),
        ),
        ("__cursor__".to_string(), super::Value::Number(0.0)),
        (
            "next".to_string(),
            super::Value::NativeFunction {
                name: "next".into(),
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
    let items_prop = this.get_prop("__items__");
    let items = match &items_prop {
        Some(super::Value::Array(a)) => a.borrow().clone(),
        _ => vec![],
    };
    let cursor = match this.get_prop("__cursor__") {
        Some(super::Value::Number(n)) => n as usize,
        _ => 0,
    };

    if cursor < items.len() {
        let val = items[cursor].clone();
        this.set_prop(
            "__cursor__".to_string(),
            super::Value::Number((cursor + 1) as f64),
        )?;
        Ok(super::call::iter_result(val, false))
    } else {
        Ok(super::call::iter_result(super::Value::Undefined, true))
    }
}

/// `[Symbol.iterator]()` on a string returns a character iterator.
fn string_iter(
    _interp: &mut super::Interpreter,
    this: super::Value,
    _args: Vec<super::Value>,
) -> Result<super::Value, crate::error::VmErr> {
    use std::cell::RefCell;
    use std::rc::Rc;

    let source: Rc<str> = match &this {
        super::Value::String(s) => Rc::from(s.clone()),
        _ => Rc::from(""),
    };

    Ok(super::Value::StringIterator {
        inner: Rc::new(RefCell::new(crate::value::StringIteratorData {
            source,
            cursor: 0,
        })),
    })
}

/// `next()` for the lazy string iterator. The cursor is a UTF-8 byte offset,
/// but the yielded value is always one complete Unicode scalar value.
fn string_iter_next(
    _interp: &mut super::Interpreter,
    this: super::Value,
    _args: Vec<super::Value>,
) -> Result<super::Value, crate::error::VmErr> {
    let super::Value::StringIterator { inner } = &this else {
        return Ok(super::call::iter_result(super::Value::Undefined, true));
    };
    let mut state = inner.borrow_mut();
    let Some(rest) = state.source.get(state.cursor..) else {
        return Ok(super::call::iter_result(super::Value::Undefined, true));
    };
    let Some(ch) = rest.chars().next() else {
        return Ok(super::call::iter_result(super::Value::Undefined, true));
    };
    state.cursor += ch.len_utf8();
    Ok(super::call::iter_result(
        super::Value::String(ch.to_string()),
        false,
    ))
}

fn string_iter_self(
    _interp: &mut super::Interpreter,
    this: super::Value,
    _args: Vec<super::Value>,
) -> Result<super::Value, crate::error::VmErr> {
    Ok(this)
}
