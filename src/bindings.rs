use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::Unknown;
use napi::{Env, JsValue, sys};
use napi_derive::napi;

use crate::error::VmErr;
use crate::host::HostBridge;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::{SendValue, Value};

/// Maximum nesting `to_string` renders before abbreviating. Together with
/// the visited set this makes stringifying any guest structure total:
/// cyclic values print `[Circular]` and very deep ones print `[Object]` /
/// `[Array]` instead of overflowing the native stack.
const MAX_PRINT_DEPTH: usize = 128;

pub fn to_string(val: &Value) -> String {
    let mut visited: std::collections::HashSet<*const ()> = std::collections::HashSet::new();
    fn vs(v: &Value, visited: &mut std::collections::HashSet<*const ()>, depth: usize) -> String {
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
            Value::Object { props, .. } => {
                if depth >= MAX_PRINT_DEPTH {
                    return "[Object]".to_string();
                }
                let ptr = Rc::as_ptr(props) as *const ();
                if !visited.insert(ptr) {
                    return "[Circular]".to_string();
                }
                let s = format!(
                    "{{{}}}",
                    props
                        .borrow()
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, vs(v, visited, depth + 1)))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                visited.remove(&ptr);
                s
            }
            Value::Array(i) => {
                if depth >= MAX_PRINT_DEPTH {
                    return "[Array]".to_string();
                }
                let ptr = Rc::as_ptr(i) as *const ();
                if !visited.insert(ptr) {
                    return "[Circular]".to_string();
                }
                let s = format!(
                    "[{}]",
                    i.borrow()
                        .iter()
                        .map(|v| vs(v, visited, depth + 1))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                visited.remove(&ptr);
                s
            }
            Value::Function(f) => {
                format!("[Function: {}]", f.name.as_deref().unwrap_or("anonymous"))
            }
            Value::NativeFunction { name, .. } => format!("[Function: {} [native]]", name),
            Value::HostFunction { name, .. } => format!("[Function: {} [native]]", name),
            Value::GlobalObject => "[object global]".to_string(),
            Value::Class(c) => format!("[class {}]", c.name),
            Value::Promise { .. } | Value::HostPending { .. } => "[object Promise]".to_string(),
            Value::Generator { .. } => "[object Generator]".to_string(),
            Value::Symbol(s) => format!("Symbol({})", s),
            Value::Error(e) => e.message.clone(),
        }
    }
    vs(val, &mut visited, 0)
}

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

// ---------------------------------------------------------------------------
// Structured marshalling + host bridge
//
// The free functions above are string-only (`runCode`/`getGlobal` return
// strings). The bridge below passes *real* structured values across the
// boundary so Node can expose functions to the VM and call VM functions with
// live arguments. It is built directly on the stable raw `napi_sys` ABI: the
// VM is single-threaded, so a persisted `napi_ref` to a JavaScript function
// can be stored and invoked synchronously on the same thread that drives the
// interpreter.
//
// Each marshalling helper is a *safe* function that confines all FFI to a
// single `unsafe` block around its body; the `env` handle always originates
// from a live N-API callback, so the operations are sound.
// ---------------------------------------------------------------------------

#[inline]
fn chk(status: sys::napi_status) -> Result<(), VmErr> {
    if status == sys::Status::napi_ok {
        Ok(())
    } else {
        Err(VmErr::Msg(format!("napi call failed (status {})", status)))
    }
}

/// Create a JS string from a Rust `&str`.
fn make_str(env: sys::napi_env, s: &str) -> Result<sys::napi_value, VmErr> {
    unsafe {
        let mut out = ptr::null_mut();
        chk(sys::napi_create_string_utf8(
            env,
            s.as_ptr() as *const c_char,
            s.len() as isize,
            &mut out,
        ))?;
        Ok(out)
    }
}

/// Set a string-valued named property on an object.
fn set_str_prop(
    env: sys::napi_env,
    obj: sys::napi_value,
    key: &str,
    val: &str,
) -> Result<(), VmErr> {
    unsafe {
        let sv = make_str(env, val)?;
        let ck =
            CString::new(key).map_err(|_| VmErr::Msg("object key contains NUL".to_string()))?;
        chk(sys::napi_set_named_property(env, obj, ck.as_ptr(), sv))
    }
}

/// Maximum nesting marshalled across the NAPI boundary in either direction.
/// A guest (or host) structure deeper than this yields a catchable error
/// instead of overflowing the native stack in the recursive walkers below.
const MAX_MARSHAL_DEPTH: usize = 512;

