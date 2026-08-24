use std::fmt;

use crate::span::Span;
use crate::value::{ErrorData, Value};

/// A single frame in the call stack trace.
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Shared so per-call frame pushes/pop-and-snapshot never allocate for
    /// the name: function values already own an `Rc<str>` and cloning one is
    /// a refcount bump.
    pub name: std::rc::Rc<str>,
    pub span: Span,
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.span.is_unknown() {
            write!(f, "    at {}", self.name)
        } else {
            write!(f, "    at {} ({})", self.name, self.span)
        }
    }
}

/// Payload of `VmErr::RuntimeError`, boxed so the error enum — and therefore
/// the `Result<Value, VmErr>` returned by every eval function — stays small
/// on the success path. The payload is only ever constructed when an error
/// actually occurred, so the extra allocation is cold-path.
#[derive(Debug)]
pub struct RuntimeErrorData {
    pub message: String,
    pub span: Option<Span>,
    pub stack: Vec<StackFrame>,
}

#[derive(Debug)]
pub enum VmErr {
    Ret(Value),
    /// A value thrown by user code (`throw expr`). Carries the original value
    /// so `catch (e)` can inspect thrown objects (e.g. `e.message`).
    Throw(Value),
    Msg(String),
    /// A runtime error with source location context.
    RuntimeError(Box<RuntimeErrorData>),
    /// Control-flow signal for `break`, with an optional target label. Caught
    /// by the enclosing loop/switch; not an error and not catchable by `try`.
    Break(Option<String>),
    /// Control-flow signal for `continue`, with an optional target label.
    Continue(Option<String>),
}

// Guard the hot-path size: every eval function returns this `Result`. If it
// grows, the whole interpreter slows down — box the offending payload.
const _: () = assert!(std::mem::size_of::<Result<Value, VmErr>>() <= 48);

impl VmErr {
    /// Attach source location and call stack to a `VmErr::Msg`.
    pub fn with_context(self, span: Option<Span>, stack: &[StackFrame]) -> Self {
        match self {
            VmErr::Msg(message) => VmErr::RuntimeError(Box::new(RuntimeErrorData {
                message,
                span,
                stack: stack.to_vec(),
            })),
            other => other,
        }
    }
}

impl fmt::Display for VmErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VmErr::Msg(s) => write!(f, "{}", s),
            VmErr::RuntimeError(inner) => {
                let RuntimeErrorData {
                    message,
                    span,
                    stack,
                } = inner.as_ref();
                write!(f, "{}", message)?;
                if let Some(span) = span
                    && !span.is_unknown()
                {
                    write!(f, "\n  at {}", span)?;
                }
                for frame in stack.iter().rev() {
                    write!(f, "\n{}", frame)?;
                }
                Ok(())
            }
            VmErr::Throw(v) => write!(f, "{}", throw_display(v)),
            VmErr::Ret(_) => write!(f, "return"),
            VmErr::Break(Some(l)) => write!(f, "break outside loop (label {})", l),
            VmErr::Break(None) => write!(f, "break outside loop"),
            VmErr::Continue(Some(l)) => write!(f, "continue outside loop (label {})", l),
            VmErr::Continue(None) => write!(f, "continue outside loop"),
        }
    }
}

/// Render a thrown value as an error message without needing an interpreter
/// (used when an uncaught throw crosses the NAPI boundary).
fn throw_display(v: &Value) -> String {
    use crate::value::MAX_STRING_LEN;

    fn append(out: &mut String, text: &str) -> bool {
        if out.len().saturating_add(text.len()) > MAX_STRING_LEN {
            return false;
        }
        out.push_str(text);
        true
    }

    let mut out = String::new();
    let ok = match v {
        Value::String(s) => append(&mut out, s),
        Value::Error(inner) => {
            let mut ok = true;
            if inner.name != "Error" {
                ok &= append(&mut out, &inner.name);
                ok &= append(&mut out, ": ");
            }
            ok &= append(&mut out, &inner.message);
            ok
        }
        Value::Object { props, .. } => {
            let borrow = props.borrow();
            let name = borrow.iter().find_map(|(key, value)| {
                (key == "name")
                    .then_some(value)
                    .and_then(|value| match value {
                        Value::String(value) => Some(value.as_str()),
                        _ => None,
                    })
            });
            let message = borrow.iter().find_map(|(key, value)| {
                (key == "message")
                    .then_some(value)
                    .and_then(|value| match value {
                        Value::String(value) => Some(value.as_str()),
                        _ => None,
                    })
            });
            match (name, message) {
                (Some(name), Some(message)) if name != "Error" => {
                    append(&mut out, name) && append(&mut out, ": ") && append(&mut out, message)
                }
                (_, Some(message)) => append(&mut out, message),
                _ => append(&mut out, "Uncaught error"),
            }
        }
        Value::Number(n) => append(&mut out, &n.to_string()),
        Value::Bool(b) => append(&mut out, if *b { "true" } else { "false" }),
        Value::Null => append(&mut out, "null"),
        Value::Undefined => append(&mut out, "undefined"),
        _ => append(&mut out, "error"),
    };
    if ok {
        out
    } else {
        "RangeError: Maximum string length exceeded".to_string()
    }
}

/// Build the guest-visible error value for an internal error message.
/// Messages may carry a `"Name: message"` prefix naming one of the standard
/// error types (as produced by `limit_err` and the interpreter guards);
/// anything else becomes a plain `Error`. This is what lets guest code do
/// `try { ... } catch (e) { e.message }` on internally raised errors.
pub fn error_value_from_msg(message: &str) -> Value {
    const NAMES: &[&str] = &[
        "TypeError",
        "RangeError",
        "SyntaxError",
        "ReferenceError",
        "Error",
    ];
    for n in NAMES {
        if let Some(rest) = message.strip_prefix(n)
            && let Some(rest) = rest.strip_prefix(": ")
        {
            return Value::Error(Box::new(ErrorData {
                name: (*n).to_string(),
                message: rest.to_string(),
            }));
        }
    }
    Value::Error(Box::new(ErrorData {
        name: "Error".to_string(),
        message: message.to_string(),
    }))
}

pub fn vm_ret(v: Value) -> Result<Value, VmErr> {
    Err(VmErr::Ret(v))
}

pub fn vm_throw(v: Value) -> Result<Value, VmErr> {
    Err(VmErr::Throw(v))
}

pub fn vm_err<T: Into<String>>(msg: T) -> Result<Value, VmErr> {
    Err(VmErr::Msg(msg.into()))
}
