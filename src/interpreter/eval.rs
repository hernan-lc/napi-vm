//! Statement and expression evaluation: the two big `match` dispatchers.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::{BindKind, Env, Environment, Interpreter, Lookup, ModifyOutcome, Module};
use crate::error::{VmErr, vm_err, vm_ret, vm_throw};
use crate::parser::{
    AssignOp, ClassMember, Expr, ExprOrBlock, ForInit, ObjectProp, Statement, UnOp, VarKind,
    arrow_body_references, stmts_reference,
};
use crate::value::{ClassData, FunctionData, PromiseState, Value};

/// Convert parser-owned parameter names into interned `Rc<str>` so call-frame
/// binding is a refcount bump, not a heap allocation.
fn intern_params(params: &[String]) -> Rc<Vec<Rc<str>>> {
    Rc::new(params.iter().map(|p| Rc::from(p.as_str())).collect())
}

fn push_call_arg(args: &mut Vec<Value>, value: Value) -> Result<(), VmErr> {
    if args.len() >= crate::value::MAX_ARRAY_LEN {
        return Err(crate::value::limit_err("Maximum argument count exceeded"));
    }
    args.push(value);
    Ok(())
}

/// Whether a labeled control-flow signal targets the loop with `label`.
/// Unlabeled signals (`None`) target the innermost loop and are handled by
/// the callers directly; this only decides labeled ones.
/// Close an iterator that a `for...of` is abandoning before exhaustion.
///
/// Only generators need this today: their bodies may be suspended inside a
/// `try`, and JavaScript runs those `finally` blocks when the loop exits
/// early. Any other iterable is a plain object with no teardown to perform.
#[cfg_attr(target_arch = "wasm32", expect(unused_variables))]
fn close_iterator(iterator: &Value) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Value::Generator { inner } = iterator {
        // A generator cannot be mid-`next()` here: this runs on the same
        // thread that just returned from it, so the cell is free.
        if let Ok(mut inner) = inner.try_borrow_mut() {
            inner.close();
        }
    }
}

