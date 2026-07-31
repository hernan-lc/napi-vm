// The NAPI layer is the only Node-specific part of the crate. Gating it behind
// a feature lets the pure-Rust core (lexer, parser, interpreter, builtins,
// format) build standalone — as a plain dependency for the language server and
// GUI frontends, and for the `wasm32` target.
#[cfg(feature = "napi")]
pub mod bindings;
pub mod builtins;
pub mod error;
pub mod format;
pub mod host;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod value;

#[cfg(feature = "napi")]
pub use bindings::{VM, create_vm, debug_parse, run_code};
pub use builtins::setup_builtins;
pub use error::VmErr;
pub use interpreter::{Environment, Interpreter, Module};
pub use lexer::{Lexer, Token};
pub use parser::{Expr, Parser, Statement};
pub use value::Value;
