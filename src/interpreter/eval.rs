//! Statement and expression evaluation: the two big `match` dispatchers.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::{Environment, Interpreter, Module};
use crate::error::{VmErr, vm_err, vm_ret, vm_throw};
use crate::parser::{ClassMember, Expr, ExprOrBlock, ForInit, ObjectProp, Statement};
use crate::value::{PromiseState, Value};

fn is_label_break(label: &Option<String>, m: &str) -> bool {
    matches!(label, Some(l) if m == format!("__BREAK__:{}", l))
}

fn is_label_continue(label: &Option<String>, m: &str) -> bool {
    matches!(label, Some(l) if m == format!("__CONTINUE__:{}", l))
}

impl Interpreter {
    pub(super) fn eval_stmt(&mut self, s: &Statement) -> Result<Value, VmErr> {
        match s {
            Statement::Expr(e) => self.eval_expr(e),
            Statement::VarDecl {
                name,
                init,
                destructuring,
                kind: _,
            } => {
                let v = match init {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Undefined,
                };
                if let Some(pat) = destructuring {
                    self.destructure(pat, &v)?;
                } else {
                    self.global.borrow_mut().set(name, v.clone());
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
                self.global.borrow_mut().set(
                    name,
                    Value::Function {
                        name: Some(name.clone()),
                        params: Rc::new(params.clone()),
                        body: Rc::new(body.clone()),
                        closure: Some(self.global.clone()),
                        is_arrow: false,
                        is_async: *is_async,
                        is_generator: *is_generator,
                    },
                );
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
                    Some(Value::Class { prototype, .. }) => Some(prototype.clone()),
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
                            let fn_val = Value::Function {
                                name: Some(mname.clone()),
                                params: Rc::new(mp.clone()),
                                body: Rc::new(mb.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                            };
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
                            let getter_fn = Value::Function {
                                name: Some(format!("get {}", gname)),
                                params: Rc::new(vec![]),
                                body: Rc::new(gb.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                            };
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
                            let setter_fn = Value::Function {
                                name: Some(format!("set {}", sname)),
                                params: Rc::new(vec![param.clone()]),
                                body: Rc::new(sb.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                            };
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
                        op: "=".to_string(),
                        value: Box::new(value),
                    }));
                }
                full_ctor_body.extend(ctor_body);

                // For a derived class, expose the superclass constructor to the
                // constructor body as `__super_ctor` so `super(...)` can call it.
                let ctor_closure = match &super_cls {
                    Some(Value::Class {
                        constructor: super_ctor,
                        ..
                    }) => {
                        let env = Rc::new(RefCell::new(Environment::child(self.global.clone())));
                        env.borrow_mut()
                            .set("__super_ctor", super_ctor.as_ref().clone());
                        env
                    }
                    _ => self.global.clone(),
                };

                let constructor = Value::Function {
                    name: Some(name.clone()),
                    params: Rc::new(ctor_params),
                    body: Rc::new(full_ctor_body),
                    closure: Some(ctor_closure),
                    is_arrow: false,
                    is_async: false,
                    is_generator: false,
                };

                let prototype = Value::object_with_proto(proto_props, super_proto);
                prototype.set_prop("constructor".to_string(), constructor.clone());

                let class_val = Value::Class {
                    name: name.clone(),
                    constructor: Box::new(constructor),
                    prototype: Rc::new(prototype),
                    statics: Rc::new(RefCell::new(statics)),
                    superclass: super_cls.map(Box::new),
                };

                self.global.borrow_mut().set(name, class_val);
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
                    self.run(then)
                } else if let Some(a) = else_ {
                    self.run(a)
                } else {
                    Ok(Value::Undefined)
                }
            }
            Statement::While { test, body } => {
                let label = self.active_label.take();
                let mut r = Value::Undefined;
                loop {
                    let t = self.eval_expr(test)?;
                    if !self.truthy(&t) {
                        break;
                    }
                    match self.run(body) {
                        Err(VmErr::Msg(m)) if m == "__BREAK__" || is_label_break(&label, &m) => {
                            break;
                        }
                        Err(VmErr::Msg(m))
                            if m == "__CONTINUE__" || is_label_continue(&label, &m) =>
                        {
                            continue;
                        }
                        other => r = other?,
                    }
                }
                Ok(r)
            }
            Statement::DoWhile { test, body } => {
                let label = self.active_label.take();
                let mut r = Value::Undefined;
                loop {
                    match self.run(body) {
                        Err(VmErr::Msg(m)) if m == "__BREAK__" || is_label_break(&label, &m) => {
                            break;
                        }
                        Err(VmErr::Msg(m))
                            if m == "__CONTINUE__" || is_label_continue(&label, &m) => {}
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
                if let Some(i) = init {
                    match i.as_ref() {
                        ForInit::Var { decls, .. } => {
                            for (name, init) in decls {
                                let v = match init {
                                    Some(e) => self.eval_expr(e)?,
                                    None => Value::Undefined,
                                };
                                self.global.borrow_mut().set(name, v);
                            }
                        }
                        ForInit::Expr(e) => {
                            self.eval_expr(e)?;
                        }
                    }
                }
                let mut r = Value::Undefined;
                let label = self.active_label.take();
                loop {
                    if let Some(t) = test {
                        let tv = self.eval_expr(t)?;
                        if !self.truthy(&tv) {
                            break;
                        }
                    }
                    match self.run(body) {
                        Err(VmErr::Msg(m)) if m == "__BREAK__" || is_label_break(&label, &m) => {
                            break;
                        }
                        Err(VmErr::Msg(m))
                            if m == "__CONTINUE__" || is_label_continue(&label, &m) => {}
                        other => r = other?,
                    }
                    if let Some(u) = update {
                        self.eval_expr(u)?;
                    }
                }
                Ok(r)
            }
            Statement::ForIn { name, obj, body } => {
                let o = self.eval_expr(obj)?;
                let ks = self.keys(&o);
                let mut r = Value::Undefined;
                let label = self.active_label.take();
                for k in ks {
                    self.global.borrow_mut().set(name, Value::String(k));
                    match self.run(body) {
                        Err(VmErr::Msg(m)) if m == "__BREAK__" || is_label_break(&label, &m) => {
                            break;
                        }
                        Err(VmErr::Msg(m))
                            if m == "__CONTINUE__" || is_label_continue(&label, &m) =>
                        {
                            continue;
                        }
                        other => r = other?,
                    }
                }
                Ok(r)
            }
            Statement::ForOf { name, iter, body } => {
                let a = self.eval_expr(iter)?;
                let items: Vec<Value> = match &a {
                    Value::Array(i) => i.borrow().clone(),
                    Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
                    Value::Generator { .. } => {
                        // Drive the generator via its `next()` until done.
                        let next_fn = self.prop(&a, &Value::String("next".to_string()))?;
                        let mut out = Vec::new();
                        loop {
                            let r = self.call_this(&next_fn, a.clone(), vec![])?;
                            let done = r.get_prop("done").map(|v| v.is_truthy()).unwrap_or(true);
                            let val = r.get_prop("value").unwrap_or(Value::Undefined);
                            if done {
                                break;
                            }
                            out.push(val);
                        }
                        out
                    }
                    // Full iterator protocol: check for a `[Symbol.iterator]()`
                    // method on arbitrary objects. If present, call it to obtain
                    // an iterator, then drive it with `next()`.
                    Value::Object { .. } => {
                        let iter_fn =
                            self.prop(&a, &Value::String("__symbol_iterator__".to_string()))?;
                        if matches!(iter_fn, Value::Undefined) {
                            return vm_err("object is not iterable (no Symbol.iterator)");
                        }
                        let iterator = self.call_this(&iter_fn, a.clone(), vec![])?;
                        self.drain_iterator(&iterator)?
                    }
                    _ => return vm_err("for...of needs iterable"),
                };
                let mut r = Value::Undefined;
                let label = self.active_label.take();
                for i in items {
                    self.global.borrow_mut().set(name, i);
                    match self.run(body) {
                        Err(VmErr::Msg(m)) if m == "__BREAK__" || is_label_break(&label, &m) => {
                            break;
                        }
                        Err(VmErr::Msg(m))
                            if m == "__CONTINUE__" || is_label_continue(&label, &m) =>
                        {
                            continue;
                        }
                        other => r = other?,
                    }
                }
                Ok(r)
            }
            Statement::Block(s) => self.run(s),
            Statement::Labeled { label, body } => {
                // Make the label available to a directly-wrapped loop, which
                // takes it on entry.
                let prev = self.active_label.take();
                self.active_label = Some(label.clone());
                let r = self.eval_stmt(body);
                self.active_label = prev;
                match r {
                    // Consume a labeled break that escaped a non-loop body.
                    Err(VmErr::Msg(m)) if m == format!("__BREAK__:{}", label) => {
                        Ok(Value::Undefined)
                    }
                    other => other,
                }
            }
            Statement::Break => vm_err("__BREAK__"),
            Statement::Continue => vm_err("__CONTINUE__"),
            Statement::LabeledBreak(label) => vm_err(format!("__BREAK__:{}", label)),
            Statement::LabeledContinue(label) => vm_err(format!("__CONTINUE__:{}", label)),
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
                let body_result = self.run(body);

                let after_catch = match body_result {
                    Err(VmErr::Throw(val)) => self.run_catch(catch, val),
                    // Control-flow signals are not catchable.
                    Err(VmErr::Msg(m))
                        if m.starts_with("__BREAK__") || m.starts_with("__CONTINUE__") =>
                    {
                        Err(VmErr::Msg(m))
                    }
                    // Runtime errors (e.g. undeclared identifier) are catchable.
                    Err(VmErr::Msg(m)) => self.run_catch(catch, Value::String(m)),
                    other => other,
                };

                // finally always runs last; its own error/return takes precedence.
                if let Some(f) = finally {
                    self.run(f)?;
                }
                after_catch
            }
            Statement::Switch { disc, cases } => {
                let d = self.eval_expr(disc)?;
                let mut r = Value::Undefined;
                let mut m = false;
                let mut found_label = None;
                for c in cases {
                    if let Some(ref t) = c.test {
                        let tv = self.eval_expr(t)?;
                        if self.seq(&d, &tv) {
                            m = true;
                        }
                    } else {
                        m = true;
                    }
                    if m {
                        match self.run(&c.body) {
                            Err(e) => {
                                let s = format!("{}", e);
                                if let Some(label) = s.strip_prefix("__BREAK__:") {
                                    found_label = Some(label.to_string());
                                    break;
                                } else if s == "__BREAK__" {
                                    break;
                                } else {
                                    return Err(e);
                                }
                            }
                            Ok(v) => {
                                r = v;
                            }
                        }
                    }
                }
                if let Some(label) = found_label {
                    return vm_err(format!("__BREAK__:{}", label));
                }
                Ok(r)
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
                if let Some(md) = self.modules.get(module) {
                    if let Some(d) = default {
                        let v = md.default.clone().unwrap_or(Value::Undefined);
                        self.global.borrow_mut().set(d, v);
                    }
                    for (l, i) in named {
                        let v = md.exports.get(i).cloned().unwrap_or(Value::Undefined);
                        self.global.borrow_mut().set(l, v);
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
                        self.global.borrow_mut().set(ns, Value::object(p));
                    }
                    Ok(Value::Undefined)
                } else {
                    vm_err(format!("Module not found: {}", module))
                }
            }
            Statement::Empty => Ok(Value::Undefined),
        }
    }