fn label_matches(label: &Option<String>, signal: &Option<String>) -> bool {
    match (label, signal) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

impl Interpreter {
    pub(super) fn eval_stmt(&mut self, s: &Statement) -> Result<Value, VmErr> {
        match s {
            Statement::Expr(e) => self.eval_expr(e),
            Statement::VarDecl {
                name,
                init,
                destructuring,
                kind,
            } => {
                let v = match init {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Undefined,
                };
                match kind {
                    // `var` was already hoisted to the enclosing function or
                    // program scope; this statement only assigns to it.
                    VarKind::Var => {
                        if let Some(pat) = destructuring {
                            self.destructure(pat, &v)?;
                        } else if init.is_some() {
                            self.assign_or_set_binding(name, v.clone())?;
                        } else {
                            // A bare `var a;` re-declaration must not erase an
                            // existing value: `var a = 1; var a; a` is 1.
                            // Hoisting already created the binding.
                            if self.global.borrow().get(name).is_none() {
                                self.assign_or_set_binding(name, Value::Undefined)?;
                            }
                        }
                    }
                    // `let`/`const` bind in *this* block, leaving the dead
                    // zone. Declaring here as well as in `hoist_lexical` keeps
                    // the statement correct on the paths that do not hoist.
                    VarKind::Let | VarKind::Const => {
                        let bind_kind = if matches!(kind, VarKind::Const) {
                            BindKind::Const
                        } else {
                            BindKind::Let
                        };
                        match destructuring {
                            Some(pat) => {
                                // Declare each name first so `destructure`'s
                                // writes land on bindings that already carry
                                // the right kind -- otherwise a destructured
                                // `const` would be reassignable.
                                for bound in crate::parser::pattern_names(pat) {
                                    self.declare_binding(
                                        &bound,
                                        Value::Undefined,
                                        bind_kind,
                                        false,
                                    )?;
                                }
                                self.destructure(pat, &v)?;
                            }
                            None => {
                                self.declare_binding(name, v.clone(), bind_kind, true)?;
                            }
                        }
                    }
                }
                Ok(v)
            }
            Statement::FnDecl {
                name,
                params,
                body,
                is_async,
                is_generator,
            } => {
                self.set_binding(
                    name,
                    Value::Function(Box::new(FunctionData {
                        name: Some(name.as_str().into()),
                        params: intern_params(params),
                        body: Rc::new(body.clone()),
                        closure: Some(self.global.clone()),
                        is_arrow: false,
                        is_async: *is_async,
                        is_generator: *is_generator,
                        uses_arguments: stmts_reference(body, "arguments"),
                    })),
                )?;
                Ok(Value::Undefined)
            }
            Statement::ClassDecl {
                name,
                superclass,
                body,
            } => {
                let super_cls = if let Some(sc) = superclass {
                    Some(self.eval_expr(sc)?)
                } else {
                    None
                };
                // Inheritance: the instance prototype chains to the superclass's
                // prototype so inherited methods resolve.
                let super_proto = match &super_cls {
                    Some(Value::Class(c)) => Some(c.prototype.clone()),
                    _ => None,
                };

                // Gather the constructor, instance fields, and methods.
                let mut ctor_params: Vec<String> = Vec::new();
                let mut ctor_body: Vec<Statement> = Vec::new();
                let mut instance_fields: Vec<(String, Option<Expr>)> = Vec::new();
                let mut proto_props: Vec<(String, Value)> = Vec::new();
                let mut statics: Vec<(String, Value)> =
                    vec![("name".to_string(), Value::String(name.clone()))];

                for member in body {
                    match member {
                        ClassMember::Method {
                            name: mname,
                            is_static: st,
                            params: mp,
                            body: mb,
                        } => {
                            let fn_val = Value::Function(Box::new(FunctionData {
                                name: Some(mname.as_str().into()),
                                params: intern_params(mp),
                                body: Rc::new(mb.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                                uses_arguments: stmts_reference(mb, "arguments"),
                            }));
                            if *st {
                                statics.push((mname.clone(), fn_val));
                            } else if mname == "constructor" {
                                ctor_params = mp.clone();
                                ctor_body = mb.clone();
                            } else {
                                proto_props.push((mname.clone(), fn_val));
                            }
                        }
                        ClassMember::Field {
                            name: fname,
                            is_static: st,
                            init,
                        } => {
                            if *st {
                                let init_val = match init {
                                    Some(e) => self.eval_expr(e)?,
                                    None => Value::Undefined,
                                };
                                statics.push((fname.clone(), init_val));
                            } else {
                                instance_fields.push((fname.clone(), init.clone()));
                            }
                        }
                        ClassMember::Getter {
                            name: gname,
                            is_static: st,
                            body: gb,
                        } => {
                            let getter_fn = Value::Function(Box::new(FunctionData {
                                name: Some(format!("get {}", gname).into()),
                                params: Rc::new(vec![]),
                                body: Rc::new(gb.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                                uses_arguments: stmts_reference(gb, "arguments"),
                            }));
                            if *st {
                                statics.push((gname.clone(), getter_fn));
                            } else {
                                proto_props.push((gname.clone(), getter_fn));
                            }
                        }
                        ClassMember::Setter {
                            name: sname,
                            param,
                            is_static: st,
                            body: sb,
                        } => {
                            let setter_fn = Value::Function(Box::new(FunctionData {
                                name: Some(format!("set {}", sname).into()),
                                params: Rc::new(vec![Rc::from(param.as_str())]),
                                body: Rc::new(sb.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                                uses_arguments: stmts_reference(sb, "arguments"),
                            }));
                            if *st {
                                statics.push((sname.clone(), setter_fn));
                            } else {
                                proto_props.push((sname.clone(), setter_fn));
                            }
                        }
                    }
                }

                // Desugar instance fields into `this.<field> = <init>;` statements
                // prepended to the constructor body.
                let mut full_ctor_body = Vec::new();
                for (fname, init) in instance_fields {
                    let value = init.unwrap_or(Expr::Undefined);
                    full_ctor_body.push(Statement::Expr(Expr::Assignment {
                        target: Box::new(Expr::Member {
                            object: Box::new(Expr::This),
                            property: Box::new(Expr::String(fname.clone())),
                            computed: false,
                        }),
                        op: AssignOp::Assign,
                        value: Box::new(value),
                    }));
                }
                full_ctor_body.extend(ctor_body);

                // For a derived class, expose the superclass constructor to the
                // constructor body as `__super_ctor` so `super(...)` can call it.
                let ctor_closure = match &super_cls {
                    Some(Value::Class(sc)) => {
                        let env = Rc::new(RefCell::new(Environment::child(self.global.clone())));
                        env.borrow_mut()
                            .set("__super_ctor", sc.constructor.as_ref().clone());
                        env
                    }
                    _ => self.global.clone(),
                };

                let constructor = Value::Function(Box::new(FunctionData {
                    name: Some(name.as_str().into()),
                    params: Rc::new(
                        ctor_params
                            .into_iter()
                            .map(|p| Rc::from(p.as_str()))
                            .collect(),
                    ),
                    uses_arguments: stmts_reference(&full_ctor_body, "arguments"),
                    body: Rc::new(full_ctor_body),
                    closure: Some(ctor_closure),
                    is_arrow: false,
                    is_async: false,
                    is_generator: false,
                }));

                let prototype = Value::object_with_proto(proto_props, super_proto);
                prototype.set_prop("constructor".to_string(), constructor.clone())?;

                let class_val = Value::Class(Box::new(ClassData {
                    name: name.clone(),
                    constructor: Box::new(constructor),
                    prototype: Rc::new(prototype),
                    statics: Rc::new(RefCell::new(statics)),
                }));

                self.set_binding(name, class_val)?;
                Ok(Value::Undefined)
            }
            Statement::Return(e) => {
                let v = match e {
                    Some(ex) => self.eval_expr(ex)?,
                    None => Value::Undefined,
                };
                vm_ret(v)
            }
            Statement::If { test, then, else_ } => {
                let t = self.eval_expr(test)?;
                if self.truthy(&t) {
                    self.run_block(then)
                } else if let Some(a) = else_ {
                    self.run_block(a)
                } else {
                    Ok(Value::Undefined)
                }
            }
            Statement::While { test, body } => {
                let label = self.active_label.take();
                let mut r = Value::Undefined;
                loop {
                    self.consume_loop()?;
                    let t = self.eval_expr(test)?;
                    if !self.truthy(&t) {
                        break;
                    }
                    match self.run_block(body) {
                        Err(VmErr::Break(None)) => break,
                        Err(VmErr::Break(l)) if label_matches(&label, &l) => break,
                        Err(VmErr::Continue(None)) => continue,
                        Err(VmErr::Continue(l)) if label_matches(&label, &l) => continue,
                        other => r = other?,
                    }
                }
                Ok(r)
            }
            Statement::DoWhile { test, body } => {
                let label = self.active_label.take();
                let mut r = Value::Undefined;
                loop {
                    self.consume_loop()?;
                    match self.run_block(body) {
                        Err(VmErr::Break(None)) => break,
                        Err(VmErr::Break(l)) if label_matches(&label, &l) => break,
                        Err(VmErr::Continue(None)) => {}
                        Err(VmErr::Continue(l)) if label_matches(&label, &l) => {}
                        other => r = other?,
                    }
                    let t = self.eval_expr(test)?;
                    if !self.truthy(&t) {
                        break;
                    }
                }
                Ok(r)
            }
            Statement::For {
                init,
                test,
                update,
                body,
            } => {
                // The loop head gets its own scope, so `for (let i = ...)`
                // does not leak `i` and does not collide with an outer `i`.
                let outer = self.push_scope();
                let result =
                    self.run_for(init.as_deref(), test.as_deref(), update.as_deref(), body);
                self.pop_scope(outer);
                result
            }
            Statement::ForIn { name, obj, body } => {
                let o = self.eval_expr(obj)?;
                let ks = self.keys(&o);
                let mut r = Value::Undefined;
                let label = self.active_label.take();
                for k in ks {
                    self.consume_loop()?;
                    self.set_binding(name, Value::String(k))?;
                    match self.run_block(body) {
                        Err(VmErr::Break(None)) => break,
                        Err(VmErr::Break(l)) if label_matches(&label, &l) => break,
                        Err(VmErr::Continue(None)) => continue,
                        Err(VmErr::Continue(l)) if label_matches(&label, &l) => continue,
                        other => r = other?,
                    }
                }
                Ok(r)
            }
            Statement::ForOf { name, iter, body } => {
                let source = self.eval_expr(iter)?;
                let iterator = self.iterator_for(&source)?;
                let next_fn = self.prop(&iterator, &Value::String("next".to_string()))?;
                if matches!(next_fn, Value::Undefined) {
                    return vm_err("iterator has no next() method");
                }
                let mut r = Value::Undefined;
                let label = self.active_label.take();
                // Leaving before the iterator reports `done` must close it, so
                // a suspended generator runs its `finally` blocks. Tracked here
                // and acted on at every exit, error paths included.
                let mut exhausted = false;
                loop {
                    // Account for the iterator's next call as well as the
                    // body iteration. This keeps custom/infinite iterators
                    // budgeted without eagerly collecting their output.
                    self.consume_loop()?;
                    let result = self.call_this(&next_fn, iterator.clone(), vec![])?;
                    let done = result
                        .get_prop("done")
                        .map(|v| v.is_truthy())
                        .unwrap_or(true);
                    if done {
                        exhausted = true;
                        break;
                    }
                    let value = result.get_prop("value").unwrap_or(Value::Undefined);
                    self.set_binding(name, value)?;
                    match self.run_block(body) {
                        Err(VmErr::Break(None)) => break,
                        Err(VmErr::Break(l)) if label_matches(&label, &l) => break,
                        Err(VmErr::Continue(None)) => continue,
                        Err(VmErr::Continue(l)) if label_matches(&label, &l) => continue,
                        // `return`, `throw`, or a break/continue aimed at an
                        // outer label also leaves the loop, and also closes.
                        Err(error) => {
                            close_iterator(&iterator);
                            return Err(error);
                        }
                        Ok(value) => r = value,
                    }
                }
                if !exhausted {
                    close_iterator(&iterator);
                }
                Ok(r)
            }
            Statement::Block(s) => self.run_block(s),
            // A declarator group shares the enclosing scope: no new frame.
            Statement::Declarations(s) => self.run(s),
            Statement::Labeled { label, body } => {
                // Make the label available to a directly-wrapped loop, which
                // takes it on entry.
                let prev = self.active_label.take();
                self.active_label = Some(label.clone());
                let r = self.eval_stmt(body);
                self.active_label = prev;
                match r {
                    // Consume a labeled break that escaped a non-loop body.
                    Err(VmErr::Break(Some(l))) if l == *label => Ok(Value::Undefined),
                    other => other,
                }
            }
            Statement::Break => Err(VmErr::Break(None)),
            Statement::Continue => Err(VmErr::Continue(None)),
            Statement::LabeledBreak(label) => Err(VmErr::Break(Some(label.clone()))),
            Statement::LabeledContinue(label) => Err(VmErr::Continue(Some(label.clone()))),
            Statement::Throw(e) => {
                let v = self.eval_expr(e)?;
                vm_throw(v)
            }
            Statement::Try {
                body,
                catch,
                finally,
            } => {
                // Run the body, routing thrown and runtime errors into catch.
                let body_result = self.run_block(body);

                let after_catch = match body_result {
                    Err(VmErr::Throw(val)) => self.run_catch(catch, val),
                    // Control-flow signals are not catchable.
                    Err(e @ (VmErr::Break(_) | VmErr::Continue(_))) => Err(e),
                    // Runtime errors (e.g. undeclared identifier, limit guards)
                    // are catchable as real error objects with `name`/`message`.
                    Err(VmErr::Msg(m)) => {
                        self.run_catch(catch, crate::error::error_value_from_msg(&m))
                    }
                    Err(VmErr::RuntimeError(re)) => {
                        self.run_catch(catch, crate::error::error_value_from_msg(&re.message))
                    }
                    other => other,
                };

                // finally always runs last; its own error/return takes precedence.
                if let Some(f) = finally {
                    self.run_block(f)?;
                }
                after_catch
            }
            Statement::Switch { disc, cases } => {
                let d = self.eval_expr(disc)?;
                // Every case shares one block scope: fall-through means a
                // `let` declared in one case is visible in the next.
                let outer = self.push_scope();
                let result = self.run_switch_cases(&d, cases);
                self.pop_scope(outer);
                result
            }
            Statement::ExportDefault(e) => {
                let v = self.eval_expr(e)?;
                let mn = self.cur_mod.clone().unwrap_or_default();
                let mo = self.modules.entry(mn).or_insert_with(|| Module {
                    exports: HashMap::new(),
                    default: None,
                });
                mo.default = Some(v);
                Ok(Value::Undefined)
            }
            Statement::ExportNamed {
                specifiers,
                source: _,
            } => {
                let mn = self.cur_mod.clone().unwrap_or_default();
                let mo = self.modules.entry(mn).or_insert_with(|| Module {
                    exports: HashMap::new(),
                    default: None,
                });
                for (l, e) in specifiers {
                    if let Some(v) = self.global.borrow().get(l) {
                        mo.exports.insert(e.clone(), v);
                    }
                }
                Ok(Value::Undefined)
            }
            Statement::Import {
                module,
                default,
                named,
                namespace,
            } => {
                let resolved_module = self.resolve_module_name(module);
                if let Some(md) = resolved_module
                    .as_ref()
                    .and_then(|name| self.modules.get(name))
                    .cloned()
                {
                    if let Some(d) = default {
                        let v = md.default.clone().unwrap_or(Value::Undefined);
                        self.set_binding(d, v)?;
                    }
                    for (l, i) in named {
                        let v = md.exports.get(i).cloned().unwrap_or(Value::Undefined);
                        self.set_binding(l, v)?;
                    }
                    if let Some(ns) = namespace {
                        let mut p: Vec<(String, Value)> = md
                            .exports
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        if let Some(ref def) = md.default {
                            p.push(("_default".to_string(), def.clone()));
                        }
                        self.set_binding(ns, Value::object(p))?;
                    }
                    Ok(Value::Undefined)
                } else {
                    if module.starts_with('.') && self.cur_mod.is_none() {
                        vm_err(format!(
                            "Relative import requires a module context: {}",
                            module
                        ))
                    } else {
                        vm_err(format!("Module not found: {}", module))
                    }
                }
            }
            Statement::Empty => Ok(Value::Undefined),
        }
    }

    /// Collect every value an iterable produces, for spread and rest.
    ///
    /// Bounded by the loop budget and the array-length cap, so an infinite
    /// generator raises a catchable `RangeError` instead of hanging.
    fn drain_iterable(&mut self, source: &Value) -> Result<Vec<Value>, VmErr> {
        let iterator = self.iterator_for(source)?;
        let next_fn = self.prop(&iterator, &Value::String("next".to_string()))?;
        if matches!(next_fn, Value::Undefined) {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        loop {
            self.consume_loop()?;
            let step = self.call_this(&next_fn, iterator.clone(), vec![])?;
            let done = step.get_prop("done").map(|v| v.is_truthy()).unwrap_or(true);
            if done {
                return Ok(out);
            }
            out.push(step.get_prop("value").unwrap_or(Value::Undefined));
            if out.len() > crate::value::MAX_ARRAY_LEN {
                return Err(crate::value::limit_err("Maximum array length exceeded"));
            }
        }
    }

    /// Obtain an iterator for `source`, following the `Symbol.iterator`
    /// protocol. Shared by `for...of` and `yield*`.
    fn iterator_for(&mut self, source: &Value) -> Result<Value, VmErr> {
        if matches!(source, Value::String(_)) {
            let iter_fn = self.prop(source, &Value::String("__symbol_iterator__".to_string()))?;
            return self.call_this(&iter_fn, source.clone(), vec![]);
        }
        match source {
            // A generator is its own iterator.
            Value::Generator { .. } => Ok(source.clone()),
            Value::Array(_) => {
                let iter_fn =
                    self.prop(source, &Value::String("__symbol_iterator__".to_string()))?;
                self.call_this(&iter_fn, source.clone(), vec![])
            }
            Value::Object { .. } => {
                let iter_fn =
                    self.prop(source, &Value::String("__symbol_iterator__".to_string()))?;
                if matches!(iter_fn, Value::Undefined) {
                    return vm_err("object is not iterable (no Symbol.iterator)");
                }
                self.call_this(&iter_fn, source.clone(), vec![])
            }
            _ => vm_err("for...of needs iterable"),
        }
    }

    /// The body of a C-style `for`, running inside the loop scope the caller
    /// pushed.
    ///
    /// `for (let i = ...)` gives **each iteration its own binding**, which is
    /// what makes a closure created in the body capture that iteration's
    /// value:
    ///
    /// ```js
    /// const fns = [];
    /// for (let i = 0; i < 3; i++) fns.push(() => i);
    /// fns.map(f => f());   // [0, 1, 2], not [3, 3, 3]
    /// ```
    ///
    /// The copy happens *after* the body and *before* the update, so the
    /// update advances the next iteration's binding rather than the one the
    /// body just captured. `var` keeps the single function-scoped binding, so
    /// the same loop written with `var` still yields `[3, 3, 3]`.
    fn run_for(
        &mut self,
        init: Option<&ForInit>,
        test: Option<&Expr>,
        update: Option<&Expr>,
        body: &[Statement],
    ) -> Result<Value, VmErr> {
        let loop_scope = self.global.clone();
        let mut per_iteration: Vec<String> = Vec::new();

        if let Some(init) = init {
            match init {
                ForInit::Var { kind, decls } => {
                    for (name, init) in decls {
                        let v = match init {
                            Some(e) => self.eval_expr(e)?,
                            None => Value::Undefined,
                        };
                        match kind {
                            // Hoisted to the function scope already.
                            VarKind::Var => self.assign_or_set_binding(name, v)?,
                            VarKind::Let => {
                                self.declare_binding(name, v, BindKind::Let, true)?;
                                per_iteration.push(name.clone());
                            }
                            VarKind::Const => {
                                self.declare_binding(name, v, BindKind::Const, true)?
                            }
                        }
                    }
                }
                ForInit::Expr(e) => {
                    self.eval_expr(e)?;
                }
            }
        }

        if !per_iteration.is_empty() {
            self.global = self.copy_iteration_scope(&loop_scope, &per_iteration);
        }

        let mut r = Value::Undefined;
        let label = self.active_label.take();
        loop {
            self.consume_loop()?;
            if let Some(t) = test {
                let tv = self.eval_expr(t)?;
                if !self.truthy(&tv) {
                    break;
                }
            }
            match self.run_block(body) {
                Err(VmErr::Break(None)) => break,
                Err(VmErr::Break(l)) if label_matches(&label, &l) => break,
                Err(VmErr::Continue(None)) => {}
                Err(VmErr::Continue(l)) if label_matches(&label, &l) => {}
                other => r = other?,
            }
            if !per_iteration.is_empty() {
                let current = self.global.clone();
                self.global = self.copy_iteration_scope(&current, &per_iteration);
            }
            if let Some(u) = update {
                self.eval_expr(u)?;
            }
        }
        Ok(r)
    }

    /// Build the next iteration's scope: a sibling of `from` carrying a fresh
    /// copy of each per-iteration binding's current value.
    fn copy_iteration_scope(&self, from: &Env, names: &[String]) -> Env {
        let parent = from
            .borrow()
            .parent_env()
            .unwrap_or_else(|| self.persistent_global.clone());
        let scope = Rc::new(RefCell::new(Environment::child(parent)));
        {
            let source = from.borrow();
            let mut target = scope.borrow_mut();
            for name in names {
                let value = source.get(name).unwrap_or(Value::Undefined);
                target.declare(name, value, BindKind::Let, true);
            }
        }
        scope
    }

    /// Run a `switch`'s cases inside the scope the caller already pushed.
    ///
    /// All cases share that one block scope, because fall-through means a
    /// `let` declared by one case is in scope for the next. Lexical
    /// declarations from *every* case are hoisted before any case runs, so a
    /// case that falls into a later declaration sees a dead zone rather than
    /// an outer binding.
    fn run_switch_cases(
        &mut self,
        disc: &Value,
        cases: &[crate::parser::SwitchCase],
    ) -> Result<Value, VmErr> {
        for case in cases {
            self.hoist_lexical_public(&case.body)?;
        }

        let mut r = Value::Undefined;
        let mut matched = false;
        let mut found_label = None;
        for c in cases {
            if let Some(ref t) = c.test {
                let tv = self.eval_expr(t)?;
                if self.seq(disc, &tv) {
                    matched = true;
                }
            } else {
                matched = true;
            }
            if matched {
                match self.run(&c.body) {
                    Err(VmErr::Break(l)) => match l {
                        None => break,
                        Some(label) => {
                            found_label = Some(label);
                            break;
                        }
                    },
                    Err(e) => return Err(e),
                    Ok(v) => {
                        r = v;
                    }
                }
            }
        }
        if let Some(label) = found_label {
            return Err(VmErr::Break(Some(label)));
        }
        Ok(r)
    }

    pub(crate) fn eval_expr(&mut self, e: &Expr) -> Result<Value, VmErr> {
        match e {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::String(s) => {
                if s.len() > crate::value::MAX_STRING_LEN {
                    return Err(crate::value::limit_err("Maximum string length exceeded"));
                }
                Ok(Value::String(s.clone()))
            }
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Null => Ok(Value::Null),
            Expr::Undefined => Ok(Value::Undefined),
            Expr::Identifier(n) => {
                if n == "undefined" {
                    return Ok(Value::Undefined);
                }
                match self.global.borrow().lookup(n) {
                    Lookup::Value(v) => Ok(v),
                    // Declared in this block but the declaration has not run:
                    // the temporal dead zone. JavaScript distinguishes this
                    // from an undeclared name, and so do we.
                    Lookup::Uninitialized => vm_err(format!(
                        "ReferenceError: Cannot access '{}' before initialization",
                        n
                    )),
                    Lookup::Missing => vm_err(format!("ReferenceError: {} is not defined", n)),
                }
            }
            Expr::Array(i) => {
                let mut v = Vec::new();
                for x in i {
                    match x {
                        Expr::Spread(inner) => {
                            let inner_val = self.eval_expr(inner)?;
                            match &inner_val {
                                Value::Array(arr) => {
                                    let items = arr.borrow();
                                    if v.len().saturating_add(items.len())
                                        > crate::value::MAX_ARRAY_LEN
                                    {
                                        return Err(crate::value::limit_err(
                                            "Maximum array length exceeded",
                                        ));
                                    }
                                    v.extend(items.iter().cloned());
                                }
                                Value::String(s) => {
                                    if v.len().saturating_add(s.chars().count())
                                        > crate::value::MAX_ARRAY_LEN
                                    {
                                        return Err(crate::value::limit_err(
                                            "Maximum array length exceeded",
                                        ));
                                    }
                                    v.extend(s.chars().map(|c| Value::String(c.to_string())))
                                }
                                // Anything else iterable -- a generator, an
                                // object with `Symbol.iterator` -- is drained
                                // through the iterator protocol. Silently
                                // producing nothing here made `[...gen()]`
                                // return an empty array.
                                Value::Generator { .. } | Value::Object { .. } => {
                                    let items = self.drain_iterable(&inner_val)?;
                                    if v.len().saturating_add(items.len())
                                        > crate::value::MAX_ARRAY_LEN
                                    {
                                        return Err(crate::value::limit_err(
                                            "Maximum array length exceeded",
                                        ));
                                    }
                                    v.extend(items);
                                }
                                _ => {}
                            }
                        }
                        _ => v.push(self.eval_expr(x)?),
                    }
                    if v.len() > crate::value::MAX_ARRAY_LEN {
                        return Err(crate::value::limit_err("Maximum array length exceeded"));
                    }
                }
                Value::checked_array(v)
            }
            Expr::Object(props) => {
                let mut o = Vec::new();
                for prop in props {
                    match prop {
                        ObjectProp::Shorthand(name) => {
                            let val = self.global.borrow().get(name).unwrap_or(Value::Undefined);
                            o.push((name.clone(), val));
                        }
                        ObjectProp::KeyValue(k, v) => {
                            o.push((k.clone(), self.eval_expr(v)?));
                        }
                        ObjectProp::Computed(k, v) => {
                            let key_val = self.eval_expr(k)?;
                            let key = match &key_val {
                                Value::String(s) => s.clone(),
                                Value::Number(n) => n.to_string(),
                                // Symbol keys are stored under an internal
                                // mangled name so they can be resolved later.
                                Value::Symbol(desc) => {
                                    if desc == "Symbol.iterator" {
                                        "__symbol_iterator__".to_string()
                                    } else {
                                        format!("__symbol:{}__", desc)
                                    }
                                }
                                _ => continue,
                            };
                            o.push((key, self.eval_expr(v)?));
                        }
                        ObjectProp::Method { name, params, body } => {
                            let fn_val = Value::Function(Box::new(FunctionData {
                                name: Some(name.as_str().into()),
                                params: intern_params(params),
                                body: Rc::new(body.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                                uses_arguments: stmts_reference(body, "arguments"),
                            }));
                            o.push((name.clone(), fn_val));
                        }
                        ObjectProp::Getter { name, body } => {
                            let fn_val = Value::Function(Box::new(FunctionData {
                                name: Some(format!("get {}", name).into()),
                                params: Rc::new(vec![]),
                                body: Rc::new(body.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                                uses_arguments: stmts_reference(body, "arguments"),
                            }));
                            o.push((name.clone(), fn_val));
                        }
                        ObjectProp::Setter { name, param, body } => {
                            let fn_val = Value::Function(Box::new(FunctionData {
                                name: Some(format!("set {}", name).into()),
                                params: Rc::new(vec![Rc::from(param.as_str())]),
                                body: Rc::new(body.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                                uses_arguments: stmts_reference(body, "arguments"),
                            }));
                            o.push((name.clone(), fn_val));
                        }
                        ObjectProp::Spread(expr) => {
                            let val = self.eval_expr(expr)?;
                            if let Value::Object { props: sprops, .. } = &val {
                                let props = sprops.borrow();
                                if o.len().saturating_add(props.len())
                                    > crate::value::MAX_OBJECT_PROPS
                                {
                                    return Err(crate::value::limit_err(
                                        "Maximum object property count exceeded",
                                    ));
                                }
                                o.extend(props.iter().cloned());
                            }
                        }
                    }
                    if o.len() > crate::value::MAX_OBJECT_PROPS {
                        return Err(crate::value::limit_err(
                            "Maximum object property count exceeded",
                        ));
                    }
                }
                Value::checked_object(o)
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                self.bin_op(*op, &l, &r)
            }
            Expr::Unary {
                op,
                operand,
                prefix,
            } => {
                if matches!(op, UnOp::Inc | UnOp::Dec)
                    && matches!(operand.as_ref(), Expr::Identifier(_) | Expr::Member { .. })
                {
                    match operand.as_ref() {
                        Expr::Identifier(n) => {
                            let inc = *op == UnOp::Inc;
                            // Fused read-modify-write: one `borrow_mut` + one
                            // scan instead of a read borrow followed by a
                            // separate write borrow. `old` captures the value
                            // before the update so postfix can return it.
                            let mut old = None;
                            let new_val = {
                                let mut env = self.global.borrow_mut();
                                env.modify(n, |cur| {
                                    let cur_num = self.tn(&cur);
                                    old = Some(cur);
                                    Value::Number(if inc { cur_num + 1.0 } else { cur_num - 1.0 })
                                })
                            };
                            let new_val = match new_val {
                                ModifyOutcome::Updated(v) => v,
                                ModifyOutcome::Missing => {
                                    return vm_err(format!("ReferenceError: {} is not defined", n));
                                }
                                ModifyOutcome::Const => {
                                    return vm_err(format!(
                                        "TypeError: Assignment to constant variable '{}'",
                                        n
                                    ));
                                }
                                ModifyOutcome::Uninitialized => {
                                    return vm_err(format!(
                                        "ReferenceError: Cannot access '{}' before initialization",
                                        n
                                    ));
                                }
                            };
                            if *prefix {
                                Ok(new_val)
                            } else {
                                Ok(old.unwrap_or(Value::Undefined))
                            }
                        }
                        Expr::Member {
                            object,
                            property,
                            computed: _,
                        } => {
                            let obj = self.eval_expr(object)?;
                            let prop = self.eval_expr(property)?;
                            let cur = self.prop(&obj, &prop)?;
                            let new_val = if *op == UnOp::Inc {
                                Value::Number(self.tn(&cur) + 1.0)
                            } else {
                                Value::Number(self.tn(&cur) - 1.0)
                            };
                            self.assign_member(&obj, &prop, new_val.clone())?;
                            if *prefix { Ok(new_val) } else { Ok(cur) }
                        }
                        _ => {
                            let v = self.eval_expr(operand)?;
                            self.un_op(*op, &v)
                        }
                    }
                } else if *op == UnOp::Typeof {
                    // `typeof` never throws, even on undeclared identifiers.
                    let v = if let Expr::Identifier(n) = operand.as_ref() {
                        if n == "undefined" {
                            Value::Undefined
                        } else {
                            self.global.borrow().get(n).unwrap_or(Value::Undefined)
                        }
                    } else {
                        self.eval_expr(operand)?
                    };
                    self.un_op(*op, &v)
                } else {
                    let v = self.eval_expr(operand)?;
                    self.un_op(*op, &v)
                }
            }
            Expr::Call { callee, args } => {
                let mut a = Vec::new();
                for x in args {
                    match x {
                        Expr::Spread(inner) => {
                            let inner_val = self.eval_expr(inner)?;
                            match &inner_val {
                                Value::Array(arr) => {
                                    let items = arr.borrow();
                                    if a.len().saturating_add(items.len())
                                        > crate::value::MAX_ARRAY_LEN
                                    {
                                        return Err(crate::value::limit_err(
                                            "Maximum argument count exceeded",
                                        ));
                                    }
                                    a.extend(items.iter().cloned());
                                }
                                _ => push_call_arg(&mut a, inner_val)?,
                            }
                        }
                        _ => push_call_arg(&mut a, self.eval_expr(x)?)?,
                    }
                }
                match callee.as_ref() {
                    // `super(...)` invokes the superclass constructor on the
                    // current `this`.
                    Expr::Super => {
                        let this_val = self.global.borrow().get("this").unwrap_or(Value::Undefined);
                        let super_ctor =
                            self.global.borrow().get("__super_ctor").ok_or_else(|| {
                                VmErr::Msg("super used outside a derived class".to_string())
                            })?;
                        self.invoke_ctor(&super_ctor, this_val, a)
                    }
                    // Method call: bind `this` to the receiver object.
                    Expr::Member {
                        object,
                        property,
                        computed: _,
                    } => {
                        let obj = self.eval_expr(object)?;
                        let prop = self.eval_expr(property)?;
                        let f = self.prop(&obj, &prop)?;
                        self.call_this(&f, obj, a)
                    }
                    Expr::OptionalChain {
                        object,
                        property,
                        computed: _,
                    } => {
                        let obj = self.eval_expr(object)?;
                        if matches!(obj, Value::Null | Value::Undefined) {
                            return Ok(Value::Undefined);
                        }
                        // A `Undefined` property marks an optional call `obj?.(args)`.
                        let f = if matches!(property.as_ref(), Expr::Undefined) {
                            obj.clone()
                        } else {
                            let prop = self.eval_expr(property)?;
                            self.prop(&obj, &prop)?
                        };
                        self.call_this(&f, obj, a)
                    }
                    _ => {
                        let c = self.eval_expr(callee)?;
                        self.call_this(&c, Value::Undefined, a)
                    }
                }
            }
            Expr::Member {
                object,
                property,
                computed: _,
            } => {
                let o = self.eval_expr(object)?;
                let p = self.eval_expr(property)?;
                self.get_prop_value(&o, &p)
            }
            Expr::OptionalChain {
                object,
                property,
                computed: _,
            } => {
                let o = self.eval_expr(object)?;
                if matches!(o, Value::Null | Value::Undefined) {
                    return Ok(Value::Undefined);
                }
                let p = self.eval_expr(property)?;
                self.get_prop_value(&o, &p)
            }
            Expr::Assignment { target, op, value } => {
                let v = self.eval_expr(value)?;
                match target.as_ref() {
                    Expr::Identifier(n) => {
                        let fv = if let Some(bin) = op.bin_op() {
                            // Fused read-modify-write: one `borrow_mut` + one
                            // scan instead of a read borrow then a write borrow.
                            // `bin_op` can still fail (e.g. string-length cap);
                            // capture the error and leave the slot unchanged.
                            let mut err = None;
                            let res = {
                                let mut env = self.global.borrow_mut();
                                env.modify(n, |cur| match self.bin_op(bin, &cur, &v) {
                                    Ok(new) => new,
                                    Err(e) => {
                                        err = Some(e);
                                        cur
                                    }
                                })
                            };
                            if let Some(e) = err {
                                return Err(e);
                            }
                            match res {
                                ModifyOutcome::Updated(v) => v,
                                ModifyOutcome::Missing => {
                                    return vm_err(format!("ReferenceError: {} is not defined", n));
                                }
                                ModifyOutcome::Const => {
                                    return vm_err(format!(
                                        "TypeError: Assignment to constant variable '{}'",
                                        n
                                    ));
                                }
                                ModifyOutcome::Uninitialized => {
                                    return vm_err(format!(
                                        "ReferenceError: Cannot access '{}' before initialization",
                                        n
                                    ));
                                }
                            }
                        } else {
                            self.assign_or_set_binding(n, v.clone())?;
                            v
                        };
                        Ok(fv)
                    }
                    Expr::Member {
                        object,
                        property,
                        computed: _,
                    } => {
                        let obj = self.eval_expr(object)?;
                        let prop = self.eval_expr(property)?;
                        let fv = if let Some(bin) = op.bin_op() {
                            let c = self.prop(&obj, &prop)?;
                            self.bin_op(bin, &c, &v)?
                        } else {
                            v
                        };
                        self.assign_member(&obj, &prop, fv.clone())?;
                        Ok(fv)
                    }
                    _ => vm_err("Invalid assignment target"),
                }
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
            } => {
                let t = self.eval_expr(test)?;
                if self.truthy(&t) {
                    self.eval_expr(consequent)
                } else {
                    self.eval_expr(alternate)
                }
            }
            Expr::ArrowFn { params, body } => Ok(Value::Function(Box::new(FunctionData {
                name: None,
                params: intern_params(params),
                closure: Some(self.global.clone()),
                uses_arguments: arrow_body_references(body, "arguments"),
                body: Rc::new(match body.as_ref() {
                    ExprOrBlock::Block(s) => s.clone(),
                    ExprOrBlock::Expr(e) => vec![Statement::Return(Some(e.clone()))],
                }),
                is_arrow: true,
                is_async: false,
                is_generator: false,
            }))),
            Expr::FnExpr {
                name,
                params,
                body,
                is_async,
                is_generator,
            } => Ok(Value::Function(Box::new(FunctionData {
                name: name.as_deref().map(Rc::from),
                params: intern_params(params),
                body: Rc::new(body.clone()),
                closure: Some(self.global.clone()),
                is_arrow: false,
                is_async: *is_async,
                is_generator: *is_generator,
                uses_arguments: stmts_reference(body, "arguments"),
            }))),
            Expr::New { callee, args } => {
                let mut a = Vec::new();
                for x in args {
                    push_call_arg(&mut a, self.eval_expr(x)?)?;
                }
                let c = self.eval_expr(callee)?;
                self.ctor(&c, a)
            }
            Expr::Spread(i) => self.eval_expr(i),
            Expr::This => Ok(self.global.borrow().get("this").unwrap_or(Value::Undefined)),
            Expr::ImportMeta => {
                let o = vec![
                    ("url".to_string(), Value::String("vm://module".to_string())),
                    ("main".to_string(), Value::Bool(self.is_main)),
                ];
                Ok(Value::object(o))
            }
            Expr::Template { quasis, exprs } => {
                let mut result = String::new();
                for (i, q) in quasis.iter().enumerate() {
                    if result.len().saturating_add(q.len()) > crate::value::MAX_STRING_LEN {
                        return Err(crate::value::limit_err("Maximum string length exceeded"));
                    }
                    result.push_str(q);
                    if i < exprs.len() {
                        let val = self.eval_expr(&exprs[i])?;
                        let rendered = self.vs(&val)?;
                        if result.len().saturating_add(rendered.len())
                            > crate::value::MAX_STRING_LEN
                        {
                            return Err(crate::value::limit_err("Maximum string length exceeded"));
                        }
                        result.push_str(&rendered);
                    }
                }
                Value::checked_string(result)
            }
            Expr::Super => vm_err("'super' must be called as a function"),
            Expr::Await(inner) => {
                // The promise model is eager: by the time we `await` a promise it
                // is already settled, so unwrap a fulfilled value or re-throw a
                // rejection reason. Awaiting a non-promise yields it unchanged.
                let v = self.eval_expr(inner)?;
                // Async host call: park the VM thread until the host resolves.
                if let Value::HostPending { id } = v {
                    let bridge = self.host.clone().ok_or_else(|| {
                        VmErr::Msg("cannot await host promise: no bridge attached".to_string())
                    })?;
                    return bridge.await_host(id);
                }
                if let Value::Promise { state, value } = &v {
                    let inner_val = value
                        .as_ref()
                        .map(|b| (**b).clone())
                        .unwrap_or(Value::Undefined);
                    if *state == PromiseState::Rejected {
                        vm_throw(inner_val)
                    } else {
                        Ok(inner_val)
                    }
                } else {
                    Ok(v)
                }
            }
            Expr::Yield(arg) => {
                // Evaluate the yielded expression, then switch back to whoever
                // called `next()`. Execution resumes here when `next(v)` is
                // called again, and `v` becomes the value of this expression.
                //
                // An abandoned generator is resumed once more with
                // `GenResume::Return`, so guest `finally` blocks still run.
                // Evaluated on every target: the expression may have side
                // effects even where suspension is unsupported.
                let v = match arg {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Undefined,
                };
                #[cfg(target_arch = "wasm32")]
                let _ = v;
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(yielder) = self.gen_yielder.as_ref() {
                    return match yielder.suspend(v) {
                        crate::value::GenResume::Next(sent) => Ok(sent.unwrap_or(Value::Undefined)),
                        // Abandoned: return from the body so the surrounding
                        // `try`/`finally` still runs on the way out.
                        crate::value::GenResume::Return => vm_ret(Value::Undefined),
                    };
                }
                // Outside a generator body: yield is a no-op returning undefined.
                Ok(Value::Undefined)
            }
            Expr::YieldFrom(inner) => {
                // `yield* it` re-yields every value `it` produces, then
                // evaluates to `it`'s own return value. Values sent in with
                // `next(v)` are forwarded to the delegate.
                let source = self.eval_expr(inner)?;
                let iterator = self.iterator_for(&source)?;
                let next_fn = self.prop(&iterator, &Value::String("next".to_string()))?;
                if matches!(next_fn, Value::Undefined) {
                    return vm_err("TypeError: yield* requires an iterable");
                }

                let mut sent = Value::Undefined;
                loop {
                    self.consume_loop()?;
                    let step = self.call_this(&next_fn, iterator.clone(), vec![sent])?;
                    let done = step.get_prop("done").map(|v| v.is_truthy()).unwrap_or(true);
                    let value = step.get_prop("value").unwrap_or(Value::Undefined);
                    if done {
                        // The delegate's return value is this expression's.
                        return Ok(value);
                    }

                    #[cfg(target_arch = "wasm32")]
                    {
                        let _ = value;
                        sent = Value::Undefined;
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    match self.gen_yielder.as_ref() {
                        Some(yielder) => match yielder.suspend(value) {
                            crate::value::GenResume::Next(v) => {
                                sent = v.unwrap_or(Value::Undefined);
                            }
                            crate::value::GenResume::Return => {
                                // The outer generator is being closed: close
                                // the delegate too, then unwind.
                                close_iterator(&iterator);
                                return vm_ret(Value::Undefined);
                            }
                        },
                        // Outside a generator body there is nobody to yield
                        // to; drain the iterator for its side effects.
                        None => sent = Value::Undefined,
                    }
                }
            }
        }
    }
}
