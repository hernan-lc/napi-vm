//! Scope collection over the AST: the set of names visible to a program, plus
//! just enough type-shape information (object-literal keys, array initializers)
//! to drive static member completion without executing anything.
//!
//! The walk deliberately over-approximates: because AST nodes carry no spans
//! yet, every declaration is treated as file-wide visible. That is safe for
//! completion (extra candidates are cheap; missing ones are not) and will be
//! tightened to true lexical scoping once the AST is span-annotated.

use crate::parser::{Expr, ForInit, ObjectProp, Pattern, Statement};

use super::CompletionKind;

/// The inferred shape of a variable's initializer, when it is a literal.
#[derive(Debug, Clone)]
pub enum InitShape {
    Array,
    Object(Vec<String>),
}

/// One declared name in the program.
#[derive(Debug, Clone)]
pub struct Decl {
    pub name: String,
    pub kind: CompletionKind,
    pub shape: Option<InitShape>,
}

/// Everything the completion engine needs to know about a program's scope.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    pub decls: Vec<Decl>,
    /// Local names bound via `import * as ns`.
    pub namespaces: Vec<String>,
    /// `(namespace, module)` pairs so `ns.<export>` can resolve to a module.
    pub namespace_to_module: Vec<(String, String)>,
    /// Module specifiers referenced by any `import`.
    pub modules: Vec<String>,
}

impl Scope {
    pub fn find(&self, name: &str) -> Option<&Decl> {
        self.decls.iter().find(|d| d.name == name)
    }

    /// The module a namespace import refers to, if any.
    pub fn module_for_namespace(&self, ns: &str) -> Option<&str> {
        self.namespace_to_module
            .iter()
            .find(|(n, _)| n == ns)
            .map(|(_, m)| m.as_str())
    }
}

/// Collect all declarations reachable from a program.
pub fn collect(stmts: &[Statement]) -> Scope {
    let mut scope = Scope::default();
    walk_stmts(stmts, &mut scope);
    scope
}

fn walk_stmts(stmts: &[Statement], scope: &mut Scope) {
    for s in stmts {
        walk_stmt(s, scope);
    }
}

fn push(scope: &mut Scope, name: &str, kind: CompletionKind, shape: Option<InitShape>) {
    if name.is_empty() {
        return;
    }
    if !scope.decls.iter().any(|d| d.name == name) {
        scope.decls.push(Decl {
            name: name.to_string(),
            kind,
            shape,
        });
    }
}

fn walk_stmt(s: &Statement, scope: &mut Scope) {
    match s {
        Statement::VarDecl {
            name,
            init,
            destructuring,
            ..
        } => {
            push(
                scope,
                name,
                CompletionKind::Variable,
                init_shape(init.as_deref()),
            );
            if let Some(p) = destructuring {
                for n in pattern_names(p) {
                    push(scope, &n, CompletionKind::Variable, None);
                }
            }
        }
        Statement::FnDecl {
            name, params, body, ..
        } => {
            push(scope, name, CompletionKind::Function, None);
            for p in params {
                push(scope, p, CompletionKind::Variable, None);
            }
            walk_stmts(body, scope);
        }
        Statement::ClassDecl {
            name,
            superclass,
            body,
        } => {
            push(scope, name, CompletionKind::Class, None);
            // A class body introduces no outer-scope names, but a superclass
            // expression may reference in-scope identifiers; nothing to collect.
            let _ = (superclass, body);
        }
        Statement::If {
            test: _,
            then,
            else_,
        } => {
            walk_stmts(then, scope);
            if let Some(e) = else_ {
                walk_stmts(e, scope);
            }
        }
        Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
            walk_stmts(body, scope);
        }
        Statement::For { init, body, .. } => {
            if let Some(fi) = init {
                if let ForInit::Var { decls, .. } = fi.as_ref() {
                    for (n, e) in decls {
                        push(scope, n, CompletionKind::Variable, init_shape(e.as_ref()));
                    }
                }
            }
            walk_stmts(body, scope);
        }
        Statement::ForIn { name, body, .. } | Statement::ForOf { name, body, .. } => {
            push(scope, name, CompletionKind::Variable, None);
            walk_stmts(body, scope);
        }
        Statement::Block(b) => walk_stmts(b, scope),
        Statement::Labeled { body, .. } => walk_stmt(body, scope),
        Statement::Try {
            body,
            catch,
            finally,
        } => {
            walk_stmts(body, scope);
            if let Some((param, block)) = catch {
                push(scope, param, CompletionKind::Variable, None);
                walk_stmts(block, scope);
            }
            if let Some(f) = finally {
                walk_stmts(f, scope);
            }
        }
        Statement::Switch { cases, .. } => {
            for c in cases {
                walk_stmts(&c.body, scope);
            }
        }
        Statement::Import {
            module,
            default,
            named,
            namespace,
        } => {
            scope.modules.push(module.clone());
            if let Some(d) = default {
                push(scope, d, CompletionKind::Variable, None);
            }
            for (_imported, local) in named {
                push(scope, local, CompletionKind::Variable, None);
            }
            if let Some(ns) = namespace {
                push(scope, ns, CompletionKind::Module, None);
                scope.namespaces.push(ns.clone());
                scope.namespace_to_module.push((ns.clone(), module.clone()));
            }
        }
        // Expressions, returns, throws, breaks, exports introduce no names.
        _ => {}
    }
}

/// Infer a literal shape from an initializer expression, if it is one.
fn init_shape(e: Option<&Expr>) -> Option<InitShape> {
    match e? {
        Expr::Array(_) => Some(InitShape::Array),
        Expr::Object(props) => Some(InitShape::Object(object_keys(props))),
        _ => None,
    }
}

/// Own, statically-known keys of an object literal.
pub fn object_keys(props: &[ObjectProp]) -> Vec<String> {
    let mut keys = Vec::new();
    for p in props {
        match p {
            ObjectProp::Shorthand(n)
            | ObjectProp::KeyValue(n, _)
            | ObjectProp::Method { name: n, .. }
            | ObjectProp::Getter { name: n, .. }
            | ObjectProp::Setter { name: n, .. } => keys.push(n.clone()),
            ObjectProp::Computed(_, _) | ObjectProp::Spread(_) => {}
        }
    }
    keys
}

/// All identifiers bound by a destructuring pattern.
fn pattern_names(p: &Pattern) -> Vec<String> {
    let mut out = Vec::new();
    collect_pattern(p, &mut out);
    out
}

fn collect_pattern(p: &Pattern, out: &mut Vec<String>) {
    match p {
        Pattern::Ident(n) => out.push(n.clone()),
        Pattern::Array(elems) => {
            for e in elems {
                collect_pattern(e, out);
            }
        }
        Pattern::Object(props) => {
            for (_, sub) in props {
                if let Some(sub) = sub {
                    collect_pattern(sub, out);
                }
            }
        }
        Pattern::Rest(inner) => collect_pattern(inner, out),
        Pattern::Default(inner, _) => collect_pattern(inner, out),
    }
}
