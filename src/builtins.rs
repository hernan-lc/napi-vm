use crate::error::{VmErr, vm_err};
use crate::interpreter::{Env, Interpreter};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::Value;

pub fn setup_builtins(env: &Env) {
    let mut e = env.borrow_mut();

    let simple: &[&str] = &[
        "Boolean",
        "Error",
        "TypeError",
        "RangeError",
        "SyntaxError",
        "ReferenceError",
        "Map",
        "Set",
        "WeakMap",
        "WeakSet",
        "DataView",
        "RegExp",
        "Function",
        "globalThis",
        "self",
        "window",
        "fetch",
        "URLSearchParams",
        "Headers",
        "Request",
        "Event",
        "EventTarget",
        "CustomEvent",
        "AbortController",
        "AbortSignal",
        "TextEncoder",
        "TextDecoder",
        "ReadableStream",
        "WritableStream",
        "TransformStream",
        "Blob",
        "File",
        "FormData",
        "queueMicrotask",
        "setTimeout",
        "setInterval",
        "clearTimeout",
        "clearInterval",
        "structuredClone",
        "Proxy",
        "undefined",
        "isNaN",
        "isFinite",
        "parseInt",
        "parseFloat",
        "encodeURI",
        "decodeURI",
        "encodeURIComponent",
        "decodeURIComponent",
        "escape",
        "unescape",
        "eval",
        "require",
        "exports",
        "__dirname",
        "__filename",
        "Worker",
        "SharedWorker",
        "MessageChannel",
        "MessagePort",
        "BroadcastChannel",
        "EventSource",
        "ByteLengthQueuingStrategy",
        "CountQueuingStrategy",
        "CompressionStream",
        "DecompressionStream",
        "DOMException",
        "Lock",
        "LockManager",
        "Navigation",
        "Navigator",
        "Notification",
        "PermissionStatus",
        "Permissions",
        "PushManager",
        "PushSubscription",
        "PushSubscriptionOptions",
        "Scheduler",
        "StorageManager",
        "Worklet",
        "CryptoKey",
        "GPU",
        "GPUAdapter",
        "GPUBindGroup",
        "GPUBuffer",
        "GPUCanvasContext",
        "GPUCommandBuffer",
        "GPUCommandEncoder",
        "GPUComputePassEncoder",
        "GPUComputePipeline",
        "GPUDevice",
        "GPUExternalTexture",
        "GPUPipelineLayout",
        "GPUQuerySet",
        "GPUQueue",
        "GPURenderBundle",
        "GPURenderBundleEncoder",
        "GPURenderPassEncoder",
        "GPURenderPipeline",
        "GPUSampler",
        "GPUShaderModule",
        "GPUTexture",
        "GPUTextureView",
        "WGSLLanguageFeatures",
        "importScripts",
        "close",
        "postMessage",
        "parentPort",
        "threadId",
        "workerData",
        "isMainThread",
        "WritableStreamDefaultWriter",
        "WritableStreamDefaultController",
        "ReadableStreamDefaultReader",
        "ReadableStreamBYOBReader",
        "ReadableStreamDefaultController",
        "ReadableByteStreamController",
        "TransformStreamDefaultController",
        "AudioData",
        "EncodedAudioChunk",
        "EncodedVideoChunk",
        "ImageBitmap",
        "OffscreenCanvas",
        "VideoFrame",
        "WebSocketStream",
        "Serial",
        "USB",
        "HID",
        "Bluetooth",
        "Clipboard",
        "Credential",
        "CredentialsContainer",
        "Geolocation",
        "GeolocationPosition",
        "GeolocationCoordinates",
        "GeolocationPositionError",
        "ServiceWorker",
        "ServiceWorkerContainer",
        "ServiceWorkerRegistration",
        "ServiceWorkerGlobalScope",
        "DedicatedWorkerGlobalScope",
        "SharedWorkerGlobalScope",
        "WorkerGlobalScope",
        "UnloadEvent",
    ];
    for name in simple {
        e.set(name, Value::object(vec![]));
    }

    let with_members: &[(&str, &[&str])] = &[
        ("console", &["log", "error", "warn", "info", "debug"]),
        ("Object", &["keys", "values", "entries", "assign"]),
        ("Array", &["isArray", "from", "of"]),
        ("String", &["fromCharCode"]),
        ("Number", &["isNaN", "isFinite", "parseInt", "parseFloat"]),
        ("Symbol", &["iterator"]),
        ("Promise", &["resolve", "reject", "all", "race"]),
        ("ArrayBuffer", &["isView"]),
        ("Date", &["now", "parse", "UTC"]),
        ("URL", &["createObjectURL", "revokeObjectURL"]),
        ("Response", &["json", "text", "redirect"]),
        ("WebSocket", &["CONNECTING", "OPEN", "CLOSING", "CLOSED"]),
        ("crypto", &["getRandomValues", "randomUUID", "subtle"]),
        ("navigator", &["userAgent", "language", "platform"]),
        ("performance", &["now"]),
        ("BigInt", &["asIntN", "asUintN"]),
        (
            "Reflect",
            &[
                "apply",
                "construct",
                "defineProperty",
                "deleteProperty",
                "get",
                "has",
                "set",
            ],
        ),
        ("Intl", &["DateTimeFormat", "NumberFormat"]),
        ("module", &["exports"]),
        (
            "process",
            &["env", "argv", "cwd", "pid", "platform", "version"],
        ),
        ("Buffer", &["alloc", "from", "concat", "isBuffer"]),
        (
            "location",
            &[
                "href", "protocol", "host", "pathname", "search", "hash", "origin",
            ],
        ),
        (
            "history",
            &[
                "length",
                "go",
                "back",
                "forward",
                "pushState",
                "replaceState",
            ],
        ),
        ("screen", &["width", "height"]),
        (
            "localStorage",
            &["getItem", "setItem", "removeItem", "clear"],
        ),
        (
            "sessionStorage",
            &["getItem", "setItem", "removeItem", "clear"],
        ),
        ("indexedDB", &["open", "deleteDatabase"]),
        ("caches", &["open", "has", "delete", "keys", "match"]),
        ("Cache", &["match", "add", "put", "delete", "keys"]),
        ("CacheStorage", &["open", "has", "delete", "keys"]),
        (
            "SubtleCrypto",
            &[
                "encrypt",
                "decrypt",
                "sign",
                "verify",
                "digest",
                "generateKey",
                "deriveKey",
                "deriveBits",
                "importKey",
                "exportKey",
                "wrapKey",
                "unwrapKey",
            ],
        ),
        (
            "MessageEvent",
            &["data", "origin", "lastEventId", "source", "ports"],
        ),
        (
            "ErrorEvent",
            &["message", "filename", "lineno", "colno", "error"],
        ),
        ("PromiseRejectionEvent", &["promise", "reason"]),
        ("CloseEvent", &["code", "reason", "wasClean"]),
        ("HashChangeEvent", &["oldURL", "newURL"]),
        ("PopStateEvent", &["state"]),
        (
            "StorageEvent",
            &["key", "oldValue", "newValue", "url", "storageArea"],
        ),
        ("SubmitEvent", &["submitter"]),
        ("FormDataEvent", &["formData"]),
        ("ProgressEvent", &["lengthComputable", "loaded", "total"]),
        ("PageTransitionEvent", &["persisted"]),
        ("BeforeUnloadEvent", &["returnValue"]),
        ("UIEvent", &["detail", "view", "which"]),
        (
            "MouseEvent",
            &[
                "screenX",
                "screenY",
                "clientX",
                "clientY",
                "ctrlKey",
                "shiftKey",
                "altKey",
                "metaKey",
                "button",
                "buttons",
                "relatedTarget",
            ],
        ),
        (
            "KeyboardEvent",
            &[
                "key",
                "code",
                "location",
                "ctrlKey",
                "shiftKey",
                "altKey",
                "metaKey",
                "repeat",
                "isComposing",
            ],
        ),
        (
            "TouchEvent",
            &["touches", "targetTouches", "changedTouches"],
        ),
        (
            "Touch",
            &[
                "identifier",
                "target",
                "screenX",
                "screenY",
                "clientX",
                "clientY",
                "pageX",
                "pageY",
            ],
        ),
        ("WheelEvent", &["deltaX", "deltaY", "deltaZ", "deltaMode"]),
        ("DragEvent", &["dataTransfer"]),
        ("FocusEvent", &["relatedTarget"]),
        ("InputEvent", &["data", "inputType", "isComposing"]),
        ("CompositionEvent", &["data"]),
        (
            "PointerEvent",
            &[
                "pointerId",
                "width",
                "height",
                "pressure",
                "pointerType",
                "isPrimary",
            ],
        ),
        (
            "AnimationEvent",
            &["animationName", "elapsedTime", "pseudoElement"],
        ),
        (
            "TransitionEvent",
            &["propertyName", "elapsedTime", "pseudoElement"],
        ),
        ("ClipboardEvent", &["clipboardData"]),
        (
            "SecurityPolicyViolationEvent",
            &["documentURI", "referrer", "blockedURI", "violatedDirective"],
        ),
        ("JSON", &["parse", "stringify"]),
    ];
    for (name, members) in with_members {
        let props: Vec<(String, Value)> = members
            .iter()
            .map(|m| (m.to_string(), Value::Undefined))
            .collect();
        e.set(name, Value::object(props));
    }

    e.set(
        "Math",
        Value::object(vec![
            ("PI".to_string(), Value::Number(std::f64::consts::PI)),
            ("E".to_string(), Value::Number(std::f64::consts::E)),
            ("LN2".to_string(), Value::Number(std::f64::consts::LN_2)),
            ("LN10".to_string(), Value::Number(std::f64::consts::LN_10)),
            ("LOG2E".to_string(), Value::Number(std::f64::consts::LOG2_E)),
            ("LOG10E".to_string(), Value::Number(std::f64::consts::LOG10_E)),
            ("SQRT1_2".to_string(), Value::Number(std::f64::consts::FRAC_1_SQRT_2)),
            ("SQRT2".to_string(), Value::Number(std::f64::consts::SQRT_2)),
            ("abs".to_string(), Value::Undefined),
            ("floor".to_string(), Value::Undefined),
            ("ceil".to_string(), Value::Undefined),
            ("round".to_string(), Value::Undefined),
            ("sqrt".to_string(), Value::Undefined),
            ("pow".to_string(), Value::Undefined),
            ("min".to_string(), Value::Undefined),
            ("max".to_string(), Value::Undefined),
            ("random".to_string(), Value::Undefined),
        ]),
    );

    install_functions(&mut e);

    e.set("Infinity", Value::Number(f64::INFINITY));
    e.set("NaN", Value::Number(f64::NAN));
}

