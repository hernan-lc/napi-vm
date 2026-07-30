//! The foldable tree model: a lazy, cycle-aware view over a live guest
//! `Value`, plus row rendering that reuses the `console.dir` palette.
//!
//! Because this walks the interpreter's `Value` directly (objects and arrays
//! are `Rc<RefCell<..>>`), circular guest structures are detected by pointer
//! identity and rendered as `[Circular *n]` — something the TypeScript
//! inspector cannot do, since cycles do not survive NAPI marshalling.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::bindings::format::{Painter, key_str, quote};
use crate::value::Value;

/// One node in the inspector tree. Children are built lazily on first expand.
struct Node {
    /// Property name / index; `None` only for the root.
    key: Option<String>,
    value: Value,
    parent: Option<usize>,
    depth: usize,
    expanded: bool,
    /// `None` = children not built yet; `Some` = built (possibly empty).
    children: Option<Vec<usize>>,
    /// If this node's container aliases an ancestor, the shared circular id.
    circular: Option<u32>,
}

/// Pointer identity of a container value, used for cycle detection.
fn container_ptr(v: &Value) -> Option<*const ()> {
    match v {
        Value::Object { props, .. } => Some(Rc::as_ptr(props) as *const ()),
        Value::Array(items) => Some(Rc::as_ptr(items) as *const ()),
        _ => None,
    }
}

fn fmt_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{:.0}", n)
    } else {
        n.to_string()
    }
}

pub struct Tree {
    nodes: Vec<Node>,
    root: usize,
    circular_ids: HashMap<*const (), u32>,
    next_circular: u32,
}

impl Tree {
    /// Build a tree rooted at `value`, collapsed: the first frame shows only
    /// the root row (`▶ …`) and the user opens what they need. Children are
    /// built lazily on first expand.
    pub fn new(value: Value) -> Tree {
        let mut t = Tree {
            nodes: Vec::new(),
            root: 0,
            circular_ids: HashMap::new(),
            next_circular: 1,
        };
        t.nodes.push(Node {
            key: None,
            value,
            parent: None,
            depth: 0,
            expanded: false,
            children: None,
            circular: None,
        });
        t
    }

    /// Container values (non-empty objects/arrays) that are not a circular
    /// back-reference can be expanded.
    pub fn is_expandable(&self, idx: usize) -> bool {
        let n = &self.nodes[idx];
        if n.circular.is_some() {
            return false;
        }
        match &n.value {
            Value::Object { props, .. } => !props.borrow().is_empty(),
            Value::Array(items) => !items.borrow().is_empty(),
            _ => false,
        }
    }

    pub fn is_expanded(&self, idx: usize) -> bool {
        self.nodes[idx].expanded
    }

    /// Set of container pointers from `idx` up to the root (inclusive).
    fn ancestor_ptrs(&self, mut idx: usize) -> HashSet<*const ()> {
        let mut set = HashSet::new();
        loop {
            if let Some(ptr) = container_ptr(&self.nodes[idx].value) {
                set.insert(ptr);
            }
            match self.nodes[idx].parent {
                Some(p) => idx = p,
                None => break,
            }
        }
        set
    }

    /// Create a node and return its index. `ancestors` is the parent's chain,
    /// used to flag circular back-references.
    fn new_node(&mut self, key: Option<String>, value: Value, parent: usize, ancestors: &HashSet<*const ()>) -> usize {
        let depth = self.nodes[parent].depth + 1;
        let mut circular = None;
        if let Some(ptr) = container_ptr(&value)
            && ancestors.contains(&ptr)
        {
            let next = &mut self.next_circular;
            let id = *self.circular_ids.entry(ptr).or_insert_with(|| {
                let id = *next;
                *next += 1;
                id
            });
            circular = Some(id);
        }
        let idx = self.nodes.len();
        self.nodes.push(Node {
            key,
            value,
            parent: Some(parent),
            depth,
            expanded: false,
            children: None,
            circular,
        });
        idx
    }

