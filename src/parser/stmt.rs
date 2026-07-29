use super::{ClassMember, Expr, ForInit, Parser, Pattern, Statement, SwitchCase, VarKind};
use crate::lexer::Token;

impl Parser {
    pub(crate) fn stmt(&mut self) -> Option<Statement> {
        match self.cur() {
            Token::KwVar => self.var_decl(VarKind::Var),
            Token::KwLet => self.var_decl(VarKind::Let),
            Token::KwConst => self.var_decl(VarKind::Const),
            Token::KwFunction => self.fn_decl(false),
            Token::KwAsync => {
                // `async function name(...) { ... }`
                if matches!(self.peek(), Token::KwFunction) {
                    self.adv(); // consume `async`
                    self.fn_decl(true)
                } else {
                    let e = self.expr()?;
                    self.semi();
                    Some(Statement::Expr(e))
                }
            }
            Token::KwClass => self.class_decl(),
            Token::KwReturn => self.ret(),
            Token::KwIf => self.if_(),
            Token::KwWhile => self.while_(),
            Token::KwDo => self.do_(),
            Token::KwFor => self.for_(),
            Token::KwBreak => {
                self.adv();
                let label = if let Token::Identifier(n) = self.cur() {
                    let l = n.clone();
                    self.adv();
                    Some(l)
                } else {
                    None
                };
                self.semi();
                if let Some(l) = label {
                    Some(Statement::LabeledBreak(l))
                } else {
                    Some(Statement::Break)
                }
            }
            Token::KwContinue => {
                self.adv();
                let label = if let Token::Identifier(n) = self.cur() {
                    let l = n.clone();
                    self.adv();
                    Some(l)
                } else {
                    None
                };
                self.semi();
                if let Some(l) = label {
                    Some(Statement::LabeledContinue(l))
                } else {
                    Some(Statement::Continue)
                }
            }
            Token::KwThrow => self.throw(),
            Token::KwTry => self.try_(),
            Token::KwSwitch => self.switch(),
            Token::KwExport => self.export(),
            Token::KwImport => {
                let saved_pos = self.pos;
                self.adv();
                if self.eat(&Token::Dot)
                    && let Token::Identifier(m) = self.cur()
                    && m == "meta"
                {
                    self.adv();
                    let mut expr = Expr::ImportMeta;
                    while self.eat(&Token::Dot) {
                        let prop = self.ident()?;
                        expr = Expr::Member {
                            object: Box::new(expr),
                            property: Box::new(Expr::String(prop)),
                            computed: false,
                        };
                    }
                    self.semi();
                    return Some(Statement::Expr(expr));
                }
                self.pos = saved_pos;
                self.import()
            }
            Token::LBrace => {
                self.adv();
                let b = self.block_body();
                self.eat(&Token::RBrace);
                Some(Statement::Block(b))
            }
            Token::Semicolon => {
                self.adv();
                Some(Statement::Empty)
            }
            _ => {
                // Labeled statement: `label: statement`
                if let Token::Identifier(n) = self.cur() {
                    if matches!(self.peek(), Token::Colon) {
                        let label = n.clone();
                        self.adv(); // identifier
                        self.adv(); // colon
                        let body = self.stmt()?;
                        return Some(Statement::Labeled {
                            label,
                            body: Box::new(body),
                        });
                    }
                }
                let e = self.expr()?;
                self.semi();
                Some(Statement::Expr(e))
            }
        }
    }

