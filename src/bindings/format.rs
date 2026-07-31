//! String rendering of VM values: the plain `to_string` coercer, the
//! multi-line `util.inspect`-style pretty printer, and its ANSI color
//! support. Used by `console.dir`, the NAPI return-value stringification,
//! and the async bridge's error formatting.
use std::rc::Rc;

use crate::value::Value;

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

/// Pretty, multi-line, indented rendering of a value — the sandbox-native
/// analogue of Node's `util.inspect`. Strings are single-quoted, object keys
/// that are not valid identifiers are quoted, and compound values (objects,
/// and arrays that contain objects/arrays) break across indented lines.
/// Cycle- and depth-safe by the same visited-set + `MAX_PRINT_DEPTH` scheme
/// as `to_string`.
pub fn to_string_pretty(val: &Value) -> String {
    let mut visited: std::collections::HashSet<*const ()> = std::collections::HashSet::new();
    pretty(val, &mut visited, 0, &Painter::PLAIN)
}

/// Like `to_string_pretty`, but with ANSI type colors when `colors` is on:
/// keys cyan, strings green, numbers blue, booleans yellow, and
/// `null`/`undefined`/circular markers dimmed — the same broad scheme
/// Node's `util.inspect({ colors: true })` uses.
pub fn to_string_pretty_colored(val: &Value, colors: bool) -> String {
    let mut visited: std::collections::HashSet<*const ()> = std::collections::HashSet::new();
    pretty(val, &mut visited, 0, &Painter { enabled: colors })
}

/// Whether ANSI colors should be emitted: stdout must be a TTY unless
/// `FORCE_COLOR` is set, and `NO_COLOR` always wins (https://no-color.org).
pub fn colors_enabled() -> bool {
    use std::io::IsTerminal;
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var_os("FORCE_COLOR").is_some() {
        return true;
    }
    std::io::stdout().is_terminal()
}

/// Applies ANSI SGR styles around rendered tokens. Every method is a no-op
/// when `enabled` is false, so the same rendering path serves both TTY and
/// piped/redirected output without branching at each call site.
///
/// `pub(crate)` so internal formatters render tokens with the exact same
/// palette as `console.dir`.
pub(crate) struct Painter {
    enabled: bool,
}

impl Painter {
    const PLAIN: Painter = Painter { enabled: false };

    fn wrap(&self, code: &str, s: String) -> String {
        if self.enabled {
            format!("\x1b[{}m{}\x1b[0m", code, s)
        } else {
            s
        }
    }

    pub(crate) fn key(&self, s: String) -> String {
        self.wrap("36", s) // cyan
    }
    pub(crate) fn string(&self, s: String) -> String {
        self.wrap("32", s) // green
    }
    pub(crate) fn number(&self, s: String) -> String {
        self.wrap("34", s) // blue
    }
    pub(crate) fn boolean(&self, s: String) -> String {
        self.wrap("33", s) // yellow
    }
    pub(crate) fn symbol(&self, s: String) -> String {
        self.wrap("32", s) // green
    }
    /// `undefined`, functions, promises, depth/cycle markers.
    pub(crate) fn special(&self, s: String) -> String {
        self.wrap("2;37", s) // dim gray
    }
    pub(crate) fn null(&self, s: String) -> String {
        self.wrap("1;90", s) // bold gray
    }
}

/// Quote and escape a string as a single-quoted JS literal.
pub(crate) fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        match ch {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// Render an object key bare when it is a valid JS identifier, quoted otherwise.
pub(crate) fn key_str(k: &str) -> String {
    let mut chars = k.chars();
    let valid = match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
        }
        _ => false,
    };
    if valid && !k.is_empty() {
        k.to_string()
    } else {
        quote(k)
    }
}

fn pretty(
    v: &Value,
    visited: &mut std::collections::HashSet<*const ()>,
    depth: usize,
    p: &Painter,
) -> String {
    match v {
        Value::Undefined => p.special("undefined".to_string()),
        Value::Null => p.null("null".to_string()),
        Value::Bool(b) => p.boolean(b.to_string()),
        Value::Number(n) => {
            let s = if n.fract() == 0.0 && n.abs() < 1e15 {
                format!("{:.0}", n)
            } else {
                n.to_string()
            };
            p.number(s)
        }
        Value::String(s) => p.string(quote(s)),
        Value::Object { props, .. } => {
            if depth >= MAX_PRINT_DEPTH {
                return p.special("[Object]".to_string());
            }
            let ptr = Rc::as_ptr(props) as *const ();
            if !visited.insert(ptr) {
                return p.special("[Circular]".to_string());
            }
            let borrow = props.borrow();
            if borrow.is_empty() {
                drop(borrow);
                visited.remove(&ptr);
                return "{}".to_string();
            }
            let inner = "  ".repeat(depth + 1);
            let outer = "  ".repeat(depth);
            let entries = borrow
                .iter()
                .map(|(k, vv)| {
                    format!(
                        "{}{}: {}",
                        inner,
                        p.key(key_str(k)),
                        pretty(vv, visited, depth + 1, p)
                    )
                })
                .collect::<Vec<_>>()
                .join(",\n");
            drop(borrow);
            visited.remove(&ptr);
            format!("{{\n{}\n{}}}", entries, outer)
        }
        Value::Array(i) => {
            if depth >= MAX_PRINT_DEPTH {
                return p.special("[Array]".to_string());
            }
            let ptr = Rc::as_ptr(i) as *const ();
            if !visited.insert(ptr) {
                return p.special("[Circular]".to_string());
            }
            let borrow = i.borrow();
            if borrow.is_empty() {
                drop(borrow);
                visited.remove(&ptr);
                return "[]".to_string();
            }
            // Arrays of scalars stay on one line; arrays containing compound
            // values break across indented lines (matches `util.inspect`).
            let has_compound = borrow
                .iter()
                .any(|x| matches!(x, Value::Object { .. } | Value::Array(_)));
            if !has_compound {
                let s = format!(
                    "[ {} ]",
                    borrow
                        .iter()
                        .map(|x| pretty(x, visited, depth + 1, p))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                drop(borrow);
                visited.remove(&ptr);
                return s;
            }
            let inner = "  ".repeat(depth + 1);
            let outer = "  ".repeat(depth);
            let entries = borrow
                .iter()
                .map(|x| format!("{}{}", inner, pretty(x, visited, depth + 1, p)))
                .collect::<Vec<_>>()
                .join(",\n");
            drop(borrow);
            visited.remove(&ptr);
            format!("[\n{}\n{}]", entries, outer)
        }
        Value::Function(f) => p.special(format!(
            "[Function: {}]",
            f.name.as_deref().unwrap_or("anonymous")
        )),
        Value::NativeFunction { name, .. } => p.special(format!("[Function: {} [native]]", name)),
        Value::HostFunction { name, .. } => p.special(format!("[Function: {} [native]]", name)),
        Value::GlobalObject => p.special("[object global]".to_string()),
        Value::Class(c) => p.special(format!("[class {}]", c.name)),
        Value::Promise { .. } | Value::HostPending { .. } => {
            p.special("[object Promise]".to_string())
        }
        Value::Generator { .. } => p.special("[object Generator]".to_string()),
        Value::Symbol(s) => p.symbol(format!("Symbol({})", s)),
        Value::Error(e) => p.special(e.message.clone()),
    }
}