fn to_napi(env: sys::napi_env, v: &Value) -> Result<sys::napi_value, VmErr> {
    to_napi_d(env, v, 0)
}

/// Marshal a VM `Value` into a raw N-API value.
///
/// Functions, promises, generators and other VM-only values have no faithful
/// representation in this direction yet and are surfaced as `undefined`.
fn to_napi_d(env: sys::napi_env, v: &Value, depth: usize) -> Result<sys::napi_value, VmErr> {
    if depth > MAX_MARSHAL_DEPTH {
        return Err(VmErr::Msg("value is too deep to marshal".to_string()));
    }
    unsafe {
        let mut out = ptr::null_mut();
        match v {
            Value::Undefined => chk(sys::napi_get_undefined(env, &mut out))?,
            Value::Null => chk(sys::napi_get_null(env, &mut out))?,
            Value::Bool(b) => chk(sys::napi_get_boolean(env, *b, &mut out))?,
            Value::Number(n) => chk(sys::napi_create_double(env, *n, &mut out))?,
            Value::String(s) => return make_str(env, s),
            Value::Array(items) => {
                let items = items.borrow();
                chk(sys::napi_create_array_with_length(
                    env,
                    items.len(),
                    &mut out,
                ))?;
                for (i, item) in items.iter().enumerate() {
                    let ev = to_napi_d(env, item, depth + 1)?;
                    chk(sys::napi_set_element(env, out, i as u32, ev))?;
                }
            }
            Value::Object { props, .. } => {
                chk(sys::napi_create_object(env, &mut out))?;
                let props = props.borrow();
                for (k, val) in props.iter() {
                    let ev = to_napi_d(env, val, depth + 1)?;
                    let ck = CString::new(k.as_str())
                        .map_err(|_| VmErr::Msg("object key contains NUL".to_string()))?;
                    chk(sys::napi_set_named_property(env, out, ck.as_ptr(), ev))?;
                }
            }
            Value::Error(e) => {
                chk(sys::napi_create_object(env, &mut out))?;
                set_str_prop(env, out, "name", &e.name)?;
                set_str_prop(env, out, "message", &e.message)?;
            }
            _ => chk(sys::napi_get_undefined(env, &mut out))?,
        }
        Ok(out)
    }
}

/// Read a raw N-API string into a Rust `String`.
fn read_string(env: sys::napi_env, raw: sys::napi_value) -> Result<String, VmErr> {
    unsafe {
        let mut len: usize = 0;
        chk(sys::napi_get_value_string_utf8(
            env,
            raw,
            ptr::null_mut(),
            0,
            &mut len,
        ))?;
        let mut buf: Vec<u8> = vec![0; len + 1];
        let mut copied: usize = 0;
        chk(sys::napi_get_value_string_utf8(
            env,
            raw,
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
            &mut copied,
        ))?;
        Ok(String::from_utf8_lossy(&buf[..copied]).into_owned())
    }
}

/// Read a named property as a string, returning `""` when the property is
/// absent or not a string. Uses `napi_get_named_property`, which reads
/// non-enumerable own properties too (e.g. an `Error`'s `message`).
fn get_named_str(env: sys::napi_env, obj: sys::napi_value, key: &str) -> Result<String, VmErr> {
    unsafe {
        let ck =
            CString::new(key).map_err(|_| VmErr::Msg("object key contains NUL".to_string()))?;
        let mut pv = ptr::null_mut();
        chk(sys::napi_get_named_property(env, obj, ck.as_ptr(), &mut pv))?;
        let mut t: sys::napi_valuetype = 0;
        chk(sys::napi_typeof(env, pv, &mut t))?;
        if t == sys::ValueType::napi_string {
            read_string(env, pv)
        } else {
            Ok(String::new())
        }
    }
}

fn from_napi(env: sys::napi_env, raw: sys::napi_value) -> Result<Value, VmErr> {
    from_napi_d(env, raw, 0)
}

