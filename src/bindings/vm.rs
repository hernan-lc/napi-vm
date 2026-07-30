//! The `#[napi]`-exported surface: the `VM` class and the free functions
//! (`createVm`, `runCode`, `debugParse`) that Node/Bun import from the
//! compiled addon.
use std::collections::HashMap;
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use napi::bindgen_prelude::Unknown;
use napi::{Env, JsValue, sys};
use napi_derive::napi;

use crate::error::VmErr;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::Value;

use super::bridge::{run_async_done_cb, NapiHostBridge};
use super::format::to_string;
use super::marshal::{chk, from_napi, make_str, to_napi, SendPtr};

pub fn run_source(source: &str, is_main: bool) -> Result<String, VmErr> {
    let mut interp = Interpreter::with_builtins();
    interp.is_main = is_main;
    interp.set_source(source);
    interp.begin_execution();
    let mut lex = Lexer::new(source);
    let toks = lex.tokenize_with_spans();
    let mut parser = Parser::new_with_spans(toks);
    let stmts = parser.parse();
    if parser.depth_exceeded {
        return Err(VmErr::Msg(
            "RangeError: Maximum parse depth exceeded".to_string(),
        ));
    }
    let val = interp.run(&stmts)?;
    Ok(to_string(&val))
}

#[napi]
pub struct VM {
    interp: Interpreter,
    modules: HashMap<String, String>,
    /// Lazily-attached bridge to Node.js functions exposed via
    /// `exposeFunction`. Also stored on the interpreter so calls made deep
    /// inside `run` can reach it.
    bridge: Option<Rc<NapiHostBridge>>,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl VM {
    #[napi(constructor)]
    pub fn new() -> Self {
        let interp = Interpreter::with_builtins();
        Self {
            interp,
            modules: HashMap::new(),
            bridge: None,
        }
    }

    /// Attach the host bridge on first use and return a handle to it.
    fn ensure_bridge(&mut self, env: Env) -> Rc<NapiHostBridge> {
        if let Some(b) = self.bridge.as_ref() {
            return b.clone();
        }
        let b = Rc::new(NapiHostBridge::new(env.raw()));
        self.interp.host = Some(b.clone());
        self.bridge = Some(b.clone());
        b
    }

    #[napi]
    pub fn run(&mut self, source: String) -> napi::Result<String> {
        self.interp.set_source(&source);
        self.interp.begin_execution();
        let mut lex = Lexer::new(&source);
        let toks = lex.tokenize_with_spans();
        let mut parser = Parser::new_with_spans(toks);
        let stmts = parser.parse();
        if parser.depth_exceeded {
            return Err(napi::Error::from_reason(
                "RangeError: Maximum parse depth exceeded",
            ));
        }
        let result = self.interp.run(&stmts);
        match result {
            Ok(v) => Ok(to_string(&v)),
            Err(e) => {
                let enriched = self.interp.enrich_error(e, None);
                Err(napi::Error::from_reason(enriched.to_string()))
            }
        }
    }

    #[napi]
    pub fn register_module(&mut self, name: String, source: String) -> napi::Result<()> {
        // Run the module on this VM's interpreter with `cur_mod` set so its
        // `export` statements populate `self.interp.modules[name]`, making them
        // visible to later `import` statements in the same VM. (Running it in a
        // throwaway interpreter, as before, discarded every export.)
        self.interp.cur_mod = Some(name.clone());
        self.interp.begin_execution();
        let mut lex = Lexer::new(&source);
        let toks = lex.tokenize_with_spans();
        let mut parser = Parser::new_with_spans(toks);
        let stmts = parser.parse();
        if parser.depth_exceeded {
            self.interp.cur_mod = None;
            return Err(napi::Error::from_reason(
                "RangeError: Maximum parse depth exceeded",
            ));
        }
        let result = self.interp.run(&stmts);
        self.interp.cur_mod = None;
        result.map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.modules.insert(name, source);
        Ok(())
    }

    #[napi]
    pub fn set_import_meta_main(&mut self, is_main: bool) {
        self.interp.is_main = is_main;
    }

    /// Cap the number of loop iterations a single execution may perform
    /// (default 100M). When the budget runs out, the VM throws a catchable
    /// `RangeError` instead of freezing the host event loop forever.
    #[napi]
    pub fn set_loop_limit(&mut self, n: u32) {
        self.interp.set_loop_budget(n as u64);
    }

    #[napi]
    pub fn get_global(&self, name: String) -> napi::Result<String> {
        match self.interp.global.borrow().get(&name) {
            Some(val) => Ok(to_string(&val)),
            None => Ok("undefined".to_string()),
        }
    }

    /// Define a global variable in the VM from a structured Node value. The
    /// value is reachable both as a bare identifier and (once the global
    /// aliases are wired) via `window`/`globalThis`/`self`.
    #[napi]
    pub fn set_global(&mut self, env: Env, name: String, value: Unknown) -> napi::Result<()> {
        let raw = value.raw();
        let v = from_napi(env.raw(), raw).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.interp.global.borrow_mut().set(&name, v);
        Ok(())
    }

    /// Expose a Node function to the VM as a global. VM code can then call it
    /// by name; arguments and the return value are marshalled across the
    /// boundary, and a thrown error propagates into the VM as a catchable
    /// exception.
    #[napi]
    pub fn expose_function(&mut self, env: Env, name: String, func: Unknown) -> napi::Result<()> {
        let raw = func.raw();
        let mut t: sys::napi_valuetype = 0;
        unsafe { sys::napi_typeof(env.raw(), raw, &mut t) };
        if t != sys::ValueType::napi_function {
            return Err(napi::Error::from_reason(format!(
                "exposeFunction: '{}' must be a function",
                name
            )));
        }
        let bridge = self.ensure_bridge(env);
        let id = bridge
            .register(raw)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let host_fn = Value::HostFunction {
            name: name.as_str().into(),
            id,
        };
        self.interp.global.borrow_mut().set(&name, host_fn);
        Ok(())
    }

    /// Expose an async Node function to the VM. Unlike `exposeFunction`, the
    /// function may return a Promise. VM code must `await` the call (use
    /// `runAsync` to execute code that awaits). The VM thread parks until the
    /// Promise settles on the Node event loop.
    #[napi]
    pub fn expose_async_function(
        &mut self,
        env: Env,
        name: String,
        func: Unknown,
    ) -> napi::Result<()> {
        let raw = func.raw();
        let mut t: sys::napi_valuetype = 0;
        unsafe { sys::napi_typeof(env.raw(), raw, &mut t) };
        if t != sys::ValueType::napi_function {
            return Err(napi::Error::from_reason(format!(
                "exposeAsyncFunction: '{}' must be a function",
                name
            )));
        }
        let bridge = self.ensure_bridge(env);
        let id = bridge
            .register_async(raw)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let host_fn = Value::HostFunction {
            name: name.as_str().into(),
            id,
        };
        self.interp.global.borrow_mut().set(&name, host_fn);
        Ok(())
    }

    /// Execute code that may `await` async host functions. Returns a Promise
    /// that resolves with the stringified result once the VM finishes.
    ///
    /// Internally, the VM runs on a dedicated thread so that `await` can park
    /// without blocking the Node event loop. Async host calls are dispatched
    /// back to the main thread via a ThreadsafeFunction.
    ///
    /// **Throughput caveat:** each call spawns a new OS thread. Under
    /// sustained high-frequency use (>100 calls/sec for extended periods),
    /// the thread spawn/cleanup cycle can exhaust native resources and crash
    /// the process. For high-frequency event handlers (chat messages, ticks),
    /// prefer the synchronous `run()` which completes in microseconds for
    /// typical workloads and never spawns a thread. Reserve `runAsync` for
    /// genuinely heavy or long-running computation where blocking the event
    /// loop is unacceptable.
    ///
    /// The caller must not call `run`/`runAsync` concurrently on the same VM.
    #[napi]
    pub fn run_async(&mut self, env: Env, source: String) -> napi::Result<Unknown<'_>> {
        let raw_env = env.raw();

        // Create a raw deferred (Promise) to return to the caller.
        let mut deferred: sys::napi_deferred = ptr::null_mut();
        let mut promise = ptr::null_mut();
        chk(unsafe { sys::napi_create_promise(raw_env, &mut deferred, &mut promise) })
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;

        // Create a completion TSFN: the VM thread calls this when done, and
        // the callback (on the main thread) resolves/rejects the deferred.
        // The deferred handle is passed as the TSFN context pointer.
        let mut done_tsfn: sys::napi_threadsafe_function = ptr::null_mut();
        let tsfn_name = make_str(raw_env, "vm-run-async-done")
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let status = unsafe {
            sys::napi_create_threadsafe_function(
                raw_env,
                ptr::null_mut(),
                ptr::null_mut(),
                tsfn_name,
                0,
                1,
                ptr::null_mut(),
                None,
                deferred as *mut std::ffi::c_void,
                Some(run_async_done_cb),
                &mut done_tsfn,
            )
        };
        if status != sys::Status::napi_ok {
            return Err(napi::Error::from_reason("failed to create completion TSFN"));
        }

        // Safety: the VM thread has exclusive access to the interpreter for
        // the duration of execution. The main thread only services async
        // bridge callbacks (which touch the TSFN + channels, not the
        // interpreter). The caller must not call run/runAsync concurrently.
        let interp_ptr = SendPtr(&mut self.interp as *mut Interpreter as usize);
        let tsfn_ptr = SendPtr(done_tsfn as usize);

        // Signal the bridge that host calls must go through the TSFN.
        // We share the atomic flag via a raw pointer (same SendPtr trick)
        // since Rc<NapiHostBridge> is not Send.
        let flag_ptr = if let Some(b) = self.bridge.as_ref() {
            b.on_vm_thread.store(1, Ordering::Release);
            SendPtr(&b.on_vm_thread as *const AtomicUsize as usize)
        } else {
            SendPtr(0usize)
        };

        let source_clone = source.clone();
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                let interp = unsafe { &mut *(interp_ptr.0 as *mut Interpreter) };
                interp.set_source(&source_clone);
                interp.begin_execution();
                let mut lex = Lexer::new(&source_clone);
                let toks = lex.tokenize_with_spans();
                let mut parser = Parser::new_with_spans(toks);
                let stmts = parser.parse();

                let result: Result<String, String> = if parser.depth_exceeded {
                    Err("RangeError: Maximum parse depth exceeded".to_string())
                } else {
                    match interp.run(&stmts) {
                        Ok(v) => {
                            // Unwrap settled promises: `runAsync` callers
                            // expect the resolved value, not `[object Promise]`.
                            match &v {
                                Value::Promise {
                                    state: crate::value::PromiseState::Fulfilled,
                                    value,
                                } => {
                                    let inner = value
                                        .as_ref()
                                        .map(|b| (**b).clone())
                                        .unwrap_or(Value::Undefined);
                                    Ok(to_string(&inner))
                                }
                                Value::Promise {
                                    state: crate::value::PromiseState::Rejected,
                                    value,
                                } => {
                                    let reason = value
                                        .as_ref()
                                        .map(|b| (**b).clone())
                                        .unwrap_or(Value::Undefined);
                                    Err(to_string(&reason))
                                }
                                _ => Ok(to_string(&v)),
                            }
                        }
                        Err(e) => Err(interp.enrich_error(e, None).to_string()),
                    }
                };

                // Clear the VM-thread flag so subsequent sync `run()` calls
                // use direct napi_env access again.
                if flag_ptr.0 != 0 {
                    unsafe { &*(flag_ptr.0 as *const AtomicUsize) }.store(0, Ordering::Release);
                }

                // Notify the main thread to resolve the deferred.
                let tsfn = tsfn_ptr.0 as sys::napi_threadsafe_function;
                let msg = Box::new(result);
                unsafe {
                    sys::napi_call_threadsafe_function(
                        tsfn,
                        Box::into_raw(msg) as *mut std::ffi::c_void,
                        sys::ThreadsafeFunctionCallMode::nonblocking,
                    );
                    sys::napi_release_threadsafe_function(
                        tsfn,
                        sys::ThreadsafeFunctionReleaseMode::release,
                    );
                }
            })
            .map_err(|e| napi::Error::from_reason(format!("failed to spawn VM thread: {}", e)))?;

        Ok(unsafe { Unknown::from_raw_unchecked(raw_env, promise) })
    }

    /// Remove a previously registered module so its exports are no longer
    /// importable. Essential for hot-reload: call this before re-registering
    /// a changed module to avoid stale export state.
    #[napi]
    pub fn remove_module(&mut self, name: String) -> bool {
        self.interp.modules.remove(&name).is_some()
    }

    /// Check whether a module with the given name is registered.
    #[napi]
    pub fn has_module(&self, name: String) -> bool {
        self.interp.modules.contains_key(&name)
    }

    /// Return the names of all registered modules.
    #[napi]
    pub fn list_modules(&self) -> Vec<String> {
        self.interp.modules.keys().cloned().collect()
    }

    /// Remove a global binding (including exposed host functions). Returns
    /// `true` if the binding existed. Use before re-exposing a function on
    /// hot-reload to avoid leaking stale references.
    #[napi]
    pub fn remove_global(&mut self, name: String) -> bool {
        self.interp.global.borrow_mut().remove(&name)
    }

    /// Check whether a global binding exists.
    #[napi]
    pub fn has_global(&self, name: String) -> bool {
        self.interp.global.borrow().has(&name)
    }

    /// Call a function defined in the VM (e.g. via a prior `run`) from Node,
    /// marshalling arguments in and the return value out.
    #[napi]
    pub fn call_function(
        &mut self,
        env: Env,
        name: String,
        args: Vec<Unknown>,
    ) -> napi::Result<Unknown<'_>> {
        let raw_env = env.raw();
        let mut vm_args = Vec::with_capacity(args.len());
        for a in &args {
            let raw = a.raw();
            vm_args.push(
                from_napi(raw_env, raw).map_err(|e| napi::Error::from_reason(e.to_string()))?,
            );
        }
        let callee = self.interp.global.borrow().get(&name).ok_or_else(|| {
            napi::Error::from_reason(format!("callFunction: '{}' is not defined", name))
        })?;
        self.interp.begin_execution();
        let result = self
            .interp
            .call_this(&callee, Value::Undefined, vm_args)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let out = to_napi(raw_env, &result).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(unsafe { Unknown::from_raw_unchecked(raw_env, out) })
    }
}

#[napi]
pub fn create_vm() -> VM {
    VM::new()
}

#[napi]
pub fn run_code(source: String) -> napi::Result<String> {
    run_source(&source, false).map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn debug_parse(source: String) -> napi::Result<String> {
    let mut lex = Lexer::new(&source);
    let toks = lex.tokenize_with_spans();
    let mut parser = Parser::new_with_spans(toks);
    let stmts = parser.parse();
    if parser.depth_exceeded {
        return Err(napi::Error::from_reason(
            "RangeError: Maximum parse depth exceeded",
        ));
    }
    Ok(format!("{:?}", stmts))
}