    /// Build `idx`'s children if not already built.
    pub fn ensure_children(&mut self, idx: usize) {
        if self.nodes[idx].children.is_some() {
            return;
        }
        let ancestors = self.ancestor_ptrs(idx);
        // Snapshot the (key, value) pairs first so we are not borrowing the
        // node's value while pushing child nodes.
        let pairs: Vec<(String, Value)> = match &self.nodes[idx].value {
            Value::Object { props, .. } => props
                .borrow()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            Value::Array(items) => items
                .borrow()
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v.clone()))
                .collect(),
            _ => Vec::new(),
        };
        let mut child_idxs = Vec::with_capacity(pairs.len());
        for (k, v) in pairs {
            child_idxs.push(self.new_node(Some(k), v, idx, &ancestors));
        }
        self.nodes[idx].children = Some(child_idxs);
    }

    /// Expand or collapse a node (no-op when it is not expandable).
    pub fn toggle(&mut self, idx: usize, want: bool) {
        if !self.is_expandable(idx) {
            return;
        }
        self.ensure_children(idx);
        self.nodes[idx].expanded = want;
    }

    /// Expand every expandable node whose depth is below `max_depth`, leaving
    /// deeper containers collapsed. Used by the static (non-TTY) dump so it
    /// shows a bounded prefix of the tree with `▶` hints for the rest.
    pub fn expand_to_depth(&mut self, max_depth: usize) {
        let mut stack = vec![self.root];
        while let Some(idx) = stack.pop() {
            if self.nodes[idx].depth < max_depth && self.is_expandable(idx) {
                self.ensure_children(idx);
                self.nodes[idx].expanded = true;
                if let Some(children) = self.nodes[idx].children.clone() {
                    stack.extend(children);
                }
            }
        }
    }

    /// Pre-order list of currently visible node indices (root first, then the
    /// children of each expanded node).
    pub fn visible_rows(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![self.root];
        while let Some(idx) = stack.pop() {
            out.push(idx);
            if self.nodes[idx].expanded
                && let Some(children) = &self.nodes[idx].children
            {
                for &c in children.iter().rev() {
                    stack.push(c);
                }
            }
        }
        out
    }

    /// A short label for a node: its key, or a type name for the root.
    pub fn label_of(&self, idx: usize) -> String {
        if let Some(k) = &self.nodes[idx].key {
            return k.clone();
        }
        match &self.nodes[idx].value {
            Value::Array(items) => format!("Array({})", items.borrow().len()),
            Value::Object { props, .. } => {
                let n = props.borrow().len();
                if n == 0 {
                    "Object {}".to_string()
                } else {
                    format!("Object {{ {} }}", n)
                }
            }
            other => crate::bindings::to_string(other),
        }
    }

    /// One-line hint shown after a collapsed compound node.
    fn header(&self, idx: usize) -> String {
        match &self.nodes[idx].value {
            Value::Array(items) => format!("[ {} ]", items.borrow().len()),
            Value::Object { .. } => "{…}".to_string(),
            _ => String::new(),
        }
    }

    /// Render a single tree row: indent, fold arrow, colored key, colored value.
    pub fn render_row(&self, idx: usize, p: &Painter) -> String {
        let n = &self.nodes[idx];
        let indent = "  ".repeat(n.depth);
        let arrow = if self.is_expandable(idx) {
            if n.expanded {
                "▼ "
            } else {
                "▶ "
            }
        } else {
            "  "
        };
        let key = match &n.key {
            Some(k) => format!("{}: ", p.key(key_str(k))),
            None => String::new(),
        };

        let val = if let Some(id) = n.circular {
            p.dim(format!("[Circular *{}]", id))
        } else if n.key.is_none() {
            p.bold(self.label_of(idx))
        } else {
            match &n.value {
                Value::Undefined => p.special("undefined".to_string()),
                Value::Null => p.null("null".to_string()),
                Value::Bool(b) => p.boolean(b.to_string()),
                Value::Number(num) => p.number(fmt_number(*num)),
                Value::String(s) => p.string(quote(s)),
                Value::Symbol(s) => p.symbol(format!("Symbol({})", s)),
                Value::Function(f) => p.special(format!(
                    "[Function: {}]",
                    f.name.as_deref().unwrap_or("anonymous")
                )),
                Value::NativeFunction { name, .. } | Value::HostFunction { name, .. } => {
                    p.special(format!("[Function: {} [native]]", name))
                }
                Value::Class(c) => p.special(format!("[class {}]", c.name)),
                Value::Promise { .. } | Value::HostPending { .. } => {
                    p.special("[object Promise]".to_string())
                }
                Value::Generator { .. } => p.special("[object Generator]".to_string()),
                Value::GlobalObject => p.special("[object global]".to_string()),
                Value::Error(e) => p.special(e.message.clone()),
                Value::Object { props, .. } => {
                    if n.expanded {
                        if props.borrow().is_empty() {
                            p.dim("{}".to_string())
                        } else {
                            String::new()
                        }
                    } else {
                        p.dim(self.header(idx))
                    }
                }
                Value::Array(items) => {
                    if n.expanded {
                        if items.borrow().is_empty() {
                            p.dim("[]".to_string())
                        } else {
                            String::new()
                        }
                    } else {
                        p.dim(self.header(idx))
                    }
                }
            }
        };

        format!("{}{}{}{}", indent, arrow, key, val)
    }
}
