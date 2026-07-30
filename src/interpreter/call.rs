//! Function and constructor calls, destructuring binding, catch handling,
//! and member assignment.

use std::cell::RefCell;
use std::rc::Rc;

use super::{Environment, Interpreter};
use crate::error::{VmErr, vm_err};
use crate::parser::{Pattern, Statement};
use crate::span::Span;
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
                        for (k, v) in props.borrow().iter() {
                            if let Ok(n) = k.parse::<usize>() {
                                while vals.len() <= n {
                                    vals.push(Value::Undefined);
                                }
                                vals[n] = v.clone();
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
                    Value::Object { props: oprops, .. } => oprops.borrow().clone(),
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
    ) -> Result<(), VmErr> {
        match (obj, prop) {
            (Value::Object { props, .. }, Value::String(k)) => {
                // If a setter is defined for this key, invoke it.
                let setter = props
                    .borrow()
                    .iter()
                    .find(|(xk, xv)| {
                        xk == k
                            && matches!(xv, Value::Function { name: Some(n), .. } if n.starts_with("set "))
                    })
                    .map(|(_, xv)| xv.clone());
                if let Some(setter_fn) = setter {
                    self.call_this(&setter_fn, obj.clone(), vec![val])?;
                    return Ok(());
                }
                let mut props = props.borrow_mut();
                for (xk, xv) in props.iter_mut() {
                    if xk == k {
                        *xv = val;
                        return Ok(());
                    }
                }
                props.push((k.clone(), val));
                Ok(())
            }
            // `window.x = v` / `globalThis.x = v` define a real global.
            (Value::GlobalObject, Value::String(k)) => {
                self.global.borrow_mut().set(k, val);
                Ok(())
            }
            (Value::Array(items), Value::Number(i)) => {
                let mut items = items.borrow_mut();
                let idx = *i as usize;
                if idx < items.len() {
                    items[idx] = val;
                } else {
                    while items.len() < idx {
                        items.push(Value::Undefined);
                    }
                    items.push(val);
                }
                Ok(())
            }
            _ => Err(VmErr::Msg("Invalid assignment target".to_string())),
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
                name,
                params,
                body,
                closure,
                is_arrow,
                is_async,
                is_generator,
                uses_arguments,
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
                        to_gen: None,
                        from_gen: None,
                        started: false,
                        done: false,
                        return_value: None,
                    };
                    return Ok(Value::Generator {
                        inner: Rc::new(RefCell::new(inner)),
                    });
                }
                // Recursion guard: each VM call costs several native frames,
                // so unbounded guest recursion would SIGSEGV the host. Fail
                // with a catchable RangeError instead (V8 semantics).
                if self.get_stack().len() >= crate::interpreter::MAX_CALL_DEPTH {
                    return Err(VmErr::Msg(
                        "RangeError: Maximum call stack size exceeded".to_string(),
                    ));
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

                // Create the (detached) arguments object only when the body
                // actually reads it; most functions never do.
                if *uses_arguments {
                    let args_obj = Value::object(
                        args.iter()
                            .enumerate()
                            .map(|(i, v)| (i.to_string(), v.clone()))
                            .collect(),
                    );
                    args_obj.set_prop("length".to_string(), Value::Number(args.len() as f64));
                    fe.borrow_mut().set("arguments", args_obj);
                }

                let s = self.global.clone();
                self.global = fe;
                let fname = name.clone().unwrap_or_else(|| "<anonymous>".to_string());
                self.push_frame(&fname, Span::unknown());
                let r = self.run(body);
                // Convert a bare message into a located runtime error *before*
                // popping the frame, so the snapshot carries the full call
                // chain. Only the error path pays for the snapshot — the
                // success path (the overwhelming majority of calls) clones
                // nothing. (Snapshotting unconditionally here was the single
                // largest per-call cost: O(depth) String clones per call.)
                let result = match r {
                    Err(VmErr::Ret(v)) => Ok(v),
                    Err(VmErr::Msg(msg)) => Err(VmErr::RuntimeError {
                        message: msg,
                        span: None,
                        stack: self.get_stack().to_vec(),
                    }),
                    other => other,
                };
                self.pop_frame();
                self.global = s;
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
            Value::HostFunction { id, .. } => {
                // Clone the bridge out so we don't hold a borrow on `self`
                // across the host call (which may re-enter the VM).
                let bridge = self.host.clone().ok_or_else(|| {
                    VmErr::Msg("cannot call host function: no bridge attached".to_string())
                })?;
                bridge.call_host(*id, args)
            }
            _ => {
                let type_name = match f {
                    Value::String(_) => "string",
                    Value::Number(_) => "number",
                    Value::Bool(_) => "boolean",
                    Value::Null => "null",
                    Value::Undefined => "undefined",
                    Value::Array(_) => "array",
                    Value::Object { .. } => "object",
                    _ => "unknown",
                };
                vm_err(format!("TypeError: {} is not a function", type_name))
            }
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
            _ => {
                let type_name = match f {
                    Value::String(_) => "string",
                    Value::Number(_) => "number",
                    Value::Bool(_) => "boolean",
                    Value::Null => "null",
                    Value::Undefined => "undefined",
                    Value::Array(_) => "array",
                    Value::Object { .. } => "object",
                    _ => "unknown",
                };
                vm_err(format!("TypeError: {} is not a constructor", type_name))
            }
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
                uses_arguments,
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

                if *uses_arguments {
                    let args_obj = Value::object(
                        args.iter()
                            .enumerate()
                            .map(|(i, v)| (i.to_string(), v.clone()))
                            .collect(),
                    );
                    args_obj.set_prop("length".to_string(), Value::Number(args.len() as f64));
                    fe.borrow_mut().set("arguments", args_obj);
                }

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
            _ => {
                let type_name = match f {
                    Value::String(_) => "string",
                    Value::Number(_) => "number",
                    Value::Bool(_) => "boolean",
                    Value::Null => "null",
                    Value::Undefined => "undefined",
                    Value::Array(_) => "array",
                    Value::Object { .. } => "object",
                    _ => "unknown",
                };
                vm_err(format!("TypeError: {} is not a constructor", type_name))
            }
        }
    }
}