    pub(crate) fn eval_expr(&mut self, e: &Expr) -> Result<Value, VmErr> {
        match e {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Null => Ok(Value::Null),
            Expr::Undefined => Ok(Value::Undefined),
            Expr::Identifier(n) => {
                if n == "undefined" {
                    return Ok(Value::Undefined);
                }
                self.global
                    .borrow()
                    .get(n)
                    .ok_or_else(|| VmErr::Msg(format!("Undefined: {}", n)))
            }
            Expr::Array(i) => {
                let mut v = Vec::new();
                for x in i {
                    match x {
                        Expr::Spread(inner) => {
                            let inner_val = self.eval_expr(inner)?;
                            match inner_val {
                                Value::Array(arr) => v.extend(arr.borrow().iter().cloned()),
                                Value::String(s) => {
                                    v.extend(s.chars().map(|c| Value::String(c.to_string())))
                                }
                                _ => {}
                            }
                        }
                        _ => v.push(self.eval_expr(x)?),
                    }
                }
                Ok(Value::array(v))
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
                            let fn_val = Value::Function {
                                name: Some(name.clone()),
                                params: Rc::new(params.clone()),
                                body: Rc::new(body.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                            };
                            o.push((name.clone(), fn_val));
                        }
                        ObjectProp::Getter { name, body } => {
                            let fn_val = Value::Function {
                                name: Some(format!("get {}", name)),
                                params: Rc::new(vec![]),
                                body: Rc::new(body.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                            };
                            o.push((name.clone(), fn_val));
                        }
                        ObjectProp::Setter { name, param, body } => {
                            let fn_val = Value::Function {
                                name: Some(format!("set {}", name)),
                                params: Rc::new(vec![param.clone()]),
                                body: Rc::new(body.clone()),
                                closure: Some(self.global.clone()),
                                is_arrow: false,
                                is_async: false,
                                is_generator: false,
                            };
                            o.push((name.clone(), fn_val));
                        }
                        ObjectProp::Spread(expr) => {
                            let val = self.eval_expr(expr)?;
                            if let Value::Object { props: sprops, .. } = val {
                                o.extend(sprops.borrow().iter().cloned());
                            }
                        }
                    }
                }
                Ok(Value::object(o))
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                self.bin_op(op, &l, &r)
            }
            Expr::Unary {
                op,
                operand,
                prefix,
            } => {
                if (op == "++" || op == "--")
                    && matches!(operand.as_ref(), Expr::Identifier(_) | Expr::Member { .. })
                {
                    match operand.as_ref() {
                        Expr::Identifier(n) => {
                            let (cur, new_val) = {
                                let env = self.global.borrow();
                                let cur = env
                                    .get(n)
                                    .ok_or_else(|| VmErr::Msg(format!("Undefined: {}", n)))?;
                                let nv = if op == "++" {
                                    Value::Number(self.tn(&cur) + 1.0)
                                } else {
                                    Value::Number(self.tn(&cur) - 1.0)
                                };
                                (cur, nv)
                            };
                            self.global.borrow_mut().assign(n, new_val.clone());
                            if *prefix { Ok(new_val) } else { Ok(cur) }
                        }
                        Expr::Member {
                            object,
                            property,
                            computed: _,
                        } => {
                            let obj = self.eval_expr(object)?;
                            let prop = self.eval_expr(property)?;
                            let cur = self.prop(&obj, &prop)?;
                            let new_val = if op == "++" {
                                Value::Number(self.tn(&cur) + 1.0)
                            } else {
                                Value::Number(self.tn(&cur) - 1.0)
                            };
                            self.assign_member(&obj, &prop, new_val.clone())?;
                            if *prefix { Ok(new_val) } else { Ok(cur) }
                        }
                        _ => {
                            let v = self.eval_expr(operand)?;
                            self.un_op(op, &v)
                        }
                    }
                } else if op == "typeof" {
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
                    self.un_op(op, &v)
                } else {
                    let v = self.eval_expr(operand)?;
                    self.un_op(op, &v)
                }
            }
            Expr::Call { callee, args } => {
                let mut a = Vec::new();
                for x in args {
                    match x {
                        Expr::Spread(inner) => {
                            let inner_val = self.eval_expr(inner)?;
                            match inner_val {
                                Value::Array(arr) => a.extend(arr.borrow().iter().cloned()),
                                _ => a.push(inner_val),
                            }
                        }
                        _ => a.push(self.eval_expr(x)?),
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
                        let fv = if *op != "=" {
                            let c = self
                                .global
                                .borrow()
                                .get(n)
                                .ok_or_else(|| VmErr::Msg(format!("Undefined: {}", n)))?;
                            let bin_op = op.trim_end_matches('=');
                            self.bin_op(bin_op, &c, &v)?
                        } else {
                            v
                        };
                        if !self.global.borrow_mut().assign(n, fv.clone()) {
                            self.global.borrow_mut().set(n, fv.clone());
                        }
                        Ok(fv)
                    }
                    Expr::Member {
                        object,
                        property,
                        computed: _,
                    } => {
                        let obj = self.eval_expr(object)?;
                        let prop = self.eval_expr(property)?;
                        let fv = if *op != "=" {
                            let c = self.prop(&obj, &prop)?;
                            let bin_op = op.trim_end_matches('=');
                            self.bin_op(bin_op, &c, &v)?
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
            Expr::ArrowFn { params, body } => Ok(Value::Function {
                name: None,
                params: Rc::new(params.clone()),
                closure: Some(self.global.clone()),
                body: Rc::new(match body.as_ref() {
                    ExprOrBlock::Block(s) => s.clone(),
                    ExprOrBlock::Expr(e) => vec![Statement::Return(Some(e.clone()))],
                }),
                is_arrow: true,
                is_async: false,
                is_generator: false,
            }),
            Expr::FnExpr {
                name,
                params,
                body,
                is_async,
                is_generator,
            } => Ok(Value::Function {
                name: name.clone(),
                params: Rc::new(params.clone()),
                body: Rc::new(body.clone()),
                closure: Some(self.global.clone()),
                is_arrow: false,
                is_async: *is_async,
                is_generator: *is_generator,
            }),
            Expr::New { callee, args } => {
                let mut a = Vec::new();
                for x in args {
                    a.push(self.eval_expr(x)?);
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
                    result.push_str(q);
                    if i < exprs.len() {
                        let val = self.eval_expr(&exprs[i])?;
                        result.push_str(&self.vs(&val));
                    }
                }
                Ok(Value::String(result))
            }
            Expr::Super => vm_err("'super' must be called as a function"),
            Expr::Await(inner) => {
                // The promise model is eager: by the time we `await` a promise it
                // is already settled, so unwrap a fulfilled value or re-throw a
                // rejection reason. Awaiting a non-promise yields it unchanged.
                let v = self.eval_expr(inner)?;
                if let Value::Promise { state, value } = v {
                    let inner_val = value.map(|b| *b).unwrap_or(Value::Undefined);
                    if state == PromiseState::Rejected {
                        vm_throw(inner_val)
                    } else {
                        Ok(inner_val)
                    }
                } else {
                    Ok(v)
                }
            }
            Expr::Yield(arg) => {
                // Evaluate the yielded expression, send it to the main thread
                // via the generator channel, then block until resumed. The
                // resume signal may carry a value (from `next(val)`) which
                // becomes the result of the `yield` expression.
                let v = match arg {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Undefined,
                };
                if let Some(chan) = self.gen_channel.as_ref() {
                    use crate::value::{GenResume, GenYield};
                    chan.to_main
                        .send(GenYield::Yielded(v))
                        .map_err(|_| VmErr::Msg("generator receiver dropped".to_string()))?;
                    // Block until the main thread calls next() again.
                    match chan.from_main.recv() {
                        Ok(GenResume::Next(sent)) => Ok(sent.unwrap_or(Value::Undefined)),
                        Err(_) => {
                            // Main thread dropped the generator; stop execution.
                            vm_ret(Value::Undefined)
                        }
                    }
                } else {
                    // Outside a generator body: yield is a no-op returning undefined.
                    Ok(Value::Undefined)
                }
            }
        }
    }

    /// Drive an iterator object (anything with a `next()` method returning
    /// `{value, done}`) to completion, collecting all yielded values.
    pub(crate) fn drain_iterator(&mut self, iterator: &Value) -> Result<Vec<Value>, VmErr> {
        let next_fn = self.prop(iterator, &Value::String("next".to_string()))?;
        if matches!(next_fn, Value::Undefined) {
            return Err(VmErr::Msg("iterator has no next() method".to_string()));
        }
        let mut out = Vec::new();
        loop {
            let r = self.call_this(&next_fn, iterator.clone(), vec![])?;
            let done = r.get_prop("done").map(|v| v.is_truthy()).unwrap_or(true);
            let val = r.get_prop("value").unwrap_or(Value::Undefined);
            if done {
                break;
            }
            out.push(val);
        }
        Ok(out)
    }
}
