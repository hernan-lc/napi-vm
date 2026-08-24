//! The `#[napi]` VM surface and its single-owner execution gate.
//!
//! The interpreter remains deliberately single-threaded (`Rc`/`RefCell`). A
//! `VM` owns it behind `RuntimeCell`, whose mutex is the only way either the
//! Node thread or a `runAsync` worker can access it. The worker captures an
//! `Arc<VMState>`, never a raw pointer into the N-API object, so dropping the
//! JavaScript wrapper cannot leave a use-after-free behind.

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::Unknown;
use napi::{Env, JsValue, sys};
use napi_derive::napi;

use crate::error::VmErr;
use crate::format::to_string;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::{PromiseState, Value};

use super::bridge::{NapiHostBridge, run_async_done_cb};
use super::marshal::{chk, from_napi, make_str, to_napi};

pub fn run_source(source: &str, is_main: bool) -> Result<String, VmErr> {
    let mut interp = Interpreter::with_builtins();
    interp.is_main = is_main;
    execute_source(&mut interp, source).map(|value| to_string(&value))
}

/// All interpreter and bridge state lives here. It is never cloned or exposed
/// independently of the runtime gate.
struct VmRuntime {
    interp: Interpreter,
    modules: HashMap<String, String>,
    bridge: Option<std::rc::Rc<NapiHostBridge>>,
}

/// A mutex-backed owner for a non-`Send` interpreter.
///
/// `Rc` values are safe to use here because the `UnsafeCell` is accessed only
/// while `gate` is held. No `Rc` from `runtime` is stored outside this cell;
/// the worker owns an `Arc<RuntimeCell>`, and all Node entry points reject a
/// busy VM before trying to access it. This is the narrow ownership boundary
/// that replaces the previous raw `*mut Interpreter` transfer.
struct RuntimeCell {
    gate: Mutex<()>,
    runtime: UnsafeCell<VmRuntime>,
}

// SAFETY: `runtime` is never accessed without locking `gate`. The `VMState`
// owns the only `Arc` to this cell used by the worker, and all public methods
// use the same lock. The N-API bridge's cross-thread data is a separate
// `Arc<BridgeState>` containing only integer handles and synchronized maps.
unsafe impl Send for RuntimeCell {}
unsafe impl Sync for RuntimeCell {}

impl RuntimeCell {
    fn new(runtime: VmRuntime) -> Self {
        Self {
            gate: Mutex::new(()),
            runtime: UnsafeCell::new(runtime),
        }
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut VmRuntime) -> R) -> R {
        let _guard = self
            .gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: the mutex guard above excludes every other access.
        unsafe { f(&mut *self.runtime.get()) }
    }
}

struct VMState {
    runtime: RuntimeCell,
    busy: Arc<AtomicBool>,
    /// Kept outside `RuntimeCell` so `VM::drop` can release N-API resources
    /// without waiting for a worker that may currently be awaiting Node.
    bridge_state: Mutex<Option<Arc<super::bridge::BridgeState>>>,
}

impl VMState {
    fn try_start(&self) -> napi::Result<BusyGuard> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| BusyGuard {
                busy: self.busy.clone(),
            })
            .map_err(|_| napi::Error::from_reason("VM is busy with another execution"))
    }
}

struct BusyGuard {
    busy: Arc<AtomicBool>,
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}

#[napi]
pub struct VM {
    state: Arc<VMState>,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    fn new_state() -> Arc<VMState> {
        Arc::new(VMState {
            runtime: RuntimeCell::new(VmRuntime {
                interp: Interpreter::with_builtins(),
                modules: HashMap::new(),
                bridge: None,
            }),
            busy: Arc::new(AtomicBool::new(false)),
            bridge_state: Mutex::new(None),
        })
    }

    /// Attach the host bridge on first use. Must be called while the runtime
    /// gate is held on Node's main thread.
    fn ensure_bridge(
        state: &Arc<VMState>,
        runtime: &mut VmRuntime,
        env: Env,
    ) -> Result<std::rc::Rc<NapiHostBridge>, VmErr> {
        if let Some(bridge) = runtime.bridge.as_ref() {
            return Ok(bridge.clone());
        }
        let bridge = std::rc::Rc::new(NapiHostBridge::new(env.raw()));
        runtime.interp.host = Some(bridge.clone());
        *state
            .bridge_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(bridge.shared_state());
        runtime.bridge = Some(bridge.clone());
        Ok(bridge)
    }

    fn current_bridge(runtime: &VmRuntime) -> Option<std::rc::Rc<NapiHostBridge>> {
        runtime.bridge.clone()
    }
}