/// Overwrite the placeholder members above with real native implementations.
fn install_functions(e: &mut crate::interpreter::Environment) {
    // Math methods.
    if let Some(math) = e.get("Math") {
        for (name, f) in math_methods() {
            math.set_prop(name, f);
        }
    }
    if let Some(o) = e.get("Object") {
        o.set_prop("keys".to_string(), nf("keys", object_keys));
        o.set_prop("values".to_string(), nf("values", object_values));
        o.set_prop("entries".to_string(), nf("entries", object_entries));
        o.set_prop("assign".to_string(), nf("assign", object_assign));
    }
    if let Some(a) = e.get("Array") {
        a.set_prop("isArray".to_string(), nf("isArray", array_is_array));
    }
    if let Some(n) = e.get("Number") {
        n.set_prop("isNaN".to_string(), nf("isNaN", number_is_nan));
        n.set_prop("isFinite".to_string(), nf("isFinite", number_is_finite));
        n.set_prop("parseInt".to_string(), nf("parseInt", global_parse_int));
        n.set_prop("parseFloat".to_string(), nf("parseFloat", global_parse_float));
    }
    if let Some(j) = e.get("JSON") {
        j.set_prop("stringify".to_string(), nf("stringify", json_stringify));
        j.set_prop("parse".to_string(), nf("parse", json_parse));
    }
    // Global functions.
    e.set("parseInt", nf("parseInt", global_parse_int));
    e.set("parseFloat", nf("parseFloat", global_parse_float));
    e.set("isNaN", nf("isNaN", global_is_nan));
    e.set("isFinite", nf("isFinite", global_is_finite));
}

