//! Function and constructor calls, destructuring binding, catch handling,
//! and member assignment.

use std::cell::RefCell;
use std::rc::Rc;

use super::{Environment, Interpreter};
use crate::error::{VmErr, vm_err};
use crate::parser::{Pattern, Statement};
use crate::value::{GeneratorInner, PromiseState, Value};

impl Interpreter {
    pub(super) fn destructure(&mut self, pat: &Pattern, val: &Value) -> Result<Value, VmErr> {
        match pat {
            Pattern::Ident(name) => {
                self.global.borrow_mut().set(name, val.clone());
                Ok(val.clone())
            }
            Pattern::Array(elements) => {
                let values: Vec<Value> = match val {
                    Value::Array(arr) => arr.borrow().clone(),
                    Value::Object { props, .. } => {
                        let mut vals = Vec::new();
                        for (k, v) in props.borrow().entries() {
                            if let Ok(n) = k.parse::<usize>() {
                                while vals.len() <= n {
                                    vals.push(Value::Undefined);
                                }
                                vals[n] = v;
                            }
                        }
                        vals
                    }
                    Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
                    _ => vec![],
                };
                let mut rest_target = None;
                for (i, elem) in elements.iter().enumerate() {
                    if elem.is_rest() {
                        rest_target = Some(i);
                        break;
                    }
                    if let Some(v) = values.get(i) {
                        self.destructure(elem, v)?;
                    } else {
                        self.destructure(elem, &Value::Undefined)?;
                    }
                }
                if let Some(rest_idx) = rest_target
                    && let Pattern::Rest(rest_pat) = &elements[rest_idx]
                {
                    let rest_vals = values[rest_idx..].to_vec();
                    let rest_val = Value::array(rest_vals);
                    self.destructure(rest_pat, &rest_val)?;
                }
                Ok(val.clone())
            }
            Pattern::Object(props) => {
                let obj: Vec<(String, Value)> = match val {
                    Value::Object { props: oprops, .. } => oprops.borrow().entries(),
                    _ => vec![],
                };
                for (key, pat) in props {
                    let mut found = Value::Undefined;
                    for (k, v) in &obj {
                        if k == key {
                            found = v.clone();
                            break;
                        }
                    }
                    if let Some(p) = pat {
                        self.destructure(p, &found)?;
                    } else {
                        self.global.borrow_mut().set(key, found);
                    }
                }
                Ok(val.clone())
            }
            Pattern::Rest(_) => Ok(val.clone()),
            Pattern::Default(pat, default_expr) => {
                if matches!(val, Value::Undefined | Value::Null) {
                    let default_val = self.eval_expr(default_expr)?;
                    self.destructure(pat, &default_val)
                } else {
                    self.destructure(pat, val)
                }
            }
        }
    }

    pub(super) fn run_catch(
        &mut self,
        catch: &Option<(String, Vec<Statement>)>,
        err_val: Value,
    ) -> Result<Value, VmErr> {
        if let Some((p, cb)) = catch {
            let ce = Rc::new(RefCell::new(Environment::child(self.global.clone())));
            ce.borrow_mut().set(p, err_val);
            let s = self.global.clone();
            self.global = ce;
            let r = self.run(cb);
            self.global = s;
            r
        } else {
            // No catch clause: re-throw the original value.
            Err(VmErr::Throw(err_val))
        }
    }

    pub(super) fn assign_member(
        &mut self,
        obj: &Value,
        prop: &Value,
        val: Value,
    ) -> Result<Value, VmErr> {
        match (obj, prop) {
            (Value::Object { props, .. }, Value::String(k)) => {
                // If a setter is defined for this key, invoke it.
                let setter = props.borrow().get(k).filter(|xv| {
                    matches!(xv, Value::Function { name: Some(n), .. } if n.starts_with("set "))
                });
                if let Some(setter_fn) = setter {
                    self.call_this(&setter_fn, obj.clone(), vec![val.clone()])?;
                    return Ok(val);
                }
                props.borrow_mut().set(k.clone(), val.clone());
                Ok(val)
            }
            (Value::Array(items), Value::Number(i)) => {
                let mut items = items.borrow_mut();
                let idx = *i as usize;
                if idx < items.len() {
                    items[idx] = val.clone();
                } else {
                    while items.len() < idx {
                        items.push(Value::Undefined);
                    }
                    items.push(val.clone());
                }
                Ok(val)
            }
            _ => vm_err("Invalid assignment target"),
        }
    }