impl Drop for VM {
    fn drop(&mut self) {
        // This runs on Node's main thread. It deliberately does not lock the
        // runtime: an async worker may be parked waiting for a Promise, and
        // waiting here would deadlock the Node event loop. The shared bridge
        // marks handles retired and releases its initial TSFN reference; any
        // in-flight callback owns the remaining lease until it finishes.
        if let Some(bridge) = self
            .state
            .bridge_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .cloned()
        {
            bridge.shutdown_on_main();
        }
    }
}

#[napi]
impl VM {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            state: Self::new_state(),
        }
    }

    #[napi]
    pub fn run(&mut self, source: String) -> napi::Result<String> {
        let _busy = self.state.try_start()?;
        let state = self.state.clone();
        state.runtime.with_mut(|runtime| {
            execute_source(&mut runtime.interp, &source)
                .map(|value| to_string(&value))
                .map_err(|error| {
                    napi::Error::from_reason(runtime.interp.enrich_error(error, None).to_string())
                })
        })
    }

    #[napi]
    pub fn register_module(&mut self, name: String, source: String) -> napi::Result<()> {
        let _busy = self.state.try_start()?;
        self.state.runtime.with_mut(|runtime| {
            runtime.interp.cur_mod = Some(name.clone());
            let result = execute_source(&mut runtime.interp, &source);
            runtime.interp.cur_mod = None;
            result
                .map(|_| ())
                .map_err(|error| napi::Error::from_reason(error.to_string()))?;
            runtime.modules.insert(name, source);
            Ok(())
        })
    }

    #[napi]
    pub fn set_import_meta_main(&mut self, is_main: bool) -> napi::Result<()> {
        let _busy = self.state.try_start()?;
        self.state.runtime.with_mut(|runtime| {
            runtime.interp.is_main = is_main;
        });
        Ok(())
    }

    /// Cap the number of loop iterations in a single execution.
    #[napi]
    pub fn set_loop_limit(&mut self, n: u32) -> napi::Result<()> {
        let _busy = self.state.try_start()?;
        self.state
            .runtime
            .with_mut(|runtime| runtime.interp.set_loop_budget(n as u64));
        Ok(())
    }

    #[napi]
    pub fn get_global(&self, name: String) -> napi::Result<String> {
        let _busy = self.state.try_start()?;
        Ok(self.state.runtime.with_mut(|runtime| {
            runtime
                .interp
                .global
                .borrow()
                .get(&name)
                .map(|value| to_string(&value))
                .unwrap_or_else(|| "undefined".to_string())
        }))
    }

    #[napi]
    pub fn set_global(&mut self, env: Env, name: String, value: Unknown) -> napi::Result<()> {
        let _busy = self.state.try_start()?;
        let value = from_napi(env.raw(), value.raw())
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;
        self.state.runtime.with_mut(|runtime| {
            if let Some(Value::HostFunction { id, .. }) = runtime.interp.global.borrow().get(&name)
                && let Some(bridge) = Self::current_bridge(runtime)
            {
                bridge.unregister(id);
            }
            runtime.interp.global.borrow_mut().set(&name, value);
        });
        Ok(())
    }

    #[napi]
    pub fn expose_function(&mut self, env: Env, name: String, func: Unknown) -> napi::Result<()> {
        self.expose_function_inner(env, name, func, false)
    }

    #[napi]
    pub fn expose_async_function(
        &mut self,
        env: Env,
        name: String,
        func: Unknown,
    ) -> napi::Result<()> {
        self.expose_function_inner(env, name, func, true)
    }

    fn expose_function_inner(
        &mut self,
        env: Env,
        name: String,
        func: Unknown,
        async_fn: bool,
    ) -> napi::Result<()> {
        let _busy = self.state.try_start()?;
        let raw = func.raw();
        let mut value_type: sys::napi_valuetype = 0;
        chk(unsafe { sys::napi_typeof(env.raw(), raw, &mut value_type) })
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;
        if value_type != sys::ValueType::napi_function {
            return Err(napi::Error::from_reason(format!(
                "{}: '{}' must be a function",
                if async_fn {
                    "exposeAsyncFunction"
                } else {
                    "exposeFunction"
                },
                name
            )));
        }

        let state = self.state.clone();
        state.runtime.with_mut(|runtime| {
            if let Some(Value::HostFunction { id, .. }) = runtime.interp.global.borrow().get(&name)
                && let Some(bridge) = Self::current_bridge(runtime)
            {
                bridge.unregister(id);
            }
            let bridge = Self::ensure_bridge(&state, runtime, env)
                .map_err(|error| napi::Error::from_reason(error.to_string()))?;
            let id = if async_fn {
                bridge.register_async(raw)
            } else {
                bridge.register(raw)
            }
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;
            runtime.interp.global.borrow_mut().set(
                &name,
                Value::HostFunction {
                    name: name.as_str().into(),
                    id,
                },
            );
            Ok(())
        })
    }

    /// Execute code that may await host functions. The worker captures only
    /// the `Arc<VMState>`; the interpreter itself remains under `RuntimeCell`'s
    /// mutex and is never accessed concurrently with a Node method.
    #[napi(ts_return_type = "Promise<string>")]
    pub fn run_async(&mut self, env: Env, source: String) -> napi::Result<Unknown<'_>> {
        let busy = self.state.try_start()?;
        let raw_env = env.raw();

        let mut deferred: sys::napi_deferred = ptr::null_mut();
        let mut promise = ptr::null_mut();
        chk(unsafe { sys::napi_create_promise(raw_env, &mut deferred, &mut promise) })
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;

        let mut done_tsfn: sys::napi_threadsafe_function = ptr::null_mut();
        let tsfn_name = make_str(raw_env, "vm-run-async-done")
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;
        chk(unsafe {
            sys::napi_create_threadsafe_function(
                raw_env,
                ptr::null_mut(),
                ptr::null_mut(),
                tsfn_name,
                1,
                1,
                ptr::null_mut(),
                None,
                deferred as *mut std::ffi::c_void,
                Some(run_async_done_cb),
                &mut done_tsfn,
            )
        })
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
        // Keep the completion TSFN referenced until the worker releases it.
        // `runAsync` must keep Node alive even when the guest performs no
        // host I/O; unref'ing this handle here lets a process exit before its
        // returned Promise is settled.

        let state = self.state.clone();
        let prepared = state.runtime.with_mut(|runtime| {
            if let Some(bridge) = runtime.bridge.as_ref() {
                bridge
                    .prepare_for_async()
                    .map_err(|error| napi::Error::from_reason(error.to_string()))?;
                bridge.on_vm_thread.store(1, Ordering::Release);
            }
            Ok::<(), napi::Error>(())
        });
        if let Err(error) = prepared {
            reject_deferred_now(raw_env, deferred, error.to_string());
            let _ = unsafe {
                sys::napi_release_threadsafe_function(
                    done_tsfn,
                    sys::ThreadsafeFunctionReleaseMode::release,
                )
            };
            return Err(error);
        }

        let done_handle = done_tsfn as usize;
        let source_for_worker = source.clone();
        let worker_state = state.clone();
        let spawn = std::thread::Builder::new()
            .name("napi-vm-async".into())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let _busy = busy;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    worker_state.runtime.with_mut(|runtime| {
                        match execute_source(&mut runtime.interp, &source_for_worker) {
                            Ok(value) => async_result_string(value),
                            Err(error) => Err(runtime.interp.enrich_error(error, None).to_string()),
                        }
                    })
                }))
                .unwrap_or_else(|_| Err("Error: VM execution panicked".to_string()));

                worker_state.runtime.with_mut(|runtime| {
                    if let Some(bridge) = runtime.bridge.as_ref() {
                        bridge.finish_async_worker();
                    }
                });

                let message = Box::new(result);
                let raw_message = Box::into_raw(message) as *mut std::ffi::c_void;
                let tsfn = done_handle as sys::napi_threadsafe_function;
                let status = unsafe {
                    sys::napi_call_threadsafe_function(
                        tsfn,
                        raw_message,
                        sys::ThreadsafeFunctionCallMode::nonblocking,
                    )
                };
                if status != sys::Status::napi_ok {
                    drop(unsafe { Box::from_raw(raw_message as *mut Result<String, String>) });
                }
                let release_status = unsafe {
                    sys::napi_release_threadsafe_function(
                        tsfn,
                        sys::ThreadsafeFunctionReleaseMode::release,
                    )
                };
                if release_status != sys::Status::napi_ok {
                    // The environment is already closing; the returned
                    // Promise cannot be observed after teardown.
                }
            });

        if let Err(error) = spawn {
            state.runtime.with_mut(|runtime| {
                if let Some(bridge) = runtime.bridge.as_ref() {
                    bridge.finish_async_worker();
                }
            });
            reject_deferred_now(
                raw_env,
                deferred,
                format!("failed to spawn VM thread: {}", error),
            );
            let _ = unsafe {
                sys::napi_release_threadsafe_function(
                    done_tsfn,
                    sys::ThreadsafeFunctionReleaseMode::release,
                )
            };
            return Err(napi::Error::from_reason(error.to_string()));
        }

        Ok(unsafe { Unknown::from_raw_unchecked(raw_env, promise) })
    }

    #[napi]
    pub fn remove_module(&mut self, name: String) -> napi::Result<bool> {
        let _busy = self.state.try_start()?;
        Ok(self
            .state
            .runtime
            .with_mut(|runtime| runtime.modules.remove(&name).is_some()))
    }

    #[napi]
    pub fn has_module(&self, name: String) -> napi::Result<bool> {
        let _busy = self.state.try_start()?;
        Ok(self
            .state
            .runtime
            .with_mut(|runtime| runtime.modules.contains_key(&name)))
    }

    #[napi]
    pub fn list_modules(&self) -> napi::Result<Vec<String>> {
        let _busy = self.state.try_start()?;
        Ok(self
            .state
            .runtime
            .with_mut(|runtime| runtime.modules.keys().cloned().collect()))
    }

    #[napi]
    pub fn remove_global(&mut self, name: String) -> napi::Result<bool> {
        let _busy = self.state.try_start()?;
        Ok(self.state.runtime.with_mut(|runtime| {
            let old = runtime.interp.global.borrow().get(&name);
            let removed = runtime.interp.global.borrow_mut().remove(&name);
            if removed
                && let Some(Value::HostFunction { id, .. }) = old
                && let Some(bridge) = Self::current_bridge(runtime)
            {
                bridge.unregister(id);
            }
            removed
        }))
    }

    #[napi]
    pub fn has_global(&self, name: String) -> napi::Result<bool> {
        let _busy = self.state.try_start()?;
        Ok(self
            .state
            .runtime
            .with_mut(|runtime| runtime.interp.global.borrow().has(&name)))
    }

    #[napi]
    pub fn call_function(
        &mut self,
        env: Env,
        name: String,
        args: Vec<Unknown>,
    ) -> napi::Result<Unknown<'_>> {
        let _busy = self.state.try_start()?;
        let raw_env = env.raw();
        if args.len() > crate::value::MAX_ARRAY_LEN {
            return Err(napi::Error::from_reason(
                "RangeError: Maximum argument count exceeded",
            ));
        }
        let mut vm_args = Vec::with_capacity(args.len());
        for arg in &args {
            vm_args.push(
                from_napi(raw_env, arg.raw())
                    .map_err(|error| napi::Error::from_reason(error.to_string()))?,
            );
        }
        self.state.runtime.with_mut(|runtime| {
            let callee = runtime.interp.global.borrow().get(&name).ok_or_else(|| {
                napi::Error::from_reason(format!("callFunction: '{}' is not defined", name))
            })?;
            runtime.interp.begin_execution();
            let result = runtime
                .interp
                .call_this(&callee, Value::Undefined, vm_args)
                .map_err(|error| napi::Error::from_reason(error.to_string()))?;
            let out = to_napi(raw_env, &result)
                .map_err(|error| napi::Error::from_reason(error.to_string()))?;
            Ok(unsafe { Unknown::from_raw_unchecked(raw_env, out) })
        })
    }
}