// ===========================================================================
// Native function implementations.
// ===========================================================================

type NativeFn = fn(&mut Interpreter, Value, Vec<Value>) -> Result<Value, VmErr>;

fn nf(name: &str, callable: NativeFn) -> Value {
    Value::NativeFunction {
        name: name.to_string(),
        callable,
    }
}

fn arg_num(args: &[Value], i: usize) -> f64 {
    args.get(i).map(|v| v.to_number()).unwrap_or(f64::NAN)
}

fn arr_items(this: &Value) -> Vec<Value> {
    match this {
        Value::Array(a) => a.borrow().clone(),
        _ => vec![],
    }
}

fn str_this(interp: &Interpreter, this: &Value) -> String {
    match this {
        Value::String(s) => s.clone(),
        _ => interp.vs(this),
    }
}

/// Display a value the way `Array.prototype.join` / string coercion does:
/// `null`/`undefined` become the empty string.
fn join_str(interp: &Interpreter, v: &Value) -> String {
    match v {
        Value::Null | Value::Undefined => String::new(),
        _ => interp.vs(v),
    }
}

// --- Math -------------------------------------------------------------------

fn math_methods() -> Vec<(String, Value)> {
    let table: Vec<(&str, NativeFn)> = vec![
        ("abs", math_abs),
        ("floor", math_floor),
        ("ceil", math_ceil),
        ("round", math_round),
        ("sqrt", math_sqrt),
        ("cbrt", math_cbrt),
        ("pow", math_pow),
        ("min", math_min),
        ("max", math_max),
        ("random", math_random),
        ("trunc", math_trunc),
        ("sign", math_sign),
        ("log", math_log),
        ("log2", math_log2),
        ("log10", math_log10),
        ("exp", math_exp),
        ("sin", math_sin),
        ("cos", math_cos),
        ("tan", math_tan),
        ("hypot", math_hypot),
    ];
    table.into_iter().map(|(n, f)| (n.to_string(), nf(n, f))).collect()
}

