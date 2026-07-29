use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::Value;

pub type Env = Rc<RefCell<Environment>>;

#[derive(Clone)]
pub struct Environment {
    vars: HashMap<String, Value>,
    parent: Option<Env>,
}

impl std::fmt::Debug for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Env({} vars)", self.vars.len())
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            parent: None,
        }
    }

    pub fn child(p: Env) -> Self {
        Self {
            vars: HashMap::new(),
            parent: Some(p),
        }
    }

    pub fn get(&self, n: &str) -> Option<Value> {
        if let Some(v) = self.vars.get(n) {
            Some(v.clone())
        } else if let Some(ref p) = self.parent {
            p.borrow().get(n)
        } else {
            None
        }
    }

    pub fn set(&mut self, n: &str, v: Value) {
        self.vars.insert(n.to_string(), v);
    }

    pub fn assign(&mut self, n: &str, v: Value) -> bool {
        if self.vars.contains_key(n) {
            self.vars.insert(n.to_string(), v);
            true
        } else if let Some(ref p) = self.parent {
            p.borrow_mut().assign(n, v)
        } else {
            false
        }
    }
}

#[derive(Clone)]
pub struct Module {
    pub exports: HashMap<String, Value>,
    pub default: Option<Value>,
}
