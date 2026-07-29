use std::fmt;

use crate::value::Value;

#[derive(Debug)]
pub enum VmErr {
    Ret(Value),
    Throw(String),
    Msg(String),
}

impl fmt::Display for VmErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VmErr::Msg(s) => write!(f, "{}", s),
            VmErr::Throw(s) => write!(f, "{}", s),
            VmErr::Ret(_) => write!(f, "return"),
        }
    }
}

pub fn vm_ret(v: Value) -> Result<Value, VmErr> {
    Err(VmErr::Ret(v))
}

pub fn vm_throw<T: Into<String>>(msg: T) -> Result<Value, VmErr> {
    Err(VmErr::Throw(msg.into()))
}

pub fn vm_err<T: Into<String>>(msg: T) -> Result<Value, VmErr> {
    Err(VmErr::Msg(msg.into()))
}