fn math_abs(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).abs()))
}
fn math_floor(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).floor()))
}
fn math_ceil(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).ceil()))
}
fn math_round(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let x = arg_num(&a, 0);
    // JS rounds halves toward +Infinity.
    Ok(Value::Number((x + 0.5).floor()))
}
fn math_sqrt(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).sqrt()))
}
fn math_cbrt(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).cbrt()))
}
fn math_pow(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).powf(arg_num(&a, 1))))
}
fn math_trunc(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).trunc()))
}
fn math_sign(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let x = arg_num(&a, 0);
    let r = if x.is_nan() { f64::NAN } else if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 };
    Ok(Value::Number(r))
}
fn math_log(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).ln()))
}
fn math_log2(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).log2()))
}
fn math_log10(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).log10()))
}
fn math_exp(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).exp()))
}
fn math_sin(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).sin()))
}
fn math_cos(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).cos()))
}
fn math_tan(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(arg_num(&a, 0).tan()))
}
fn math_min(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    if a.is_empty() {
        return Ok(Value::Number(f64::INFINITY));
    }
    let mut m = f64::INFINITY;
    for v in &a {
        let n = v.to_number();
        if n.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        if n < m {
            m = n;
        }
    }
    Ok(Value::Number(m))
}
fn math_max(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    if a.is_empty() {
        return Ok(Value::Number(f64::NEG_INFINITY));
    }
    let mut m = f64::NEG_INFINITY;
    for v in &a {
        let n = v.to_number();
        if n.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        if n > m {
            m = n;
        }
    }
    Ok(Value::Number(m))
}
fn math_hypot(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let mut sum = 0.0;
    for v in &a {
        let n = v.to_number();
        sum += n * n;
    }
    Ok(Value::Number(sum.sqrt()))
}
fn math_random(_: &mut Interpreter, _: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);
    let mut x = SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    Ok(Value::Number((x >> 11) as f64 / (1u64 << 53) as f64))
}

// --- Array statics ----------------------------------------------------------

fn array_is_array(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Bool(matches!(a.get(0), Some(Value::Array(_)))))
}

// --- Array prototype --------------------------------------------------------

/// Dispatch table for `Array.prototype` methods, looked up by `prop()`.
pub fn array_method(name: &str) -> Option<Value> {
    let f: NativeFn = match name {
        "map" => array_map,
        "filter" => array_filter,
        "reduce" => array_reduce,
        "forEach" => array_for_each,
        "find" => array_find,
        "some" => array_some,
        "every" => array_every,
        "push" => array_push,
        "pop" => array_pop,
        "join" => array_join,
        "indexOf" => array_index_of,
        "includes" => array_includes,
        "slice" => array_slice,
        "concat" => array_concat,
        "reverse" => array_reverse,
        _ => return None,
    };
    Some(nf(name, f))
}