/// Marshal a raw N-API value into a VM `Value`.
///
/// JavaScript functions are not marshalled into callable VM values here; use
/// `Vm.exposeFunction` to make a Node function callable from the VM.
fn from_napi_d(env: sys::napi_env, raw: sys::napi_value, depth: usize) -> Result<Value, VmErr> {
    if depth > MAX_MARSHAL_DEPTH {
        return Err(VmErr::Msg("value is too deep to marshal".to_string()));
    }
    unsafe {
        if raw.is_null() {
            return Ok(Value::Undefined);
        }
        let mut t: sys::napi_valuetype = 0;
        chk(sys::napi_typeof(env, raw, &mut t))?;
        Ok(match t {
            sys::ValueType::napi_undefined => Value::Undefined,
            sys::ValueType::napi_null => Value::Null,
            sys::ValueType::napi_boolean => {
                let mut b = false;
                chk(sys::napi_get_value_bool(env, raw, &mut b))?;
                Value::Bool(b)
            }
            sys::ValueType::napi_number => {
                let mut n = 0.0;
                chk(sys::napi_get_value_double(env, raw, &mut n))?;
                Value::Number(n)
            }
            sys::ValueType::napi_string => Value::String(read_string(env, raw)?),
            sys::ValueType::napi_object => {
                // A JS `Error` carries its `message` as a *non-enumerable*
                // property, so the generic enumerable-key walk below would drop
                // it. Surface it as a plain object with `name`/`message`, which
                // is exactly how the VM's own `Error` instances are shaped, so
                // `catch (e) { e.message }` works across the boundary.
                let mut is_error = false;
                chk(sys::napi_is_error(env, raw, &mut is_error))?;
                if is_error {
                    let name = get_named_str(env, raw, "name").unwrap_or_else(|_| "Error".into());
                    let message = get_named_str(env, raw, "message").unwrap_or_default();
                    return Ok(Value::object(vec![
                        ("name".to_string(), Value::String(name)),
                        ("message".to_string(), Value::String(message)),
                    ]));
                }
                let mut is_array = false;
                chk(sys::napi_is_array(env, raw, &mut is_array))?;
                if is_array {
                    let mut len: u32 = 0;
                    chk(sys::napi_get_array_length(env, raw, &mut len))?;
                    let mut items =
                        Vec::with_capacity(len.min(crate::value::MAX_ARRAY_LEN as u32) as usize);
                    for i in 0..len {
                        if i as usize >= crate::value::MAX_ARRAY_LEN {
                            return Err(VmErr::Msg(
                                "RangeError: Maximum array length exceeded".to_string(),
                            ));
                        }
                        let mut ev = ptr::null_mut();
                        chk(sys::napi_get_element(env, raw, i, &mut ev))?;
                        items.push(from_napi_d(env, ev, depth + 1)?);
                    }
                    Value::array(items)
                } else {
                    let mut names = ptr::null_mut();
                    chk(sys::napi_get_property_names(env, raw, &mut names))?;
                    let mut len: u32 = 0;
                    chk(sys::napi_get_array_length(env, names, &mut len))?;
                    let mut props = Vec::with_capacity(len as usize);
                    for i in 0..len {
                        let mut key = ptr::null_mut();
                        chk(sys::napi_get_element(env, names, i, &mut key))?;
                        let key_str = read_string(env, key)?;
                        let mut pv = ptr::null_mut();
                        chk(sys::napi_get_property(env, raw, key, &mut pv))?;
                        props.push((key_str, from_napi_d(env, pv, depth + 1)?));
                    }
                    Value::object(props)
                }
            }
            // Functions, symbols, bigints, externals: no VM representation yet.
            _ => Value::Undefined,
        })
    }
}

/// Wrapper asserting a raw pointer can be sent across threads. Used by
/// `run_async` to move the interpreter pointer and TSFN handle to the VM
/// thread. Safety: the channel/TSFN protocol guarantees mutual exclusion —
/// only one thread accesses the pointed-to data at a time.
///
/// Stores the pointer as `usize` to sidestep the compiler's auto-trait
/// analysis on raw pointers (which are `!Send` by default).
#[derive(Clone, Copy)]
struct SendPtr(usize);
unsafe impl Send for SendPtr {}

// ---------------------------------------------------------------------------
// Async host bridge infrastructure
//
// Async host functions (registered via `exposeAsyncFunction`) are dispatched
// to the Node.js main thread via a ThreadsafeFunction. The VM thread parks
// on a channel until the main thread resolves the JS Promise and sends the
// result back.
// ---------------------------------------------------------------------------

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
struct NapiHostBridge {
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
    on_vm_thread: AtomicUsize,
}

impl NapiHostBridge {
    fn new(env: sys::napi_env) -> Self {
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
    fn register(&self, func: sys::napi_value) -> Result<usize, VmErr> {
        let mut r: sys::napi_ref = ptr::null_mut();
        chk(unsafe { sys::napi_create_reference(self.env, func, 1, &mut r) })?;
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.funcs.borrow_mut().insert(id, r);
        Ok(id)
    }

    /// Register an async JS function. Creates the TSFN on first use.
    fn register_async(&self, func: sys::napi_value) -> Result<usize, VmErr> {
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
extern "C" fn run_async_done_cb(
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
            .stack_size(8 * 1024 * 1024)
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
