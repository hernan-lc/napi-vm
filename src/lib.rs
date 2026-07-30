pub mod bindings;
pub mod builtins;
pub mod error;
pub mod host;
#[cfg(feature = "inspector")]
pub mod inspector;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod value;

pub use bindings::{VM, create_vm, debug_parse, run_code};
pub use builtins::setup_builtins;
pub use error::VmErr;
pub use interpreter::{Environment, Interpreter, Module};
pub use lexer::{Lexer, Token};
pub use parser::{Expr, Parser, Statement};
pub use value::Value;