fn array_map(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    let mut out = Vec::with_capacity(items.len());
    for (i, it) in items.iter().enumerate() {
        let r = interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
        out.push(r);
    }
    Ok(Value::array(out))
}

fn array_filter(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    let mut out = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let keep = interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
        if keep.is_truthy() {
            out.push(it.clone());
        }
    }
    Ok(Value::array(out))
}

fn array_reduce(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    let (mut acc, start) = if a.len() >= 2 {
        (a[1].clone(), 0)
    } else {
        (items.get(0).cloned().unwrap_or(Value::Undefined), 1)
    };
    for i in start..items.len() {
        acc = interp.call_this(
            &cb,
            Value::Undefined,
            vec![acc, items[i].clone(), Value::Number(i as f64), this.clone()],
        )?;
    }
    Ok(acc)
}

fn array_for_each(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    for (i, it) in items.iter().enumerate() {
        interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
    }
    Ok(Value::Undefined)
}

fn array_find(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    for (i, it) in items.iter().enumerate() {
        let hit = interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
        if hit.is_truthy() {
            return Ok(it.clone());
        }
    }
    Ok(Value::Undefined)
}

fn array_some(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    for (i, it) in items.iter().enumerate() {
        let hit = interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
        if hit.is_truthy() {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

fn array_every(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let cb = a.get(0).cloned().unwrap_or(Value::Undefined);
    for (i, it) in items.iter().enumerate() {
        let hit = interp.call_this(
            &cb,
            Value::Undefined,
            vec![it.clone(), Value::Number(i as f64), this.clone()],
        )?;
        if !hit.is_truthy() {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn array_push(_: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    if let Value::Array(items) = &this {
        let mut b = items.borrow_mut();
        for x in a {
            b.push(x);
        }
        return Ok(Value::Number(b.len() as f64));
    }
    Ok(Value::Undefined)
}

fn array_pop(_: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    if let Value::Array(items) = &this {
        return Ok(items.borrow_mut().pop().unwrap_or(Value::Undefined));
    }
    Ok(Value::Undefined)
}

fn array_join(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let sep = match a.get(0) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Undefined) | None => ",".to_string(),
        Some(v) => interp.vs(v),
    };
    let parts: Vec<String> = items.iter().map(|v| join_str(interp, v)).collect();
    Ok(Value::String(parts.join(&sep)))
}

fn array_index_of(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let target = a.get(0).cloned().unwrap_or(Value::Undefined);
    for (i, it) in items.iter().enumerate() {
        if interp.seq(it, &target) {
            return Ok(Value::Number(i as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

fn array_includes(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let target = a.get(0).cloned().unwrap_or(Value::Undefined);
    for it in &items {
        if interp.seq(it, &target) {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

fn array_slice(_: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let items = arr_items(&this);
    let len = items.len() as i64;
    let norm = |v: f64| -> i64 {
        if v.is_nan() {
            return 0;
        }
        let i = v as i64;
        if i < 0 {
            (len + i).max(0)
        } else {
            i.min(len)
        }
    };
    let start = norm(a.get(0).map(|v| v.to_number()).unwrap_or(0.0));
    let end = match a.get(1) {
        Some(v) => norm(v.to_number()),
        None => len,
    };
    if start >= end {
        return Ok(Value::array(vec![]));
    }
    Ok(Value::array(items[start as usize..end as usize].to_vec()))
}

fn array_concat(_: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let mut out = arr_items(&this);
    for v in a {
        match v {
            Value::Array(items) => out.extend(items.borrow().iter().cloned()),
            other => out.push(other),
        }
    }
    Ok(Value::array(out))
}

fn array_reverse(_: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    if let Value::Array(items) = &this {
        items.borrow_mut().reverse();
        return Ok(this);
    }
    Ok(Value::Undefined)
}

// --- String prototype -------------------------------------------------------

pub fn string_method(name: &str) -> Option<Value> {
    let f: NativeFn = match name {
        "toUpperCase" => string_to_upper,
        "toLowerCase" => string_to_lower,
        "trim" => string_trim,
        "slice" => string_slice,
        "substring" => string_slice,
        "split" => string_split,
        "includes" => string_includes,
        "indexOf" => string_index_of,
        "charAt" => string_char_at,
        "startsWith" => string_starts_with,
        "endsWith" => string_ends_with,
        "repeat" => string_repeat,
        "replace" => string_replace,
        _ => return None,
    };
    Some(nf(name, f))
}

fn string_to_upper(interp: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::String(str_this(interp, &this).to_uppercase()))
}
fn string_to_lower(interp: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::String(str_this(interp, &this).to_lowercase()))
}
fn string_trim(interp: &mut Interpreter, this: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::String(str_this(interp, &this).trim().to_string()))
}
fn string_slice(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len() as i64;
    let norm = |v: f64| -> i64 {
        if v.is_nan() {
            return 0;
        }
        let i = v as i64;
        if i < 0 {
            (len + i).max(0)
        } else {
            i.min(len)
        }
    };
    let start = norm(a.get(0).map(|v| v.to_number()).unwrap_or(0.0));
    let end = match a.get(1) {
        Some(v) => norm(v.to_number()),
        None => len,
    };
    if start >= end {
        return Ok(Value::String(String::new()));
    }
    Ok(Value::String(chars[start as usize..end as usize].iter().collect()))
}
fn string_split(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    match a.get(0) {
        Some(Value::String(sep)) => {
            let mut parts: Vec<Value> = s
                .split(sep.as_str())
                .map(|p| Value::String(p.to_string()))
                .collect();
            if let Some(l) = a.get(1) {
                parts.truncate(l.to_number().max(0.0) as usize);
            }
            Ok(Value::array(parts))
        }
        _ => Ok(Value::array(vec![Value::String(s)])),
    }
}
fn string_includes(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let needle = match a.get(0) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => String::new(),
    };
    Ok(Value::Bool(s.contains(&needle)))
}
fn string_index_of(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let needle = match a.get(0) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => String::new(),
    };
    Ok(Value::Number(s.find(&needle).map(|i| i as f64).unwrap_or(-1.0)))
}
fn string_char_at(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let idx = a.get(0).map(|v| v.to_number() as usize).unwrap_or(0);
    Ok(Value::String(
        s.chars().nth(idx).map(|c| c.to_string()).unwrap_or_default(),
    ))
}
fn string_starts_with(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let needle = match a.get(0) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => String::new(),
    };
    Ok(Value::Bool(s.starts_with(&needle)))
}
fn string_ends_with(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let needle = match a.get(0) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => String::new(),
    };
    Ok(Value::Bool(s.ends_with(&needle)))
}
fn string_repeat(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let n = a.get(0).map(|v| v.to_number() as usize).unwrap_or(0);
    Ok(Value::String(s.repeat(n)))
}
fn string_replace(interp: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = str_this(interp, &this);
    let from = match a.get(0) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => return Ok(Value::String(s)),
    };
    let to = match a.get(1) {
        Some(Value::String(n)) => n.clone(),
        Some(v) => interp.vs(v),
        None => String::new(),
    };
    Ok(Value::String(s.replacen(&from, &to, 1)))
}

// --- Number prototype -------------------------------------------------------

pub fn number_method(name: &str) -> Option<Value> {
    let f: NativeFn = match name {
        "toFixed" => number_to_fixed,
        _ => return None,
    };
    Some(nf(name, f))
}

fn number_to_fixed(_: &mut Interpreter, this: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let n = this.to_number();
    let digits = a.get(0).map(|v| v.to_number() as usize).unwrap_or(0);
    Ok(Value::String(format!("{:.*}", digits, n)))
}

// --- Number statics ---------------------------------------------------------

fn number_is_nan(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Bool(matches!(a.get(0), Some(Value::Number(n)) if n.is_nan())))
}
fn number_is_finite(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Bool(matches!(a.get(0), Some(Value::Number(n)) if n.is_finite())))
}

// --- Global functions -------------------------------------------------------

fn global_is_nan(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let n = a.get(0).map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Value::Bool(n.is_nan()))
}
fn global_is_finite(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let n = a.get(0).map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Value::Bool(n.is_finite()))
}

