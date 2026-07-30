//! Async host bridge infrastructure.
//!
//! Stores persisted references to Node.js functions and invokes them on
//! behalf of the VM, synchronously on the main thread or asynchronously
//! via a ThreadsafeFunction: async host functions (registered via
//! `exposeAsyncFunction`) are dispatched to the Node.js main thread, while
//! the VM thread parks on a channel until the main thread resolves the JS
//! Promise and sends the result back.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::os::raw::c_char;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use napi::sys;

use crate::error::VmErr;
use crate::host::HostBridge;
use crate::value::{SendValue, Value};

use super::format::to_string;
use super::marshal::{chk, from_napi, get_named_str, make_str, to_napi};

/// Message sent from the VM thread to the main thread via the TSFN.
/// Boxed and passed as the `data` pointer in `napi_call_threadsafe_function`.
struct AsyncCallMsg {
    /// Persisted reference to the JS async function.
    func_ref: sys::napi_ref,
    /// Marshalled VM arguments (wrapped for Send safety).
    args: Vec<SendValue>,
    /// Channel to send the resolved value back to the VM thread.
    reply_tx: mpsc::Sender<Result<SendValue, String>>,
}

/// Shared async state accessible from both the VM thread and the main thread.
struct AsyncState {
    /// ThreadsafeFunction handle for dispatching calls to the main thread.
    tsfn: sys::napi_threadsafe_function,
    /// Monotonic counter for pending async call ids.
    next_pending: AtomicUsize,
    /// Pending async calls: the VM thread stores a receiver here and blocks
    /// on it in `await_host`; the main thread sends the result through the
    /// corresponding sender (carried in `AsyncCallMsg`).
    pending: Mutex<HashMap<usize, mpsc::Receiver<Result<SendValue, String>>>>,
}

// Safety: `tsfn` is a raw pointer (hence `!Send + !Sync` by default), but
// `napi_threadsafe_function` is explicitly designed for cross-thread use —
// `napi_call_threadsafe_function` is thread-safe by contract. The remaining
// fields (`AtomicUsize`, `Mutex<..>`) are already `Send + Sync`.
unsafe impl Send for AsyncState {}
unsafe impl Sync for AsyncState {}

/// Bridge that stores persisted references to Node.js functions and invokes
/// them synchronously (or asynchronously) on behalf of the VM.
pub(super) struct NapiHostBridge {
    env: sys::napi_env,
    funcs: RefCell<HashMap<usize, sys::napi_ref>>,
    /// Functions registered via `exposeAsyncFunction`.
    async_funcs: RefCell<HashMap<usize, sys::napi_ref>>,
    next_id: Cell<usize>,
    /// Shared async dispatch state. `None` until the first async function is
    /// registered (at which point the TSFN is created).
    async_state: Mutex<Option<Arc<AsyncState>>>,
    /// Set to `true` while `runAsync` is executing on the VM thread. When
    /// true, `call_host` routes through the TSFN instead of touching
    /// `napi_env` directly (which is only valid on the main thread).
    pub(super) on_vm_thread: AtomicUsize,
}

impl NapiHostBridge {
    pub(super) fn new(env: sys::napi_env) -> Self {
        Self {
            env,
            funcs: RefCell::new(HashMap::new()),
            async_funcs: RefCell::new(HashMap::new()),
            next_id: Cell::new(0),
            async_state: Mutex::new(None),
            on_vm_thread: AtomicUsize::new(0),
        }
    }

    /// Persist a reference to a JS function and return the id the VM uses to
    /// call it back.
    pub(super) fn register(&self, func: sys::napi_value) -> Result<usize, VmErr> {
        let mut r: sys::napi_ref = ptr::null_mut();
        chk(unsafe { sys::napi_create_reference(self.env, func, 1, &mut r) })?;
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.funcs.borrow_mut().insert(id, r);
        Ok(id)
    }