/// Spawn the generator thread. The thread runs the generator body in its own
/// interpreter, blocking at each `yield` to send the value back and wait for a
/// resume signal. This gives true mid-body suspension: infinite generators work
/// correctly, and yields inside loops/conditionals/try-finally all behave as
/// specified.
pub(crate) fn spawn_generator_thread(
    body: Rc<Vec<Statement>>,
    closure: Option<super::Env>,
    params: Rc<Vec<String>>,
    args: Vec<Value>,
    builtins_env: Option<super::Env>,
) -> (
    std::sync::mpsc::Sender<crate::value::GenResume>,
    std::sync::mpsc::Receiver<crate::value::GenYield>,
) {
    use crate::value::{GenResume, GenYield, SendGenInit};

    let (to_gen_tx, to_gen_rx) = std::sync::mpsc::channel::<GenResume>();
    let (from_gen_tx, from_gen_rx) = std::sync::mpsc::channel::<GenYield>();

    // Bundle everything the thread needs into a single `Send` wrapper.
    let init = SendGenInit {
        body,
        closure,
        params,
        args,
        to_gen_rx,
        from_gen_tx,
        builtins_env,
    };

    // Use a function boundary so the compiler sees only `SendGenInit` (which is
    // `Send`) crossing the thread boundary, not the individual `Rc` fields.
    //
    // The thread gets an 8MB stack to match the main thread's: the recursion
    // limit (`MAX_CALL_DEPTH`) is calibrated against 8MB, and the default 2MB
    // thread stack would overflow well before the limit kicks in. If the
    // spawn fails, `init` is dropped, both channels close, and the main
    // thread's `recv()` sees a disconnect, which it already handles as a
    // completed generator.
    let _ = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || run_generator_thread(init));

    (to_gen_tx, from_gen_rx)
}