fn global_parse_int(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = match a.get(0) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => interp.vs(v),
        None => return Ok(Value::Number(f64::NAN)),
    };
    let mut radix = match a.get(1) {
        Some(v) => v.to_number() as u32,
        None => 0,
    };
    let t = s.trim();
    let mut chars = t.chars();
    let mut first = chars.next();
    let mut neg = false;
    if first == Some('+') {
        first = chars.next();
    } else if first == Some('-') {
        neg = true;
        first = chars.next();
    }
    // Infer radix from a 0x/0X prefix when unspecified.
    if radix == 0 {
        if first == Some('0') {
            let mut peek = chars.clone();
            if matches!(peek.next(), Some('x') | Some('X')) {
                radix = 16;
                chars.next();
                first = chars.next();
            } else {
                radix = 10;
            }
        } else {
            radix = 10;
        }
    }
    let mut val: i64 = 0;
    let mut any = false;
    let mut cur = first;
    while let Some(c) = cur {
        match c.to_digit(radix) {
            Some(d) => {
                val = val.saturating_mul(radix as i64).saturating_add(d as i64);
                any = true;
                cur = chars.next();
            }
            None => break,
        }
    }
    if !any {
        return Ok(Value::Number(f64::NAN));
    }
    Ok(Value::Number((if neg { -val } else { val }) as f64))
}