fn execute_source(interp: &mut Interpreter, source: &str) -> Result<Value, VmErr> {
    interp.set_source(source);
    interp.begin_execution();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_spans();
    let mut parser = Parser::new_with_spans(tokens);
    let statements = parser.parse();
    if parser.depth_exceeded {
        return Err(VmErr::Msg(
            "RangeError: Maximum parse depth exceeded".to_string(),
        ));
    }
    interp.run(&statements)
}

fn async_result_string(value: Value) -> Result<String, String> {
    match &value {
        Value::Promise {
            state: PromiseState::Fulfilled,
            value,
        } => Ok(value
            .as_ref()
            .map(|value| to_string(value))
            .unwrap_or_else(|| "undefined".to_string())),
        Value::Promise {
            state: PromiseState::Rejected,
            value,
        } => Err(value
            .as_ref()
            .map(|value| to_string(value))
            .unwrap_or_else(|| "undefined".to_string())),
        _ => Ok(to_string(&value)),
    }
}

fn reject_deferred_now(env: sys::napi_env, deferred: sys::napi_deferred, message: String) {
    let Ok(js_message) = make_str(env, &message) else {
        return;
    };
    let mut js_error = ptr::null_mut();
    let error_status =
        unsafe { sys::napi_create_error(env, ptr::null_mut(), js_message, &mut js_error) };
    if error_status == sys::Status::napi_ok {
        let reject_status = unsafe { sys::napi_reject_deferred(env, deferred, js_error) };
        if reject_status != sys::Status::napi_ok {
            // The environment may already be closing; there is no safe
            // follow-up operation for this deferred.
        }
    }
}

#[napi]
pub fn create_vm() -> VM {
    VM::new()
}

#[napi]
pub fn run_code(source: String) -> napi::Result<String> {
    run_source(&source, false).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn debug_parse(source: String) -> napi::Result<String> {
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize_with_spans();
    let mut parser = Parser::new_with_spans(tokens);
    let statements = parser.parse();
    if parser.depth_exceeded {
        return Err(napi::Error::from_reason(
            "RangeError: Maximum parse depth exceeded",
        ));
    }
    Ok(format!("{:?}", statements))
}
