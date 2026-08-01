//! Static knowledge about the interpreter's built-in globals. This is the
//! single source of truth shared by every frontend (WASM playground, LSP,
//! native GUIs) — completions are never re-derived per frontend.
//!
//! The live interpreter can also be probed at runtime (see the WASM layer),
//! but this catalog is what makes completion work on code that does not yet
//! parse or run, which is the common case while typing.

/// Top-level globals present in a fresh VM.
pub const GLOBALS: &[&str] = &[
    "Math",
    "JSON",
    "Object",
    "Array",
    "String",
    "Number",
    "Boolean",
    "Promise",
    "Date",
    "Symbol",
    "console",
    "Error",
    "TypeError",
    "RangeError",
    "SyntaxError",
    "ReferenceError",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "undefined",
    "NaN",
    "Infinity",
    "globalThis",
    "window",
    "self",
];

/// Language keywords offered during bare-identifier completion.
pub const KEYWORDS: &[&str] = &[
    "var",
    "let",
    "const",
    "function",
    "return",
    "if",
    "else",
    "for",
    "while",
    "do",
    "break",
    "continue",
    "switch",
    "case",
    "default",
    "try",
    "catch",
    "finally",
    "throw",
    "new",
    "typeof",
    "instanceof",
    "class",
    "extends",
    "super",
    "import",
    "export",
    "from",
    "async",
    "await",
    "yield",
    "of",
    "in",
    "this",
    "null",
    "undefined",
    "true",
    "false",
    "void",
    "delete",
    "static",
];

/// Members of a named built-in global, keyed by the receiver that precedes the
/// dot (e.g. `Math` → `["abs", "floor", …]`).
pub fn builtin_members(receiver: &str) -> Option<&'static [&'static str]> {
    Some(match receiver {
        "Math" => &[
            "abs", "floor", "ceil", "round", "sqrt", "cbrt", "pow", "min", "max", "random",
            "trunc", "sign", "log", "log2", "log10", "exp", "sin", "cos", "tan", "hypot", "PI",
            "E", "LN2", "LN10", "LOG2E", "LOG10E", "SQRT1_2", "SQRT2",
        ],
        "JSON" => &["parse", "stringify"],
        "console" => &["log", "info", "debug", "error", "warn", "dir"],
        "Object" => &["keys", "values", "entries", "assign"],
        "Array" => &["isArray", "from", "of"],
        "Number" => &[
            "isNaN",
            "isFinite",
            "isInteger",
            "MAX_SAFE_INTEGER",
            "MIN_SAFE_INTEGER",
            "EPSILON",
        ],
        "Promise" => &["resolve", "reject", "all", "race", "allSettled", "any"],
        "Date" => &["now", "parse", "UTC"],
        "Symbol" => &[
            "iterator",
            "asyncIterator",
            "toStringTag",
            "hasInstance",
            "for",
            "keyFor",
        ],
        "String" => &["fromCharCode"],
        _ => return None,
    })
}

/// Prototype methods offered for a value of a known built-in type. Used when
/// the receiver is a literal (`[1,2].`, `"x".`) or a variable whose initializer
/// reveals its type — cases where `Object.keys`-style own-key enumeration would
/// miss the prototype chain.
pub fn prototype_members(kind: ProtoKind) -> &'static [&'static str] {
    match kind {
        ProtoKind::Array => &[
            "length",
            "map",
            "filter",
            "reduce",
            "reduceRight",
            "forEach",
            "find",
            "some",
            "every",
            "push",
            "pop",
            "shift",
            "unshift",
            "join",
            "slice",
            "splice",
            "concat",
            "reverse",
            "sort",
            "flat",
            "flatMap",
            "indexOf",
            "includes",
        ],
        ProtoKind::String => &[
            "length",
            "toUpperCase",
            "toLowerCase",
            "slice",
            "substring",
            "split",
            "includes",
            "indexOf",
            "trim",
            "replace",
            "charAt",
            "startsWith",
            "endsWith",
            "repeat",
        ],
        ProtoKind::Number => &["toFixed", "toString", "valueOf"],
        ProtoKind::Promise => &["then", "catch", "finally"],
    }
}

/// A coarse built-in type used to select prototype members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtoKind {
    Array,
    String,
    Number,
    Promise,
}
