pub mod error;
pub mod value;
pub mod lexer;
pub mod parser;
pub mod interpreter;
pub mod builtins;
pub mod bindings;

pub use error::VmErr;
pub use value::Value;
pub use lexer::{Lexer, Token};
pub use parser::{Parser, Expr, Statement};
pub use interpreter::{Interpreter, Environment, Module};
pub use builtins::setup_builtins;
pub use bindings::{VM, create_vm, run_code, debug_parse};
