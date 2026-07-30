//! Structured marshalling of values across the NAPI boundary.
//!
//! The free functions in the parent module are string-only (`runCode` /
//! `getGlobal` return strings). This module passes *real* structured values
//! across the boundary so Node can expose functions to the VM and call VM
//! functions with live arguments. It is built directly on the stable raw
//! `napi_sys` ABI: the VM is single-threaded, so a persisted `napi_ref` to
//! a JavaScript function can be stored and invoked synchronously on the
//! same thread that drives the interpreter.
//!
//! Each marshalling helper is a *safe* function that confines all FFI to a
//! single `unsafe` block around its body; the `env` handle always
//! originates from a live N-API callback, so the operations are sound.
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use napi::sys;

use crate::error::VmErr;
use crate::value::Value;


#[inline]
pub(super) fn chk(status: sys::napi_status) -> Result<(), VmErr> {
    if status == sys::Status::napi_ok {
        Ok(())
    } else {
        Err(VmErr::Msg(format!("napi call failed (status {})", status)))
    }
}

/// Create a JS string from a Rust `&str`.
pub(super) fn make_str(env: sys::napi_env, s: &str) -> Result<sys::napi_value, VmErr> {
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

pub(super) fn to_napi(env: sys::napi_env, v: &Value) -> Result<sys::napi_value, VmErr> {
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
pub(super) fn get_named_str(
    env: sys::napi_env,
    obj: sys::napi_value,
    key: &str,
) -> Result<String, VmErr> {
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

pub(super) fn from_napi(env: sys::napi_env, raw: sys::napi_value) -> Result<Value, VmErr> {
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
pub(super) struct SendPtr(pub(super) usize);
unsafe impl Send for SendPtr {}