    pub(crate) fn call_this(
        &mut self,
        f: &Value,
        this_val: Value,
        args: Vec<Value>,
    ) -> Result<Value, VmErr> {
        match f {
            Value::Function {
                params,
                body,
                closure,
                is_arrow,
                is_async,
                is_generator,
                ..
            } => {
                // Calling a generator function does not run its body; it returns
                // a generator object whose `next()` method drives execution.
                if *is_generator {
                    let inner = GeneratorInner {
                        body: body.clone(),
                        closure: closure.clone(),
                        params: params.clone(),
                        args,
                        queue: Vec::new(),
                        started: false,
                        cursor: 0,
                    };
                    return Ok(Value::Generator {
                        inner: Rc::new(RefCell::new(inner)),
                    });
                }
                let parent_env = closure.clone().unwrap_or_else(|| self.global.clone());
                let fe = Rc::new(RefCell::new(Environment::child(parent_env)));
                // Regular functions bind their own `this`; arrows inherit the
                // enclosing lexical `this` through the closure chain.
                if !is_arrow {
                    fe.borrow_mut().set("this", this_val);
                }
                let mut has_rest = false;
                let mut rest_idx = 0;
                let mut rest_name = String::new();

                for (i, p) in params.iter().enumerate() {
                    if p.starts_with("...") {
                        has_rest = true;
                        rest_idx = i;
                        rest_name = p.trim_start_matches("...").to_string();
                        break;
                    }
                }

                if has_rest {
                    for (i, p) in params.iter().enumerate() {
                        if i == rest_idx {
                            let rest_args = args[i..].to_vec();
                            fe.borrow_mut().set(&rest_name, Value::array(rest_args));
                        } else {
                            let arg = if i < args.len() {
                                let is_rest_param = params
                                    .get(i + 1)
                                    .map(|p| p.starts_with("..."))
                                    .unwrap_or(false);
                                if !is_rest_param && i >= rest_idx {
                                    Value::Undefined
                                } else {
                                    args.get(i).cloned().unwrap_or(Value::Undefined)
                                }
                            } else {
                                Value::Undefined
                            };
                            fe.borrow_mut().set(p, arg);
                        }
                    }
                } else {
                    for (i, p) in params.iter().enumerate() {
                        let arg = args.get(i).cloned().unwrap_or(Value::Undefined);
                        fe.borrow_mut().set(p, arg);
                    }
                }

                // Create arguments object
                let args_obj = Value::object(
                    args.iter()
                        .enumerate()
                        .map(|(i, v)| (i.to_string(), v.clone()))
                        .collect(),
                );
                args_obj.set_prop("length".to_string(), Value::Number(args.len() as f64));
                fe.borrow_mut().set("arguments", args_obj);

                let s = self.global.clone();
                self.global = fe;
                let r = self.run(body);
                self.global = s;
                let result = match r {
                    Err(VmErr::Ret(v)) => Ok(v),
                    other => other,
                };
                if *is_async {
                    // An async function always resolves to a promise.
                    match result {
                        Ok(v) => Ok(Value::Promise {
                            state: PromiseState::Fulfilled,
                            value: Some(Box::new(v)),
                        }),
                        Err(VmErr::Throw(v)) => Ok(Value::Promise {
                            state: PromiseState::Rejected,
                            value: Some(Box::new(v)),
                        }),
                        other => other,
                    }
                } else {
                    result
                }
            }
            Value::NativeFunction { callable, .. } => callable(self, this_val, args),
            _ => vm_err("Not a function"),
        }
    }

    /// Run a constructor (class or function) against an already-created `this`,
    /// as done by `super(...)`. Returns `this`.
    pub(super) fn invoke_ctor(
        &mut self,
        f: &Value,
        this_val: Value,
        args: Vec<Value>,
    ) -> Result<Value, VmErr> {
        match f {
            Value::Class { constructor, .. } => {
                let ctor = constructor.as_ref().clone();
                self.call_this(&ctor, this_val.clone(), args)?;
                Ok(this_val)
            }
            Value::Function { .. } => {
                self.call_this(f, this_val.clone(), args)?;
                Ok(this_val)
            }
            _ => vm_err("Not a constructor"),
        }
    }

