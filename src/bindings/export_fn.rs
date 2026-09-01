//! Exporting a VM function to the host.
//!
//! A VM function has no wire form: it closes over interpreter state that
//! cannot leave the interpreter. What *can* cross is a host function that
//! re-enters the VM and calls it — which is what this builds.
//!
//! The exported value is kept in a table on the runtime, and the host function
//! carries the table index plus an `Arc<VMState>`. The `Arc` is what makes
//! this sound: the VM's state outlives every function exported from it, so a
//! call after the `VM` object is gone finds a disposed VM and reports that,
//! rather than dereferencing freed memory.

use std::ffi::c_void;
use std::ptr;
use std::sync::Arc;

use napi::sys;

use super::marshal::{from_napi, to_napi};
use super::vm::VMState;
use crate::error::VmErr;
use crate::value::Value;

/// What an exported function needs to call back into the VM.
struct ExportedFunction {
    state: Arc<VMState>,
    /// Index into the runtime's export table.
    index: usize,
}

/// Is this a value that must cross as a callable rather than as data?
pub(super) fn is_callable(value: &Value) -> bool {
    matches!(
        value,
        Value::Function(_) | Value::NativeFunction { .. } | Value::Class(_)
    ) || matches!(value, Value::Generator { .. })
}

/// Build a host function that calls `value` inside the VM.
pub(super) fn export(
    env: sys::napi_env,
    state: &Arc<VMState>,
    value: &Value,
) -> Result<sys::napi_value, VmErr> {
    let index = state.register_export(value.clone());
    let context = Box::new(ExportedFunction {
        state: state.clone(),
        index,
    });
    let context_ptr = Box::into_raw(context);

    let mut function = ptr::null_mut();
    let create = unsafe {
        sys::napi_create_function(
            env,
            c"vmFunction".as_ptr(),
            10,
            Some(call_into_vm),
            context_ptr as *mut c_void,
            &mut function,
        )
    };
    if create != sys::Status::napi_ok {
        drop(unsafe { Box::from_raw(context_ptr) });
        return Err(VmErr::Msg(format!(
            "failed to export a VM function (status {})",
            create
        )));
    }

    // N-API owns the function object from here; the finalizer reclaims the
    // context when it becomes unreachable. A rejected finalizer (an
    // environment already tearing down) leaves a bounded leak, never a
    // use-after-free.
    let mut ignored = ptr::null_mut();
    let finalize = unsafe {
        sys::napi_add_finalizer(
            env,
            function,
            context_ptr as *mut c_void,
            Some(finalize_export),
            ptr::null_mut(),
            &mut ignored,
        )
    };
    if finalize != sys::Status::napi_ok {
        drop(unsafe { Box::from_raw(context_ptr) });
        return Err(VmErr::Msg(format!(
            "failed to register the export finalizer (status {})",
            finalize
        )));
    }
    Ok(function)
}

extern "C" fn finalize_export(_env: sys::napi_env, data: *mut c_void, _hint: *mut c_void) {
    if data.is_null() {
        return;
    }
    // SAFETY: `data` is the pointer `export` leaked, handed back exactly once.
    let context = unsafe { Box::from_raw(data as *mut ExportedFunction) };
    context.state.release_export(context.index);
}

/// Throw a JavaScript error and return `undefined`, the shape a failing N-API
/// callback must have.
///
/// `napi_throw_error` takes a C string, so the message is copied into one. A
/// message containing a NUL is truncated at it rather than dropped.
fn throw(env: sys::napi_env, message: &str) -> sys::napi_value {
    let text = std::ffi::CString::new(message).unwrap_or_else(|error| {
        let valid = error.into_vec();
        let end = valid.iter().position(|b| *b == 0).unwrap_or(valid.len());
        std::ffi::CString::new(&valid[..end]).unwrap_or_default()
    });
    unsafe { sys::napi_throw_error(env, ptr::null(), text.as_ptr()) };
    let mut undefined = ptr::null_mut();
    unsafe { sys::napi_get_undefined(env, &mut undefined) };
    undefined
}

extern "C" fn call_into_vm(env: sys::napi_env, info: sys::napi_callback_info) -> sys::napi_value {
    // Read the arguments and the context the function was created with.
    let mut argc = 0usize;
    let mut data = ptr::null_mut();
    let probe = unsafe {
        sys::napi_get_cb_info(
            env,
            info,
            &mut argc,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut data,
        )
    };
    if probe != sys::Status::napi_ok || data.is_null() {
        return throw(env, "VM function called without its context");
    }
    let mut argv = vec![ptr::null_mut(); argc];
    if argc > 0 {
        let read = unsafe {
            sys::napi_get_cb_info(
                env,
                info,
                &mut argc,
                argv.as_mut_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if read != sys::Status::napi_ok {
            return throw(env, "could not read the arguments of a VM function");
        }
    }

    // SAFETY: the pointer was created by `export` and is only freed by the
    // finalizer, which runs after the function is unreachable from JavaScript.
    let context = unsafe { &*(data as *const ExportedFunction) };

    match context.invoke(env, &argv) {
        Ok(value) => value,
        Err(error) => throw(env, &error.to_string()),
    }
}

impl ExportedFunction {
    fn invoke(
        &self,
        env: sys::napi_env,
        argv: &[sys::napi_value],
    ) -> Result<sys::napi_value, VmErr> {
        // The VM is single-threaded and re-entrant calls are refused, so a
        // host callback that fires while the VM is already running reports
        // that rather than corrupting interpreter state.
        let _busy = self
            .state
            .try_start()
            .map_err(|_| VmErr::Msg("VM is busy with another execution".to_string()))?;

        let mut args = Vec::with_capacity(argv.len());
        for raw in argv {
            args.push(from_napi(env, *raw)?);
        }

        let result = self.state.with_runtime(|runtime| {
            let Some(callee) = runtime.export(self.index) else {
                return Err(VmErr::Msg(
                    "this VM function is no longer available".to_string(),
                ));
            };
            runtime.interp.begin_execution();
            let value = runtime.interp.call_this(&callee, Value::Undefined, args)?;
            // The event loop runs before the value crosses out, so a promise
            // the call produced is settled by the time the host sees it.
            runtime.interp.drain_jobs()?;
            Ok(value)
        })?;
        // Outside the gate, so a function among the results can be exported in
        // turn; the busy guard still holds.
        super::marshal::exporting_from(&self.state, || to_napi(env, &result))
    }
}
