mod env;
mod ops;

pub use env::{Env, Environment, Module};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::error::{VmErr, vm_err, vm_ret, vm_throw};
use crate::parser::{Expr, ExprOrBlock, ForInit, Statement};
use crate::value::Value;

pub struct Interpreter {
    pub global: Env,
    pub modules: HashMap<String, Module>,
    pub cur_mod: Option<String>,
    pub is_main: bool,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            global: Rc::new(RefCell::new(Environment::new())),
            modules: HashMap::new(),
            cur_mod: None,
            is_main: false,
        }
    }

    pub fn run(&mut self, stmts: &[Statement]) -> Result<Value, VmErr> {
        let mut r = Value::Undefined;
        for s in stmts {
            r = self.eval_stmt(s)?;
        }
        Ok(r)
    }

    fn eval_stmt(&mut self, s: &Statement) -> Result<Value, VmErr> {
        match s {
            Statement::Expr(e) => self.eval_expr(e),
            Statement::VarDecl { name, init, .. } => {
                let v = match init {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Undefined,
                };
                self.global.borrow_mut().set(name, v.clone());
                Ok(v)
            }
            Statement::FnDecl { name, params, body } => {
                self.global.borrow_mut().set(
                    name,
                    Value::Function {
                        name: Some(name.clone()),
                        params: params.clone(),
                        body: body.clone(),
                        closure: Some(self.global.clone()),
                    },
                );
                Ok(Value::Undefined)
            }
            Statement::ClassDecl { name, .. } => {
                self.global.borrow_mut().set(
                    name,
                    Value::NativeFunction {
                        name: name.clone(),
                        callable: |_| Ok(Value::Object(vec![])),
                    },
                );
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
                let mut r = Value::Undefined;
                loop {
                    let t = self.eval_expr(test)?;
                    if !self.truthy(&t) {
                        break;
                    }
                    match self.run(body) {
                        Err(VmErr::Msg(m)) if m == "__BREAK__" => break,
                        Err(VmErr::Msg(m)) if m == "__CONTINUE__" => continue,
                        other => r = other?,
                    }
                }
                Ok(r)
            }
            Statement::DoWhile { test, body } => {
                let mut r = Value::Undefined;
                loop {
                    match self.run(body) {
                        Err(VmErr::Msg(m)) if m == "__BREAK__" => break,
                        Err(VmErr::Msg(m)) if m == "__CONTINUE__" => {}
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
                        ForInit::Var { name, init, .. } => {
                            let v = match init {
                                Some(e) => self.eval_expr(e)?,
                                None => Value::Undefined,
                            };
                            self.global.borrow_mut().set(name, v);
                        }
                        ForInit::Expr(e) => {
                            self.eval_expr(e)?;
                        }
                    }
                }
                let mut r = Value::Undefined;
                loop {
                    if let Some(t) = test {
                        let tv = self.eval_expr(t)?;
                        if !self.truthy(&tv) {
                            break;
                        }
                    }
                    match self.run(body) {
                        Err(VmErr::Msg(m)) if m == "__BREAK__" => break,
                        Err(VmErr::Msg(m)) if m == "__CONTINUE__" => {}
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
                for k in ks {
                    self.global.borrow_mut().set(name, Value::String(k));
                    match self.run(body) {
                        Err(VmErr::Msg(m)) if m == "__BREAK__" => break,
                        Err(VmErr::Msg(m)) if m == "__CONTINUE__" => continue,
                        other => r = other?,
                    }
                }
                Ok(r)
            }
            Statement::ForOf { name, iter, body } => {
                let a = self.eval_expr(iter)?;
                let items = match &a {
                    Value::Array(i) => i.clone(),
                    _ => return vm_err("for...of needs iterable"),
                };
                let mut r = Value::Undefined;
                for i in items {
                    self.global.borrow_mut().set(name, i);
                    match self.run(body) {
                        Err(VmErr::Msg(m)) if m == "__BREAK__" => break,
                        Err(VmErr::Msg(m)) if m == "__CONTINUE__" => continue,
                        other => r = other?,
                    }
                }
                Ok(r)
            }
            Statement::Block(s) => self.run(s),
            Statement::Break => vm_err("__BREAK__"),
            Statement::Continue => vm_err("__CONTINUE__"),
            Statement::Throw(e) => {
                let v = self.eval_expr(e)?;
                vm_throw(self.vs(&v))
            }
            Statement::Try {
                body,
                catch,
                finally,
            } => match self.run(body) {
                Err(VmErr::Throw(msg)) => {
                    if let Some((p, cb)) = catch {
                        let ce = Rc::new(RefCell::new(Environment::child(self.global.clone())));
                        ce.borrow_mut().set(p, Value::String(msg));
                        let s = self.global.clone();
                        self.global = ce;
                        let r = self.run(cb);
                        self.global = s;
                        r
                    } else {
                        Ok(Value::Undefined)
                    }
                }
                Err(VmErr::Ret(v)) => Err(VmErr::Ret(v)),
                other => {
                    if let Some(f) = finally {
                        self.run(f)?;
                    }
                    other
                }
            },
            Statement::Switch { disc, cases } => {
                let d = self.eval_expr(disc)?;
                let mut r = Value::Undefined;
                let mut m = false;
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
                                if s == "__BREAK__" {
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
                        self.global.borrow_mut().set(ns, Value::Object(p));
                    }
                    Ok(Value::Undefined)
                } else {
                    vm_err(format!("Module not found: {}", module))
                }
            }
            Statement::Empty => Ok(Value::Undefined),
        }
    }

    fn eval_expr(&mut self, e: &Expr) -> Result<Value, VmErr> {
        match e {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Null => Ok(Value::Null),
            Expr::Undefined => Ok(Value::Undefined),
            Expr::Identifier(n) => self
                .global
                .borrow()
                .get(n)
                .ok_or_else(|| VmErr::Msg(format!("Undefined: {}", n))),
            Expr::Array(i) => {
                let mut v = Vec::new();
                for x in i {
                    v.push(self.eval_expr(x)?);
                }
                Ok(Value::Array(v))
            }
            Expr::Object(p) => {
                let mut o = Vec::new();
                for (k, v) in p {
                    o.push((k.clone(), self.eval_expr(v)?));
                }
                Ok(Value::Object(o))
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
                if (op == "++" || op == "--") && matches!(operand.as_ref(), Expr::Identifier(_)) {
                    if let Expr::Identifier(n) = operand.as_ref() {
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
                    } else {
                        unreachable!()
                    }
                } else {
                    let v = self.eval_expr(operand)?;
                    self.un_op(op, &v)
                }
            }
            Expr::Call { callee, args } => {
                let mut a = Vec::new();
                for x in args {
                    a.push(self.eval_expr(x)?);
                }
                let c = self.eval_expr(callee)?;
                self.call(&c, a)
            }
            Expr::Member {
                object,
                property,
                computed: _,
            } => {
                let o = self.eval_expr(object)?;
                let p = self.eval_expr(property)?;
                self.prop(&o, &p)
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
                params: params.clone(),
                closure: Some(self.global.clone()),
                body: match body.as_ref() {
                    ExprOrBlock::Block(s) => s.clone(),
                    ExprOrBlock::Expr(e) => vec![Statement::Return(Some(e.clone()))],
                },
            }),
            Expr::FnExpr { name, params, body } => Ok(Value::Function {
                name: name.clone(),
                params: params.clone(),
                body: body.clone(),
                closure: Some(self.global.clone()),
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
                Ok(Value::Object(o))
            }
        }
    }

    fn call(&mut self, f: &Value, args: Vec<Value>) -> Result<Value, VmErr> {
        match f {
            Value::Function {
                params,
                body,
                closure,
                ..
            } => {
                let parent_env = closure.clone().unwrap_or_else(|| self.global.clone());
                let fe = Rc::new(RefCell::new(Environment::child(parent_env)));
                for (i, p) in params.iter().enumerate() {
                    fe.borrow_mut()
                        .set(p, args.get(i).cloned().unwrap_or(Value::Undefined));
                }
                let s = self.global.clone();
                self.global = fe;
                let r = self.run(body);
                self.global = s;
                match r {
                    Err(VmErr::Ret(v)) => Ok(v),
                    other => other,
                }
            }
            Value::NativeFunction { callable, .. } => callable(args),
            _ => vm_err("Not a function"),
        }
    }

    fn ctor(&mut self, f: &Value, args: Vec<Value>) -> Result<Value, VmErr> {
        let inst = Value::Object(vec![]);
        if let Value::Function {
            params,
            body,
            closure,
            ..
        } = f
        {
            let parent_env = closure.clone().unwrap_or_else(|| self.global.clone());
            let fe = Rc::new(RefCell::new(Environment::child(parent_env)));
            fe.borrow_mut().set("this", inst.clone());
            for (i, p) in params.iter().enumerate() {
                fe.borrow_mut()
                    .set(p, args.get(i).cloned().unwrap_or(Value::Undefined));
            }
            let s = self.global.clone();
            self.global = fe;
            let r = self.run(body);
            self.global = s;
            match r {
                Err(VmErr::Ret(v)) => match v {
                    Value::Object(_) => Ok(v),
                    _ => Ok(inst),
                },
                _ => Ok(inst),
            }
        } else {
            vm_err("Not a constructor")
        }
    }

    fn prop(&self, o: &Value, p: &Value) -> Result<Value, VmErr> {
        match (o, p) {
            (Value::Object(props), Value::String(k)) => {
                for (xk, xv) in props {
                    if xk == k {
                        return Ok(xv.clone());
                    }
                }
                Ok(Value::Undefined)
            }
            (Value::Array(items), Value::Number(i)) => {
                let idx = *i as usize;
                if idx < items.len() {
                    Ok(items[idx].clone())
                } else {
                    Ok(Value::Undefined)
                }
            }
            (Value::Array(items), Value::String(k)) => {
                if k == "length" {
                    Ok(Value::Number(items.len() as f64))
                } else {
                    Ok(Value::Undefined)
                }
            }
            (Value::String(s), Value::String(k)) => {
                if k == "length" {
                    Ok(Value::Number(s.len() as f64))
                } else {
                    Ok(Value::Undefined)
                }
            }
            _ => Ok(Value::Undefined),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::setup_builtins;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn eval(src: &str) -> Result<Value, VmErr> {
        let mut interp = Interpreter::new();
        setup_builtins(&interp.global);
        let mut lex = Lexer::new(src);
        let toks = lex.tokenize();
        let mut parser = Parser::new(toks);
        let stmts = parser.parse();
        interp.run(&stmts)
    }

    fn eval_str(src: &str) -> String {
        let interp = Interpreter::new();
        match eval(src) {
            Ok(v) => interp.vs(&v),
            Err(e) => format!("ERROR: {}", e),
        }
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(eval_str("2 + 2;"), "4");
        assert_eq!(eval_str("10 - 3;"), "7");
        assert_eq!(eval_str("4 * 5;"), "20");
        assert_eq!(eval_str("15 / 3;"), "5");
        assert_eq!(eval_str("10 % 3;"), "1");
    }

    #[test]
    fn test_variables() {
        assert_eq!(eval_str("const x = 42; x;"), "42");
        assert_eq!(eval_str("let x = 1; x = 2; x;"), "2");
    }

    #[test]
    fn test_functions() {
        assert_eq!(
            eval_str("function add(a, b) { return a + b; } add(3, 4);"),
            "7"
        );
        assert_eq!(eval_str("const f = (x) => x * x; f(5);"), "25");
    }

    #[test]
    fn test_closures() {
        assert_eq!(
            eval_str(
                "function counter() { let n = 0; return () => ++n; } const c = counter(); c(); c(); c();"
            ),
            "3"
        );
    }

    #[test]
    fn test_recursion() {
        assert_eq!(
            eval_str("function fib(n) { return n <= 1 ? n : fib(n-1) + fib(n-2); } fib(10);"),
            "55"
        );
    }

    #[test]
    fn test_strings() {
        assert_eq!(eval_str("'hello' + ' ' + 'world';"), "hello world");
        assert_eq!(eval_str("'hello'.length;"), "5");
    }

    #[test]
    fn test_arrays() {
        assert_eq!(eval_str("const a = [1,2,3]; a.length;"), "3");
        assert_eq!(eval_str("const a = [10,20,30]; a[1];"), "20");
    }

    #[test]
    fn test_objects() {
        assert_eq!(eval_str("const o = {x: 1}; o.x;"), "1");
        assert_eq!(eval_str("const o = {x: 1}; o['x'];"), "1");
    }

    #[test]
    fn test_loops() {
        assert_eq!(
            eval_str("let s = 0; for (let i = 0; i < 10; i++) { s += i; } s;"),
            "45"
        );
        assert_eq!(eval_str("let i = 0; while (i < 5) { i++; } i;"), "5");
    }

    #[test]
    fn test_try_catch() {
        assert_eq!(
            eval_str("try { throw 'oops'; } catch(e) { 'caught: ' + e; }"),
            "caught: oops"
        );
    }

    #[test]
    fn test_typeof() {
        assert_eq!(eval_str("typeof 42;"), "number");
        assert_eq!(eval_str("typeof 'hi';"), "string");
        assert_eq!(eval_str("typeof true;"), "boolean");
        assert_eq!(eval_str("typeof undefined;"), "undefined");
        assert_eq!(eval_str("typeof null;"), "object");
    }

    #[test]
    fn test_comparison() {
        assert_eq!(eval_str("5 === 5;"), "true");
        assert_eq!(eval_str("5 !== 3;"), "true");
        assert_eq!(eval_str("5 == 5;"), "true");
        assert_eq!(eval_str("'5' === 5;"), "false");
    }

    #[test]
    fn test_logical() {
        assert_eq!(eval_str("true && false;"), "false");
        assert_eq!(eval_str("true || false;"), "true");
        assert_eq!(eval_str("!true;"), "false");
    }

    #[test]
    fn test_ternary() {
        assert_eq!(eval_str("true ? 'yes' : 'no';"), "yes");
        assert_eq!(eval_str("false ? 'yes' : 'no';"), "no");
    }

    #[test]
    fn test_increment() {
        assert_eq!(eval_str("let i = 0; i++;"), "0");
        assert_eq!(eval_str("let i = 0; ++i;"), "1");
    }

    #[test]
    fn test_compound_assign() {
        assert_eq!(eval_str("let x = 5; x += 3; x;"), "8");
        assert_eq!(eval_str("let x = 10; x -= 4; x;"), "6");
        assert_eq!(eval_str("let x = 3; x *= 2; x;"), "6");
    }

    #[test]
    fn test_for_of() {
        assert_eq!(
            eval_str("let s = 0; for (const x of [1,2,3]) { s += x; } s;"),
            "6"
        );
    }

    #[test]
    fn test_for_in() {
        assert_eq!(
            eval_str("let r = ''; for (const k in {a: 1, b: 2}) { r += k; } r;"),
            "ab"
        );
    }

    #[test]
    fn test_switch() {
        assert_eq!(
            eval_str(
                "let r = ''; switch (2) { case 1: r = 'one'; break; case 2: r = 'two'; break; default: r = 'other'; } r;"
            ),
            "two"
        );
    }

    #[test]
    fn test_nested_functions() {
        assert_eq!(
            eval_str(
                "function outer() { function inner() { return 42; } return inner(); } outer();"
            ),
            "42"
        );
    }

    #[test]
    fn test_math_constants() {
        assert_eq!(eval_str("Math.PI;"), "3.141592653589793");
        assert_eq!(eval_str("Math.E;"), "2.718281828459045");
    }

    #[test]
    fn test_do_while() {
        assert_eq!(eval_str("let i = 0; do { i++; } while (i < 5); i;"), "5");
    }

    #[test]
    fn test_break_in_loops() {
        assert_eq!(
            eval_str("let i = 0; while (true) { if (i >= 3) { break; } i++; } i;"),
            "3"
        );
        assert_eq!(
            eval_str("let n = 0; for (let i = 0; i < 10; i++) { if (i === 4) { break; } n++; } n;"),
            "4"
        );
        assert_eq!(
            eval_str("let i = 0; do { if (i >= 2) { break; } i++; } while (true); i;"),
            "2"
        );
    }

    #[test]
    fn test_continue_in_loops() {
        assert_eq!(
            eval_str("let s = 0; for (let i = 0; i < 5; i++) { if (i % 2) { continue; } s += i; } s;"),
            "6"
        );
        assert_eq!(
            eval_str("let s = 0; let i = 0; while (i < 5) { i++; if (i === 3) { continue; } s += i; } s;"),
            "12"
        );
    }
}