    /// Register an async JS function. Creates the TSFN on first use.
    pub(super) fn register_async(&self, func: sys::napi_value) -> Result<usize, VmErr> {
        let mut r: sys::napi_ref = ptr::null_mut();
        chk(unsafe { sys::napi_create_reference(self.env, func, 1, &mut r) })?;
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.async_funcs.borrow_mut().insert(id, r);

        // Ensure the TSFN exists.
        let mut guard = self.async_state.lock().unwrap();
        if guard.is_none() {
            let tsfn = self.create_tsfn()?;
            *guard = Some(Arc::new(AsyncState {
                tsfn,
                next_pending: AtomicUsize::new(0),
                pending: Mutex::new(HashMap::new()),
            }));
        }
        Ok(id)
    }

    /// Create the ThreadsafeFunction used to dispatch async calls to the main
    /// thread. The callback (`tsfn_callback`) runs on the main thread with a
    /// valid `napi_env`.
    fn create_tsfn(&self) -> Result<sys::napi_threadsafe_function, VmErr> {
        let env = self.env;
        let mut tsfn: sys::napi_threadsafe_function = ptr::null_mut();
        let name = make_str(env, "vm-async-dispatch")?;
        chk(unsafe {
            sys::napi_create_threadsafe_function(
                env,
                ptr::null_mut(), // no JS callback function
                ptr::null_mut(), // no async_resource
                name,
                0,               // max_queue_size (0 = unlimited)
                1,               // initial_thread_count
                ptr::null_mut(), // thread_finalize_data
                None,            // thread_finalize_cb
                ptr::null_mut(), // context
                Some(tsfn_callback),
                &mut tsfn,
            )
        })?;
        Ok(tsfn)
    }

    /// Get a clone of the async state, if initialized.
    fn get_async_state(&self) -> Option<Arc<AsyncState>> {
        self.async_state.lock().unwrap().clone()
    }

    /// Dispatch a host function call through the TSFN and block until the
    /// main thread resolves it. Used when running on the VM thread (inside
    /// `runAsync`) where direct `napi_env` access is not possible.
    fn call_via_tsfn(
        &self,
        state: Arc<AsyncState>,
        id: usize,
        args: Vec<Value>,
        func_map: &RefCell<HashMap<usize, sys::napi_ref>>,
    ) -> Result<Value, VmErr> {
        let func_ref = *func_map
            .borrow()
            .get(&id)
            .ok_or_else(|| VmErr::Msg(format!("host function #{} is not registered", id)))?;

        let (reply_tx, reply_rx) = mpsc::channel::<Result<SendValue, String>>();
        let msg = Box::new(AsyncCallMsg {
            func_ref,
            args: args.into_iter().map(SendValue).collect(),
            reply_tx,
        });
        let status = unsafe {
            sys::napi_call_threadsafe_function(
                state.tsfn,
                Box::into_raw(msg) as *mut std::ffi::c_void,
                sys::ThreadsafeFunctionCallMode::nonblocking,
            )
        };
        if status != sys::Status::napi_ok {
            return Err(VmErr::Msg(format!(
                "failed to dispatch host call (status {})",
                status
            )));
        }
        match reply_rx.recv() {
            Ok(Ok(SendValue(v))) => Ok(v),
            Ok(Err(msg)) => Err(VmErr::Msg(format!("Error: {}", msg))),
            Err(_) => Err(VmErr::Msg("host call channel closed".to_string())),
        }
    }
}

