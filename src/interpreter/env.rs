use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::Value;

pub type Env = Rc<RefCell<Environment>>;

/// Frames with more bindings than this are promoted from a flat vector to a
/// hash map. Call frames (params + `this`) almost never reach the threshold,
/// so they pay no hashing and no hash-table allocation; the builtins frame
/// (dozens of names) promotes once and stays a map.
const PROMOTE_AT: usize = 16;

#[derive(Clone)]
enum Vars {
    Small(Vec<(String, Value)>),
    Large(HashMap<String, Value>),
}

impl Vars {
    fn len(&self) -> usize {
        match self {
            Vars::Small(v) => v.len(),
            Vars::Large(m) => m.len(),
        }
    }

    fn get(&self, n: &str) -> Option<&Value> {
        match self {
            Vars::Small(v) => v.iter().find(|(k, _)| k == n).map(|(_, v)| v),
            Vars::Large(m) => m.get(n),
        }
    }

    /// Update `n` in place if it is already bound in this frame. Returns the
    /// value back to the caller on a miss so it can be inserted or forwarded
    /// up the scope chain without cloning.
    fn try_set(&mut self, n: &str, v: Value) -> Result<(), Value> {
        match self {
            Vars::Small(vars) => {
                if let Some(slot) = vars.iter_mut().find(|(k, _)| k == n) {
                    slot.1 = v;
                    Ok(())
                } else {
                    Err(v)
                }
            }
            Vars::Large(map) => {
                if let Some(slot) = map.get_mut(n) {
                    *slot = v;
                    Ok(())
                } else {
                    Err(v)
                }
            }
        }
    }

    /// Move all bound values out of this frame (keys are dropped). Used by
    /// the iterative `Drop` of `Value` to tear down closure chains without
    /// recursing.
    fn drain_into(&mut self, work: &mut Vec<Value>) {
        match self {
            Vars::Small(vars) => work.extend(vars.drain(..).map(|(_, v)| v)),
            Vars::Large(map) => work.extend(map.drain().map(|(_, v)| v)),
        }
    }

    /// Bind `n` in this frame, assuming it is not already bound. Small frames
    /// are promoted to a hash map once they outgrow `PROMOTE_AT`.
    fn insert_new(&mut self, n: &str, v: Value) {
        match self {
            Vars::Small(vars) => {
                if vars.len() >= PROMOTE_AT {
                    let mut map: HashMap<String, Value> = vars.drain(..).collect();
                    map.insert(n.to_string(), v);
                    *self = Vars::Large(map);
                } else {
                    vars.push((n.to_string(), v));
                }
            }
            Vars::Large(map) => {
                map.insert(n.to_string(), v);
            }
        }
    }
}

#[derive(Clone)]
pub struct Environment {
    vars: Vars,
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
            vars: Vars::Small(Vec::new()),
            parent: None,
        }
    }

    pub fn child(p: Env) -> Self {
        Self {
            vars: Vars::Small(Vec::new()),
            parent: Some(p),
        }
    }

    /// Create a child frame from a pre-built binding list. The call fast
    /// path uses this to bind `this` + params in one allocation, with no
    /// per-parameter `RefCell` borrows or insertion scans.
    pub fn with_bindings(p: Env, vars: Vec<(String, Value)>) -> Self {
        let vars = if vars.len() > PROMOTE_AT {
            Vars::Large(vars.into_iter().collect())
        } else {
            Vars::Small(vars)
        };
        Self {
            vars,
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
        // Reuse the existing key allocation when the variable is already bound
        // (the common case in loops); only allocate on first insertion.
        if let Err(v) = self.vars.try_set(n, v) {
            self.vars.insert_new(n, v);
        }
    }

    pub fn assign(&mut self, n: &str, v: Value) -> bool {
        match self.vars.try_set(n, v) {
            Ok(()) => true,
            Err(v) => match self.parent {
                Some(ref p) => p.borrow_mut().assign(n, v),
                None => false,
            },
        }
    }

    /// Remove a binding from this frame only (does not walk the parent chain).
    /// Returns `true` if the binding existed and was removed.
    pub fn remove(&mut self, n: &str) -> bool {
        match &mut self.vars {
            Vars::Small(vars) => {
                if let Some(pos) = vars.iter().position(|(k, _)| k == n) {
                    vars.remove(pos);
                    true
                } else {
                    false
                }
            }
            Vars::Large(map) => map.remove(n).is_some(),
        }
    }

    /// Check whether a binding exists in this frame only (no parent walk).
    pub fn has(&self, n: &str) -> bool {
        self.vars.get(n).is_some()
    }

    /// Return the parent environment, if any. Used by the generator thread
    /// spawner to find the builtins frame.
    pub fn parent_env(&self) -> Option<Env> {
        self.parent.clone()
    }

    /// Iteratively drain a scope chain into `work` for the iterative `Drop`
    /// of `Value`. Walks parent frames one Rc at a time; stops at the first
    /// shared frame (shared scopes stay alive and drop themselves later).
    pub(crate) fn drain_chain(env: Env, work: &mut Vec<Value>) {
        let mut cur = Some(env);
        while let Some(e) = cur {
            match Rc::try_unwrap(e) {
                Ok(cell) => {
                    let mut env = cell.into_inner();
                    env.vars.drain_into(work);
                    cur = env.parent.take();
                }
                Err(_) => break,
            }
        }
    }

    /// Return all variable names bound in this frame (not walking the parent
    /// chain). Used by `Object.getOwnPropertyNames(window)` to enumerate
    /// globals.
    pub fn own_keys(&self) -> Vec<String> {
        match &self.vars {
            Vars::Small(v) => v.iter().map(|(k, _)| k.clone()).collect(),
            Vars::Large(m) => m.keys().cloned().collect(),
        }
    }

    /// Return all variable names reachable from this scope, walking the parent
    /// chain. Duplicates across frames are preserved (last write wins at
    /// lookup time, but the name list is a union).
    pub fn all_keys(&self) -> Vec<String> {
        let mut names = self.own_keys();
        if let Some(ref p) = self.parent {
            let parent_keys = p.borrow().all_keys();
            for k in parent_keys {
                if !names.contains(&k) {
                    names.push(k);
                }
            }
        }
        names
    }
}

#[derive(Clone)]
pub struct Module {
    pub exports: HashMap<String, Value>,
    pub default: Option<Value>,
}