    pub(super) fn ctor(&mut self, f: &Value, args: Vec<Value>) -> Result<Value, VmErr> {
        match f {
            Value::Class {
                constructor,
                prototype,
                ..
            } => {
                // The instance's prototype is the class prototype (shared Rc, so
                // `instanceof` can compare identity).
                let inst = Value::object_with_proto(vec![], Some(prototype.clone()));
                let ctor = constructor.as_ref().clone();
                let r = self.call_this(&ctor, inst.clone(), args)?;
                match r {
                    Value::Object { .. } => Ok(r),
                    _ => Ok(inst),
                }
            }
            Value::Function {
                params,
                body,
                closure,
                ..
            } => {
                let inst = Value::object(vec![]);
                let parent_env = closure.clone().unwrap_or_else(|| self.global.clone());
                let fe = Rc::new(RefCell::new(Environment::child(parent_env)));
                fe.borrow_mut().set("this", inst.clone());

                let mut has_rest = false;
                let mut rest_idx = 0;
                let mut rest_name = String::new();
                for (i, p) in params.iter().enumerate() {
                    if p.starts_with("...") {
                        has_rest = true;
                        rest_idx = i;
                        rest_name = p.trim_start_matches("...").to_string();
                        break;
                    }
                }

                if has_rest {
                    for (i, p) in params.iter().enumerate() {
                        if i == rest_idx {
                            let rest_args = args[i..].to_vec();
                            fe.borrow_mut().set(&rest_name, Value::array(rest_args));
                        } else {
                            let is_rest_param = params
                                .get(i + 1)
                                .map(|p| p.starts_with("..."))
                                .unwrap_or(false);
                            let arg = if !is_rest_param && i >= rest_idx {
                                Value::Undefined
                            } else {
                                args.get(i).cloned().unwrap_or(Value::Undefined)
                            };
                            fe.borrow_mut().set(p, arg);
                        }
                    }
                } else {
                    for (i, p) in params.iter().enumerate() {
                        let arg = args.get(i).cloned().unwrap_or(Value::Undefined);
                        fe.borrow_mut().set(p, arg);
                    }
                }

                let args_obj = Value::object(
                    args.iter()
                        .enumerate()
                        .map(|(i, v)| (i.to_string(), v.clone()))
                        .collect(),
                );
                args_obj.set_prop("length".to_string(), Value::Number(args.len() as f64));
                fe.borrow_mut().set("arguments", args_obj);

                let s = self.global.clone();
                self.global = fe;
                let r = self.run(body);
                self.global = s;
                match r {
                    Err(VmErr::Ret(v)) => match v {
                        Value::Object { .. } => Ok(v),
                        _ => Ok(inst),
                    },
                    _ => Ok(inst),
                }
            }
            _ => vm_err("Not a constructor"),
        }
    }

    /// Run a generator body eagerly to completion, collecting every `yield`ed
    /// value into the returned queue. Parameters are bound in a fresh child of
    /// the generator's closure, mirroring a normal function call.
    pub(super) fn run_generator_body(
        &mut self,
        body: &[Statement],
        closure: &Option<super::Env>,
        params: &[String],
        args: &[Value],
    ) -> Vec<Value> {
        let parent_env = closure.clone().unwrap_or_else(|| self.global.clone());
        let fe = Rc::new(RefCell::new(Environment::child(parent_env)));
        for (i, p) in params.iter().enumerate() {
            let arg = args.get(i).cloned().unwrap_or(Value::Undefined);
            fe.borrow_mut().set(p, arg);
        }
        let queue = Rc::new(RefCell::new(Vec::new()));
        self.gen_yields.push(queue.clone());
        let s = self.global.clone();
        self.global = fe;
        let _ = self.run(body);
        self.global = s;
        self.gen_yields.pop();
        Rc::try_unwrap(queue)
            .map(|c| c.into_inner())
            .unwrap_or_default()
    }
}

/// `Generator.prototype.next`: runs the body on first call, then drains the
/// collected yields one per call, producing `{ value, done }` result objects.
pub(crate) fn generator_next(
    interp: &mut Interpreter,
    this: Value,
    _args: Vec<Value>,
) -> Result<Value, VmErr> {
    let inner_rc = match this {
        Value::Generator { inner } => inner,
        _ => return Ok(iter_result(Value::Undefined, true)),
    };

    // Phase 1: run the body to completion once, filling the queue.
    let needs_start = !inner_rc.borrow().started;
    if needs_start {
        let (body, closure, params, args) = {
            let mut inner = inner_rc.borrow_mut();
            inner.started = true;
            (
                inner.body.clone(),
                inner.closure.clone(),
                inner.params.clone(),
                inner.args.clone(),
            )
        };
        let queue = interp.run_generator_body(&body, &closure, &params, &args);
        inner_rc.borrow_mut().queue = queue;
    }

    // Phase 2: hand out the next queued value, or signal completion.
    let mut inner = inner_rc.borrow_mut();
    if inner.cursor < inner.queue.len() {
        let v = inner.queue[inner.cursor].clone();
        inner.cursor += 1;
        Ok(iter_result(v, false))
    } else {
        Ok(iter_result(Value::Undefined, true))
    }
}

/// Build an iterator result object `{ value, done }`.
fn iter_result(value: Value, done: bool) -> Value {
    Value::object(vec![
        ("value".to_string(), value),
        ("done".to_string(), Value::Bool(done)),
    ])
}