impl HostBridge for NapiHostBridge {
    fn call_host(&self, id: usize, args: Vec<Value>) -> Result<Value, VmErr> {
        // When running on the VM thread (inside `runAsync`), we cannot touch
        // `napi_env` directly — it's bound to the main thread. Route through
        // the TSFN + channel instead.
        if self.on_vm_thread.load(Ordering::Acquire) != 0
            && let Some(state) = self.get_async_state()
        {
            return self.call_via_tsfn(state, id, args, &self.funcs);
        }

        let env = self.env;
        let func_ref = *self
            .funcs
            .borrow()
            .get(&id)
            .ok_or_else(|| VmErr::Msg(format!("host function #{} is not registered", id)))?;
        let mut func = ptr::null_mut();
        chk(unsafe { sys::napi_get_reference_value(env, func_ref, &mut func) })?;

        let mut argv = Vec::with_capacity(args.len());
        for a in &args {
            argv.push(to_napi(env, a)?);
        }
        let mut recv = ptr::null_mut();
        chk(unsafe { sys::napi_get_global(env, &mut recv) })?;

        let mut result = ptr::null_mut();
        let status = unsafe {
            sys::napi_call_function(env, recv, func, argv.len(), argv.as_ptr(), &mut result)
        };
        if status != sys::Status::napi_ok {
            let mut exc = ptr::null_mut();
            unsafe { sys::napi_get_and_clear_last_exception(env, &mut exc) };
            if !exc.is_null() {
                return Err(VmErr::Throw(from_napi(env, exc)?));
            }
            return Err(VmErr::Msg(format!(
                "host function call failed (status {})",
                status
            )));
        }
        from_napi(env, result)
    }

    fn is_async_fn(&self, id: usize) -> bool {
        self.async_funcs.borrow().contains_key(&id)
    }

    fn call_host_async(&self, id: usize, args: Vec<Value>) -> Result<Value, VmErr> {
        let state = self
            .get_async_state()
            .ok_or_else(|| VmErr::Msg("async bridge not initialized".to_string()))?;
        let func_ref = *self
            .async_funcs
            .borrow()
            .get(&id)
            .ok_or_else(|| VmErr::Msg(format!("async host function #{} not registered", id)))?;

        // Create a channel for the result.
        let (reply_tx, reply_rx) = mpsc::channel::<Result<SendValue, String>>();

        // Assign a pending id and store the receiver.
        let pending_id = state.next_pending.fetch_add(1, Ordering::SeqCst);
        state.pending.lock().unwrap().insert(pending_id, reply_rx);

        // Package the call and dispatch to the main thread via TSFN.
        let msg = Box::new(AsyncCallMsg {
            func_ref,
            args: args.into_iter().map(SendValue).collect(),
            reply_tx,
        });
        let status = unsafe {
            sys::napi_call_threadsafe_function(
                state.tsfn,
                Box::into_raw(msg) as *mut std::ffi::c_void,
                sys::ThreadsafeFunctionCallMode::nonblocking,
            )
        };
        if status != sys::Status::napi_ok {
            state.pending.lock().unwrap().remove(&pending_id);
            return Err(VmErr::Msg(format!(
                "failed to dispatch async call (status {})",
                status
            )));
        }

        Ok(Value::HostPending { id: pending_id })
    }

    fn await_host(&self, pending_id: usize) -> Result<Value, VmErr> {
        let state = self
            .get_async_state()
            .ok_or_else(|| VmErr::Msg("async bridge not initialized".to_string()))?;
        // Take the receiver out of the pending map and block on it.
        let rx = state
            .pending
            .lock()
            .unwrap()
            .remove(&pending_id)
            .ok_or_else(|| VmErr::Msg(format!("no pending async call #{}", pending_id)))?;
        match rx.recv() {
            Ok(Ok(SendValue(v))) => Ok(v),
            Ok(Err(msg)) => Err(VmErr::Msg(format!("Error: {}", msg))),
            Err(_) => Err(VmErr::Msg("async host call channel closed".to_string())),
        }
    }
}

