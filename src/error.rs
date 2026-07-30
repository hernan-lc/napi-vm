use std::fmt;

use crate::span::Span;
use crate::value::Value;

/// A single frame in the call stack trace.
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub name: String,
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

#[derive(Debug)]
pub enum VmErr {
    Ret(Value),
    /// A value thrown by user code (`throw expr`). Carries the original value
    /// so `catch (e)` can inspect thrown objects (e.g. `e.message`).
    Throw(Value),
    Msg(String),
    /// A runtime error with source location context.
    RuntimeError {
        message: String,
        span: Option<Span>,
        stack: Vec<StackFrame>,
    },
    /// Control-flow signal for `break`, with an optional target label. Caught
    /// by the enclosing loop/switch; not an error and not catchable by `try`.
    Break(Option<String>),
    /// Control-flow signal for `continue`, with an optional target label.
    Continue(Option<String>),
}

impl VmErr {
    /// Attach source location and call stack to a `VmErr::Msg`.
    pub fn with_context(self, span: Option<Span>, stack: &[StackFrame]) -> Self {
        match self {
            VmErr::Msg(message) => VmErr::RuntimeError {
                message,
                span,
                stack: stack.to_vec(),
            },
            other => other,
        }
    }
}

impl fmt::Display for VmErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VmErr::Msg(s) => write!(f, "{}", s),
            VmErr::RuntimeError {
                message,
                span,
                stack,
            } => {
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
    match v {
        Value::String(s) => s.clone(),
        Value::Error { message, name } => {
            if name == "Error" {
                message.clone()
            } else {
                format!("{}: {}", name, message)
            }
        }
        Value::Object { props, .. } => {
            let borrow = props.borrow();
            let get_str = |k: &str| {
                borrow.iter().find_map(|(pk, pv)| {
                    if pk == k
                        && let Value::String(s) = pv
                    {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
            };
            match (get_str("name"), get_str("message")) {
                (Some(name), Some(msg)) if name != "Error" => format!("{}: {}", name, msg),
                (_, Some(msg)) => msg,
                _ => "Uncaught error".to_string(),
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Undefined => "undefined".to_string(),
        _ => "error".to_string(),
    }
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