/// Entry point for the generator thread. Waits for the first resume signal
/// (matching JS semantics: the body does not execute until the first `next()`),
/// then runs the generator body to completion, communicating yields and the
/// final return value over the channel.
fn run_generator_thread(init: crate::value::SendGenInit) {
    use crate::value::{GenResume, GenYield, SendValue};

    let crate::value::SendGenInit {
        body,
        closure,
        params,
        args,
        to_gen_rx,
        from_gen_tx,
        builtins_env,
    } = init;

    // Wait for the first `next()` call before executing any of the body.
    // This matches JS semantics where `function*` bodies are lazy.
    match to_gen_rx.recv() {
        Ok(GenResume::Next(_)) => {}
        Err(_) => return, // Main thread dropped the generator without calling next().
    }

    // Build a fresh interpreter for the generator thread. If we have a
    // builtins environment, chain to it so standard library functions work.
    let mut interp = if let Some(builtins) = builtins_env {
        let mut i = Interpreter::new();
        i.global = Rc::new(RefCell::new(Environment::child(builtins)));
        i
    } else {
        Interpreter::with_builtins()
    };

    // Install the generator channel so `yield` expressions can communicate.
    interp.gen_channel = Some(super::GenChannel {
        to_main: from_gen_tx,
        from_main: to_gen_rx,
    });

    // Set up the function environment with bound parameters.
    let parent_env = closure.unwrap_or_else(|| interp.global.clone());
    let fe = Rc::new(RefCell::new(Environment::child(parent_env)));
    for (i, p) in params.iter().enumerate() {
        let arg = args.get(i).cloned().unwrap_or(Value::Undefined);
        fe.borrow_mut().set(p, arg);
    }

    interp.global = fe;

    // Run the body. Yields are handled via the channel inside eval_expr.
    let result = interp.run(&body);

    // Signal completion or error to the main thread.
    let chan = interp.gen_channel.as_ref().unwrap();
    match result {
        Ok(v) | Err(VmErr::Ret(v)) => {
            let _ = chan.to_main.send(GenYield::Returned(SendValue(v).0));
        }
        Err(VmErr::Throw(v)) => {
            let msg = match &v {
                Value::String(s) => s.clone(),
                Value::Error { message, .. } => message.clone(),
                other => interp.vs(other),
            };
            let _ = chan.to_main.send(GenYield::Threw(msg));
        }
        Err(VmErr::Msg(m)) => {
            let _ = chan.to_main.send(GenYield::Threw(m));
        }
        Err(VmErr::RuntimeError { message, .. }) => {
            let _ = chan.to_main.send(GenYield::Threw(message));
        }
        // A break/continue that escapes the generator body is a runtime error.
        Err(e @ (VmErr::Break(_) | VmErr::Continue(_))) => {
            let _ = chan.to_main.send(GenYield::Threw(format!("{}", e)));
        }
    }
}

/// `Generator.prototype.next`: resumes the generator thread (spawning it on
/// first call), waits for the next yielded or returned value, and produces a
/// `{ value, done }` result object.
pub(crate) fn generator_next(
    interp: &mut Interpreter,
    this: Value,
    args: Vec<Value>,
) -> Result<Value, VmErr> {
    use crate::value::{GenResume, GenYield};

    let inner_rc = match &this {
        Value::Generator { inner } => inner.clone(),
        _ => return Ok(iter_result(Value::Undefined, true)),
    };

    let mut inner = inner_rc.borrow_mut();

    if inner.done {
        let rv = inner.return_value.clone().unwrap_or(Value::Undefined);
        return Ok(iter_result(rv, true));
    }

    // Spawn the thread on first call.
    if !inner.started {
        inner.started = true;

        // Find the builtins environment (the parent of the current global) so
        // the generator thread has access to standard library functions.
        let builtins_env = interp.global.borrow().parent_env();

        let (tx, rx) = spawn_generator_thread(
            inner.body.clone(),
            inner.closure.clone(),
            inner.params.clone(),
            inner.args.clone(),
            builtins_env,
        );
        inner.to_gen = Some(tx);
        inner.from_gen = Some(rx);
    }

    // Send the resume signal with the optional sent value.
    let sent = args.first().cloned();
    let to_gen = inner.to_gen.as_ref().unwrap();
    to_gen
        .send(GenResume::Next(sent))
        .map_err(|_| VmErr::Msg("generator thread terminated".to_string()))?;

    // Wait for the generator to yield or finish.
    let from_gen = inner.from_gen.as_ref().unwrap();
    match from_gen.recv() {
        Ok(GenYield::Yielded(v)) => Ok(iter_result(v, false)),
        Ok(GenYield::Returned(v)) => {
            inner.done = true;
            inner.return_value = Some(v.clone());
            inner.to_gen = None;
            inner.from_gen = None;
            Ok(iter_result(v, true))
        }
        Ok(GenYield::Threw(msg)) => {
            inner.done = true;
            inner.to_gen = None;
            inner.from_gen = None;
            Err(VmErr::Msg(msg))
        }
        Err(_) => {
            // Channel closed: the thread panicked or was dropped.
            inner.done = true;
            inner.to_gen = None;
            inner.from_gen = None;
            Ok(iter_result(Value::Undefined, true))
        }
    }
}

/// Build an iterator result object `{ value, done }`.
pub(crate) fn iter_result(value: Value, done: bool) -> Value {
    Value::object(vec![
        ("value".to_string(), value),
        ("done".to_string(), Value::Bool(done)),
    ])
}