/// TSFN callback — runs on the **main thread** with a valid `env`.
/// Receives an `AsyncCallMsg`, calls the JS async function, and wires the
/// resulting Promise's settlement back through the reply channel.
extern "C" fn tsfn_callback(
    env: sys::napi_env,
    _js_callback: sys::napi_value,
    _context: *mut std::ffi::c_void,
    data: *mut std::ffi::c_void,
) {
    if data.is_null() {
        return;
    }
    let msg = unsafe { Box::from_raw(data as *mut AsyncCallMsg) };

    // Resolve the function reference.
    let mut func = ptr::null_mut();
    let status = unsafe { sys::napi_get_reference_value(env, msg.func_ref, &mut func) };
    if status != sys::Status::napi_ok || func.is_null() {
        let _ = msg
            .reply_tx
            .send(Err("failed to resolve async function ref".into()));
        return;
    }

    // Marshal VM args → napi values.
    let mut argv = Vec::with_capacity(msg.args.len());
    for sv in &msg.args {
        match to_napi(env, &sv.0) {
            Ok(v) => argv.push(v),
            Err(e) => {
                let _ = msg.reply_tx.send(Err(format!("marshal error: {}", e)));
                return;
            }
        }
    }

    // Call the async function.
    let mut recv = ptr::null_mut();
    unsafe { sys::napi_get_global(env, &mut recv) };
    let mut result = ptr::null_mut();
    let status =
        unsafe { sys::napi_call_function(env, recv, func, argv.len(), argv.as_ptr(), &mut result) };
    if status != sys::Status::napi_ok {
        let mut exc = ptr::null_mut();
        unsafe { sys::napi_get_and_clear_last_exception(env, &mut exc) };
        let err_msg = if !exc.is_null() {
            match from_napi(env, exc) {
                Ok(Value::String(ref s)) => s.clone(),
                Ok(ref v) => to_string(v),
                Err(_) => "unknown error".into(),
            }
        } else {
            format!("async function call failed (status {})", status)
        };
        let _ = msg.reply_tx.send(Err(err_msg));
        return;
    }

    // The result should be a Promise (thenable). Attach .then/.catch to
    // capture settlement.
    let reply_tx = msg.reply_tx;

    // Check if result is a Promise (has a .then method).
    let mut then_val = ptr::null_mut();
    let then_key = unsafe {
        let mut k = ptr::null_mut();
        sys::napi_create_string_utf8(env, c"then".as_ptr(), 4, &mut k);
        k
    };
    unsafe { sys::napi_get_property(env, result, then_key, &mut then_val) };

    let mut then_type: sys::napi_valuetype = 0;
    unsafe { sys::napi_typeof(env, then_val, &mut then_type) };

    if then_type != sys::ValueType::napi_function {
        // Not a thenable — resolve immediately with the value.
        match from_napi(env, result) {
            Ok(v) => {
                let _ = reply_tx.send(Ok(SendValue(v)));
            }
            Err(e) => {
                let _ = reply_tx.send(Err(e.to_string()));
            }
        }
        return;
    }

    // Create resolve/reject callbacks. Each gets a clone of the sender so
    // exactly one settlement path delivers the result. Resolve sends
    // `Ok(value)`, reject sends `Err(message)` so the VM can re-throw.
    let resolve_tx = Box::into_raw(Box::new(reply_tx.clone()));
    let reject_tx = Box::into_raw(Box::new(reply_tx));

    let mut resolve_fn = ptr::null_mut();
    unsafe {
        sys::napi_create_function(
            env,
            c"resolve".as_ptr(),
            7,
            Some(promise_resolve_cb),
            resolve_tx as *mut std::ffi::c_void,
            &mut resolve_fn,
        );
    }

    let mut reject_fn = ptr::null_mut();
    unsafe {
        sys::napi_create_function(
            env,
            c"reject".as_ptr(),
            6,
            Some(promise_reject_cb),
            reject_tx as *mut std::ffi::c_void,
            &mut reject_fn,
        );
    }

    // Call promise.then(resolve, reject).
    let mut then_argv = [resolve_fn, reject_fn];
    let mut _then_result = ptr::null_mut();
    unsafe {
        sys::napi_call_function(
            env,
            result, // this = the promise
            then_val,
            2,
            then_argv.as_mut_ptr(),
            &mut _then_result,
        );
    }
}

