use crate::error::VmErr;
use crate::interpreter::Env;
use crate::parser::Statement;

#[derive(Debug, Clone)]
pub enum Value {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Object(Vec<(String, Value)>),
    Array(Vec<Value>),
    Function {
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Statement>,
        closure: Option<Env>,
    },
    NativeFunction {
        name: String,
        callable: fn(Vec<Value>) -> Result<Value, VmErr>,
    },
}