fn global_parse_float(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = match a.get(0) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => interp.vs(v),
        None => return Ok(Value::Number(f64::NAN)),
    };
    let t = s.trim();
    let mut end = 0usize;
    let mut seen_digit = false;
    for (i, c) in t.char_indices() {
        let ok = c.is_ascii_digit()
            || (c == '.' && seen_digit)
            || ((c == '+' || c == '-') && i == 0)
            || ((c == 'e' || c == 'E') && seen_digit);
        if ok {
            if c.is_ascii_digit() {
                seen_digit = true;
            }
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    match t[..end].parse::<f64>() {
        Ok(n) => Ok(Value::Number(n)),
        Err(_) => Ok(Value::Number(f64::NAN)),
    }
}

// --- Object statics ---------------------------------------------------------

fn object_keys(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let v = a.get(0).cloned().unwrap_or(Value::Undefined);
    Ok(Value::array(interp.keys(&v).into_iter().map(Value::String).collect()))
}
fn object_values(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    match a.get(0) {
        Some(Value::Object { props, .. }) => {
            Ok(Value::array(props.borrow().iter().map(|(_, v)| v.clone()).collect()))
        }
        _ => Ok(Value::array(vec![])),
    }
}
fn object_entries(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    match a.get(0) {
        Some(Value::Object { props, .. }) => {
            let entries = props
                .borrow()
                .iter()
                .map(|(k, v)| Value::array(vec![Value::String(k.clone()), v.clone()]))
                .collect();
            Ok(Value::array(entries))
        }
        _ => Ok(Value::array(vec![])),
    }
}
fn object_assign(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let target = a.get(0).cloned().unwrap_or_else(|| Value::object(vec![]));
    for src in a.iter().skip(1) {
        if let Value::Object { props, .. } = src {
            for (k, v) in props.borrow().iter() {
                target.set_prop(k.clone(), v.clone());
            }
        }
    }
    Ok(target)
}

// --- JSON -------------------------------------------------------------------

fn json_stringify(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let v = a.get(0).cloned().unwrap_or(Value::Undefined);
    if matches!(v, Value::Undefined) {
        return Ok(Value::Undefined);
    }
    Ok(Value::String(json_serialize(interp, &v)))
}

fn json_serialize(interp: &Interpreter, v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Undefined => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if n.is_nan() || n.is_infinite() {
                "null".to_string()
            } else if n.fract() == 0.0 && n.abs() < 1e15 {
                format!("{:.0}", n)
            } else {
                n.to_string()
            }
        }
        Value::String(s) => format!("\"{}\"", escape_json(s)),
        Value::Array(items) => {
            let parts: Vec<String> = items.borrow().iter().map(|x| json_serialize(interp, x)).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object { props, .. } => {
            let parts: Vec<String> = props
                .borrow()
                .iter()
                .filter(|(_, v)| !matches!(v, Value::Undefined))
                .map(|(k, v)| format!("\"{}\":{}", escape_json(k), json_serialize(interp, v)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        _ => "null".to_string(),
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn json_parse(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = match a.get(0) {
        Some(Value::String(s)) => s.clone(),
        _ => return vm_err("JSON.parse requires a string argument"),
    };
    let mut lex = Lexer::new(&s);
    let toks = lex.tokenize();
    let mut parser = Parser::new(toks);
    let expr = parser
        .expr()
        .ok_or_else(|| VmErr::Msg("Invalid JSON".to_string()))?;
    interp.eval_expr(&expr)
}