/// Promise resolve callback. Marshals the fulfilled value and sends
/// `Ok(value)` to the VM thread.
extern "C" fn promise_resolve_cb(
    env: sys::napi_env,
    info: sys::napi_callback_info,
) -> sys::napi_value {
    let mut argc: usize = 1;
    let mut argv = [ptr::null_mut(); 1];
    let mut data = ptr::null_mut();
    unsafe {
        sys::napi_get_cb_info(
            env,
            info,
            &mut argc,
            argv.as_mut_ptr(),
            ptr::null_mut(),
            &mut data,
        );
    }
    if data.is_null() {
        return ptr::null_mut();
    }
    let reply_tx = unsafe { Box::from_raw(data as *mut mpsc::Sender<Result<SendValue, String>>) };

    let value = if argc > 0 && !argv[0].is_null() {
        match from_napi(env, argv[0]) {
            Ok(v) => v,
            Err(e) => {
                let _ = reply_tx.send(Err(format!("marshal error in resolve: {}", e)));
                return ptr::null_mut();
            }
        }
    } else {
        Value::Undefined
    };

    let _ = reply_tx.send(Ok(SendValue(value)));
    ptr::null_mut()
}

/// Promise reject callback. Extracts the error message from the rejection
/// reason and sends `Err(message)` to the VM thread, which re-throws it.
extern "C" fn promise_reject_cb(
    env: sys::napi_env,
    info: sys::napi_callback_info,
) -> sys::napi_value {
    let mut argc: usize = 1;
    let mut argv = [ptr::null_mut(); 1];
    let mut data = ptr::null_mut();
    unsafe {
        sys::napi_get_cb_info(
            env,
            info,
            &mut argc,
            argv.as_mut_ptr(),
            ptr::null_mut(),
            &mut data,
        );
    }
    if data.is_null() {
        return ptr::null_mut();
    }
    let reply_tx = unsafe { Box::from_raw(data as *mut mpsc::Sender<Result<SendValue, String>>) };

    // Extract a useful error message from the rejection reason.
    let msg = if argc > 0 && !argv[0].is_null() {
        // Try to get .message property (Error objects).
        let message = get_named_str(env, argv[0], "message").unwrap_or_default();
        if !message.is_empty() {
            message
        } else {
            match from_napi(env, argv[0]) {
                Ok(ref v) => to_string(v),
                Err(_) => "unknown rejection".into(),
            }
        }
    } else {
        "unknown rejection".into()
    };

    let _ = reply_tx.send(Err(msg));
    ptr::null_mut()
}

/// Completion callback for `run_async`. Runs on the main thread when the VM
/// thread finishes. Resolves or rejects the deferred (Promise) returned to
/// the caller. The `context` pointer is a raw `napi_deferred`.
pub(super) extern "C" fn run_async_done_cb(
    env: sys::napi_env,
    _js_callback: sys::napi_value,
    context: *mut std::ffi::c_void,
    data: *mut std::ffi::c_void,
) {
    if data.is_null() || context.is_null() {
        return;
    }
    let result = unsafe { *Box::from_raw(data as *mut Result<String, String>) };
    let deferred = context as sys::napi_deferred;

    unsafe {
        match result {
            Ok(val) => {
                let mut js_str = ptr::null_mut();
                sys::napi_create_string_utf8(
                    env,
                    val.as_ptr() as *const c_char,
                    val.len() as isize,
                    &mut js_str,
                );
                sys::napi_resolve_deferred(env, deferred, js_str);
            }
            Err(msg) => {
                let mut js_err = ptr::null_mut();
                let mut js_msg = ptr::null_mut();
                sys::napi_create_string_utf8(
                    env,
                    msg.as_ptr() as *const c_char,
                    msg.len() as isize,
                    &mut js_msg,
                );
                sys::napi_create_error(env, ptr::null_mut(), js_msg, &mut js_err);
                sys::napi_reject_deferred(env, deferred, js_err);
            }
        }
    }
}