    fn var_decl(&mut self, k: VarKind) -> Option<Statement> {
        self.adv();
        let mut decls = Vec::new();
        loop {
            let mut name = String::new();
            let mut destructuring = None;
            let mut init = None;

            if matches!(self.cur(), Token::LBracket) || matches!(self.cur(), Token::LBrace) {
                // Destructuring declaration: `const [a, b] = ...` / `const {a} = ...`
                destructuring = Some(Box::new(self.pattern()?));
                if self.eat(&Token::Equal) {
                    init = Some(Box::new(self.assign()?));
                }
            } else {
                name = self.ident()?;
                if self.eat(&Token::Equal) {
                    init = Some(Box::new(self.assign()?));
                }
            }

            decls.push(Statement::VarDecl {
                kind: k.clone(),
                name,
                init,
                destructuring,
            });
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.semi();
        if decls.len() == 1 {
            Some(decls.pop().unwrap())
        } else {
            Some(Statement::Block(decls))
        }
    }

    fn pattern(&mut self) -> Option<Pattern> {
        match self.cur() {
            Token::LBracket => {
                self.adv();
                let mut elements = Vec::new();
                while !matches!(self.cur(), Token::RBracket) {
                    if self.eat(&Token::Comma) {
                        elements.push(Pattern::Rest(Box::new(Pattern::Ident("hole".to_string()))));
                        continue;
                    }
                    if self.eat(&Token::DotDotDot) {
                        let p = self.pattern()?;
                        elements.push(Pattern::Rest(Box::new(p)));
                    } else {
                        elements.push(self.pattern()?);
                    }
                    if !matches!(self.cur(), Token::RBracket) {
                        self.eat(&Token::Comma);
                    }
                }
                self.eat(&Token::RBracket);
                Some(Pattern::Array(elements))
            }
            Token::LBrace => {
                self.adv();
                let mut props = Vec::new();
                while !matches!(self.cur(), Token::RBrace) {
                    let key = self.ident()?;
                    let mut pat = None;
                    if self.eat(&Token::Colon) {
                        pat = Some(self.pattern()?);
                    }
                    props.push((key, pat));
                    if !matches!(self.cur(), Token::RBrace) {
                        self.eat(&Token::Comma);
                    }
                }
                self.eat(&Token::RBrace);
                Some(Pattern::Object(props))
            }
            Token::Identifier(n) => {
                let name = n.clone();
                self.adv();
                if self.eat(&Token::Equal) {
                    Some(Pattern::Default(Box::new(Pattern::Ident(name)), Box::new(self.assign()?)))
                } else {
                    Some(Pattern::Ident(name))
                }
            }
            _ => None,
        }
    }

    fn fn_decl(&mut self, is_async: bool) -> Option<Statement> {
        self.adv(); // consume `function`
        // Generator declaration: `function*` (the `*` is accepted and ignored;
        // the body is stored as a plain function).
        self.eat(&Token::Star);
        let n = self.ident()?;
        self.eat(&Token::LParen);
        let (p, defaults) = self.params();
        self.eat(&Token::RParen);
        self.eat(&Token::LBrace);
        let b = self.block_body();
        self.eat(&Token::RBrace);
        let mut body = defaults;
        body.extend(b);
        Some(Statement::FnDecl {
            name: n,
            params: p,
            body,
            is_async,
        })
    }

    fn class_decl(&mut self) -> Option<Statement> {
        self.adv();
        let n = self.ident()?;
        let sc = if self.eat(&Token::KwExtends) {
            Some(Box::new(self.expr()?))
        } else {
            None
        };
        self.eat(&Token::LBrace);
        let mut b = Vec::new();
        while !matches!(self.cur(), Token::RBrace) {
            if self.eof() {
                break;
            }
            let st = self.eat(&Token::KwStatic);
            let is_getter = self.eat(&Token::KwGet);
            let is_setter = self.eat(&Token::KwSet);
            let mn = match self.cur() {
                Token::Identifier(x) => {
                    let v = x.clone();
                    self.adv();
                    v
                }
                Token::KwConstructor => {
                    self.adv();
                    "constructor".to_string()
                }
                _ => return None,
            };
            if self.eat(&Token::LParen) {
                let (p, defaults) = self.params();
                self.eat(&Token::RParen);
                self.eat(&Token::LBrace);
                let bd = self.block_body();
                self.eat(&Token::RBrace);
                let mut body = defaults;
                body.extend(bd);
                if is_getter {
                    b.push(ClassMember::Getter {
                        name: mn,
                        is_static: st,
                        body,
                    });
                } else if is_setter {
                    let param = p.first().cloned().unwrap_or_default();
                    b.push(ClassMember::Setter {
                        name: mn,
                        param,
                        is_static: st,
                        body,
                    });
                } else {
                    b.push(ClassMember::Method {
                        name: mn,
                        is_static: st,
                        params: p,
                        body,
                    });
                }
            } else {
                let i = if self.eat(&Token::Equal) {
                    Some(self.assign()?)
                } else {
                    None
                };
                self.semi();
                b.push(ClassMember::Field {
                    name: mn,
                    is_static: st,
                    init: i,
                });
            }
        }
        self.eat(&Token::RBrace);
        Some(Statement::ClassDecl {
            name: n,
            superclass: sc,
            body: b,
        })
    }

    fn ret(&mut self) -> Option<Statement> {
        self.adv();
        let e = if matches!(self.cur(), Token::Semicolon) || matches!(self.cur(), Token::RBrace) {
            None
        } else {
            Some(Box::new(self.expr()?))
        };
        self.semi();
        Some(Statement::Return(e))
    }

    fn if_(&mut self) -> Option<Statement> {
        self.adv();
        self.eat(&Token::LParen);
        let t = Box::new(self.expr()?);
        self.eat(&Token::RParen);
        let c = self.block_or_stmt();
        let a = if self.eat(&Token::KwElse) {
            if matches!(self.cur(), Token::KwIf) {
                Some(vec![self.if_()?])
            } else {
                Some(self.block_or_stmt())
            }
        } else {
            None
        };
        Some(Statement::If {
            test: t,
            then: c,
            else_: a,
        })
    }

    /// Parses either a `{ ... }` block or a single statement, returning the
    /// body as a statement list. Enables braceless `if`/`for`/`while` bodies.
    fn block_or_stmt(&mut self) -> Vec<Statement> {
        if self.eat(&Token::LBrace) {
            let b = self.block_body();
            self.eat(&Token::RBrace);
            b
        } else {
            match self.stmt() {
                Some(s) => vec![s],
                None => vec![],
            }
        }
    }

    fn while_(&mut self) -> Option<Statement> {
        self.adv();
        self.eat(&Token::LParen);
        let t = Box::new(self.expr()?);
        self.eat(&Token::RParen);
        let b = self.block_or_stmt();
        Some(Statement::While { test: t, body: b })
    }

    fn do_(&mut self) -> Option<Statement> {
        self.adv();
        let b = self.block_or_stmt();
        if !self.eat(&Token::KwWhile) {
            return None;
        }
        self.eat(&Token::LParen);
        let t = Box::new(self.expr()?);
        self.eat(&Token::RParen);
        self.semi();
        Some(Statement::DoWhile { test: t, body: b })
    }

    fn for_(&mut self) -> Option<Statement> {
        self.adv();
        self.eat(&Token::LParen);
        let init = if matches!(
            self.cur(),
            Token::KwVar | Token::KwLet | Token::KwConst
        ) {
            let kind = match self.cur() {
                Token::KwVar => VarKind::Var,
                Token::KwLet => VarKind::Let,
                _ => VarKind::Const,
            };
            self.adv();
            let mut decls = Vec::new();
            loop {
                let n = self.ident()?;
                let i = if self.eat(&Token::Equal) {
                    Some(self.assign()?)
                } else {
                    None
                };
                decls.push((n, i));
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            Some(Box::new(ForInit::Var { kind, decls }))
        } else if matches!(self.cur(), Token::Semicolon) {
            None
        } else {
            Some(Box::new(ForInit::Expr(self.expr()?)))
        };
        if let Some(init) = init.as_ref()
            && !matches!(self.cur(), Token::Semicolon)
        {
            if self.eat(&Token::KwIn) {
                let o = Box::new(self.expr()?);
                self.eat(&Token::RParen);
                let b = self.block_or_stmt();
                let n = match init.as_ref() {
                    ForInit::Var { decls, .. } => decls.first()?.0.clone(),
                    _ => return None,
                };
                return Some(Statement::ForIn {
                    name: n,
                    obj: o,
                    body: b,
                });
            }
            if self.eat(&Token::KwOf) {
                let i = Box::new(self.expr()?);
                self.eat(&Token::RParen);
                let b = self.block_or_stmt();
                let n = match init.as_ref() {
                    ForInit::Var { decls, .. } => decls.first()?.0.clone(),
                    _ => return None,
                };
                return Some(Statement::ForOf {
                    name: n,
                    iter: i,
                    body: b,
                });
            }
        }
        self.semi();
        let t = if !matches!(self.cur(), Token::Semicolon) {
            Some(Box::new(self.expr()?))
        } else {
            None
        };
        self.semi();
        let u = if !matches!(self.cur(), Token::RParen) {
            Some(Box::new(self.expr()?))
        } else {
            None
        };
        self.eat(&Token::RParen);
        let b = self.block_or_stmt();
        Some(Statement::For {
            init,
            test: t,
            update: u,
            body: b,
        })
    }

    fn throw(&mut self) -> Option<Statement> {
        self.adv();
        let e = self.expr()?;
        self.semi();
        Some(Statement::Throw(Box::new(e)))
    }

    fn try_(&mut self) -> Option<Statement> {
        self.adv();
        self.eat(&Token::LBrace);
        let b = self.block_body();
        self.eat(&Token::RBrace);
        let c = if self.eat(&Token::KwCatch) {
            let p = if self.eat(&Token::LParen) {
                let x = self.ident()?;
                self.eat(&Token::RParen);
                x
            } else {
                String::new()
            };
            self.eat(&Token::LBrace);
            let cb = self.block_body();
            self.eat(&Token::RBrace);
            Some((p, cb))
        } else {
            None
        };
        let f = if self.eat(&Token::KwFinally) {
            self.eat(&Token::LBrace);
            let fb = self.block_body();
            self.eat(&Token::RBrace);
            Some(fb)
        } else {
            None
        };
        Some(Statement::Try {
            body: b,
            catch: c,
            finally: f,
        })
    }

    fn switch(&mut self) -> Option<Statement> {
        self.adv();
        self.eat(&Token::LParen);
        let d = Box::new(self.expr()?);
        self.eat(&Token::RParen);
        self.eat(&Token::LBrace);
        let mut cs = Vec::new();
        while !matches!(self.cur(), Token::RBrace) {
            if self.eof() {
                break;
            }
            let t = if self.eat(&Token::KwCase) {
                let e = self.expr()?;
                self.eat(&Token::Colon);
                Some(e)
            } else if self.eat(&Token::KwDefault) {
                self.eat(&Token::Colon);
                None
            } else {
                break;
            };
            let mut b = Vec::new();
            while !matches!(self.cur(), Token::KwCase)
                && !matches!(self.cur(), Token::KwDefault)
                && !matches!(self.cur(), Token::RBrace)
            {
                if self.eof() {
                    break;
                }
                b.push(self.stmt()?);
            }
            cs.push(SwitchCase { test: t, body: b });
        }
        self.eat(&Token::RBrace);
        Some(Statement::Switch { disc: d, cases: cs })
    }

    fn export(&mut self) -> Option<Statement> {
        self.adv();
        if self.eat(&Token::KwDefault) {
            let e = self.expr()?;
            self.semi();
            Some(Statement::ExportDefault(Box::new(e)))
        } else if self.eat(&Token::LBrace) {
            let mut sp = Vec::new();
            while !matches!(self.cur(), Token::RBrace) {
                let l = self.ident()?;
                let e = if self.eat(&Token::KwAs) {
                    self.ident()?
                } else {
                    l.clone()
                };
                sp.push((l, e));
                if !matches!(self.cur(), Token::RBrace) {
                    self.eat(&Token::Comma);
                }
            }
            self.eat(&Token::RBrace);
            let s = if self.eat(&Token::KwFrom) {
                match self.cur() {
                    Token::String(x) => {
                        let v = x.clone();
                        self.adv();
                        Some(v)
                    }
                    _ => None,
                }
            } else {
                None
            };
            self.semi();
            Some(Statement::ExportNamed {
                specifiers: sp,
                source: s,
            })
        } else {
            Some(Statement::ExportNamed {
                specifiers: vec![],
                source: None,
            })
        }
    }

    fn import(&mut self) -> Option<Statement> {
        self.adv();
        let def = if let Token::Identifier(n) = self.cur() {
            let nm = n.clone();
            self.adv();
            if self.eat(&Token::Comma) {
                if self.eat(&Token::LBrace) {
                    let mut nd = Vec::new();
                    while !matches!(self.cur(), Token::RBrace) {
                        let l = self.ident()?;
                        let i = if self.eat(&Token::KwAs) {
                            self.ident()?
                        } else {
                            l.clone()
                        };
                        nd.push((l, i));
                        if !matches!(self.cur(), Token::RBrace) {
                            self.eat(&Token::Comma);
                        }
                    }
                    self.eat(&Token::RBrace);
                    let m = self.from()?;
                    Some(Statement::Import {
                        module: m,
                        default: Some(nm),
                        named: nd,
                        namespace: None,
                    })
                } else {
                    None
                }
            } else if self.eat(&Token::KwFrom) {
                let m = self.from()?;
                Some(Statement::Import {
                    module: m,
                    default: Some(nm),
                    named: vec![],
                    namespace: None,
                })
            } else {
                None
            }
        } else if self.eat(&Token::Star) {
            self.eat(&Token::KwAs);
            let ns = self.ident()?;
            let m = self.from()?;
            Some(Statement::Import {
                module: m,
                default: None,
                named: vec![],
                namespace: Some(ns),
            })
        } else if self.eat(&Token::LBrace) {
            let mut nd = Vec::new();
            while !matches!(self.cur(), Token::RBrace) {
                let l = self.ident()?;
                let i = if self.eat(&Token::KwAs) {
                    self.ident()?
                } else {
                    l.clone()
                };
                nd.push((l, i));
                if !matches!(self.cur(), Token::RBrace) {
                    self.eat(&Token::Comma);
                }
            }
            self.eat(&Token::RBrace);
            let m = self.from()?;
            Some(Statement::Import {
                module: m,
                default: None,
                named: nd,
                namespace: None,
            })
        } else if let Token::String(s) = self.cur() {
            let m = s.clone();
            self.adv();
            self.semi();
            Some(Statement::Import {
                module: m,
                default: None,
                named: vec![],
                namespace: None,
            })
        } else {
            None
        };
        self.semi();
        def
    }

    fn from(&mut self) -> Option<String> {
        self.eat(&Token::KwFrom);
        match self.cur() {
            Token::String(s) => {
                let v = s.clone();
                self.adv();
                Some(v)
            }
            _ => None,
        }
    }

    pub(crate) fn block_body(&mut self) -> Vec<Statement> {
        let mut s = Vec::new();
        while !matches!(self.cur(), Token::RBrace) {
            if self.eof() {
                break;
            }
            if let Some(st) = self.stmt() {
                s.push(st);
            } else {
                break;
            }
        }
        s
    }

    /// Build the guard statement implementing a parameter default value:
    /// `if (name === undefined) name = <default>;`
    pub(crate) fn default_guard(name: &str, d: Expr) -> Statement {
        Statement::If {
            test: Box::new(Expr::Binary {
                op: "===".to_string(),
                left: Box::new(Expr::Identifier(name.to_string())),
                right: Box::new(Expr::Undefined),
            }),
            then: vec![Statement::Expr(Expr::Assignment {
                target: Box::new(Expr::Identifier(name.to_string())),
                op: "=".to_string(),
                value: Box::new(d),
            })],
            else_: None,
        }
    }

    /// Parse a parameter list. Returns the parameter names (rest params keep
    /// their `...` prefix) plus guard statements that implement default values
    /// (`if (name === undefined) name = <default>;`), to be prepended to a body.
    pub(crate) fn params(&mut self) -> (Vec<String>, Vec<Statement>) {
        let mut names = Vec::new();
        let mut defaults = Vec::new();
        while !matches!(self.cur(), Token::RParen) {
            match self.cur() {
                Token::DotDotDot => {
                    self.adv();
                    if let Token::Identifier(n) = self.cur() {
                        names.push(format!("...{}", n));
                        self.adv();
                    }
                }
                Token::Identifier(n) => {
                    let name = n.clone();
                    self.adv();
                    if self.eat(&Token::Equal) {
                        if let Some(d) = self.assign() {
                            defaults.push(Self::default_guard(&name, d));
                        }
                    }
                    names.push(name);
                }
                _ => {
                    self.adv();
                }
            }
            if !matches!(self.cur(), Token::RParen) {
                self.eat(&Token::Comma);
            }
        }
        (names, defaults)
    }

    pub(crate) fn ident(&mut self) -> Option<String> {
        match self.cur() {
            Token::Identifier(n) => {
                let v = n.clone();
                self.adv();
                Some(v)
            }
            _ => None,
        }
    }
}
