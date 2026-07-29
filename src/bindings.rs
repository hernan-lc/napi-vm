use std::collections::HashMap;

use napi_derive::napi;

use crate::builtins::setup_builtins;
use crate::error::VmErr;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::Value;

pub fn to_string(val: &Value) -> String {
    fn vs(v: &Value) -> String {
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
            Value::Object { props, .. } => format!(
                "{{{}}}",
                props
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, vs(v)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Array(i) => format!(
                "[{}]",
                i.borrow().iter().map(vs).collect::<Vec<_>>().join(", ")
            ),
            Value::Function { name, .. } => {
                format!("[Function: {}]", name.as_deref().unwrap_or("anonymous"))
            }
            Value::NativeFunction { name, .. } => format!("[Function: {} [native]]", name),
            Value::Class { name, .. } => format!("[class {}]", name),
            Value::Promise { .. } => "[object Promise]".to_string(),
            Value::Generator { .. } => "[object Generator]".to_string(),
            Value::Symbol(s) => format!("Symbol({})", s),
            Value::Error { message, .. } => message.clone(),
        }
    }
    vs(val)
}

pub fn run_source(source: &str, is_main: bool) -> Result<String, VmErr> {
    let mut interp = Interpreter::new();
    interp.is_main = is_main;
    setup_builtins(&interp.global);
    let mut lex = Lexer::new(source);
    let toks = lex.tokenize();
    let mut parser = Parser::new(toks);
    let stmts = parser.parse();
    let val = interp.run(&stmts)?;
    Ok(to_string(&val))
}

#[napi]
pub struct VM {
    interp: Interpreter,
    modules: HashMap<String, String>,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl VM {
    #[napi(constructor)]
    pub fn new() -> Self {
        let interp = Interpreter::new();
        setup_builtins(&interp.global);
        Self {
            interp,
            modules: HashMap::new(),
        }
    }

    #[napi]
    pub fn run(&mut self, source: String) -> napi::Result<String> {
        let mut lex = Lexer::new(&source);
        let toks = lex.tokenize();
        let mut parser = Parser::new(toks);
        let stmts = parser.parse();
        Ok(to_string(
            &self
                .interp
                .run(&stmts)
                .map_err(|e| napi::Error::from_reason(e.to_string()))?,
        ))
    }

    #[napi]
    pub fn register_module(&mut self, name: String, source: String) -> napi::Result<()> {
        // Run the module on this VM's interpreter with `cur_mod` set so its
        // `export` statements populate `self.interp.modules[name]`, making them
        // visible to later `import` statements in the same VM. (Running it in a
        // throwaway interpreter, as before, discarded every export.)
        self.interp.cur_mod = Some(name.clone());
        let mut lex = Lexer::new(&source);
        let toks = lex.tokenize();
        let mut parser = Parser::new(toks);
        let stmts = parser.parse();
        let result = self.interp.run(&stmts);
        self.interp.cur_mod = None;
        result.map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.modules.insert(name, source);
        Ok(())
    }

    #[napi]
    pub fn set_import_meta_main(&mut self, is_main: bool) {
        self.interp.is_main = is_main;
    }

    #[napi]
    pub fn get_global(&self, name: String) -> napi::Result<String> {
        match self.interp.global.borrow().get(&name) {
            Some(val) => Ok(to_string(&val)),
            None => Ok("undefined".to_string()),
        }
    }
}

#[napi]
pub fn create_vm() -> VM {
    VM::new()
}

#[napi]
pub fn run_code(source: String) -> napi::Result<String> {
    run_source(&source, false).map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn debug_parse(source: String) -> napi::Result<String> {
    let mut lex = Lexer::new(&source);
    let toks = lex.tokenize();
    let mut parser = Parser::new(toks);
    let stmts = parser.parse();
    Ok(format!("{:?}", stmts))
}
