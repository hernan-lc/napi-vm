//! Compound statement parsers: classes, `import` / `export`, and the shared
//! block-body / default-guard helpers used across the parser.

use super::{ClassMember, Expr, Parser, Statement, VarKind};
use crate::lexer::Token;

/// Collect the bound identifier names from a variable declaration statement
/// (which may be a single `VarDecl` or a block of them for `let a, b`).
fn declared_names(stmt: &Statement) -> Vec<String> {
    match stmt {
        Statement::VarDecl { name, .. } if !name.is_empty() => vec![name.clone()],
        Statement::Block(stmts) => stmts.iter().flat_map(declared_names).collect(),
        _ => vec![],
    }
}

/// The declared name of a function or class declaration, as a one-element list.
fn decl_name(stmt: &Statement) -> Vec<String> {
    match stmt {
        Statement::FnDecl { name, .. } | Statement::ClassDecl { name, .. } => vec![name.clone()],
        _ => vec![],
    }
}

impl Parser {
    pub(super) fn class_decl(&mut self) -> Option<Statement> {
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

    pub(super) fn export(&mut self) -> Option<Statement> {
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
            self.export_decl()
        }
    }

    /// Parse `export <declaration>` (`var`/`let`/`const`/`function`/`class`),
    /// declaring the binding and re-exporting the declared names. Desugars to a
    /// block of the declaration followed by an `export { name }` so the existing
    /// named-export evaluation (which reads from the global scope) applies.
    fn export_decl(&mut self) -> Option<Statement> {
        let (decl, names) = match self.cur() {
            Token::KwVar => {
                let d = self.var_decl(VarKind::Var)?;
                let n = declared_names(&d);
                (d, n)
            }
            Token::KwLet => {
                let d = self.var_decl(VarKind::Let)?;
                let n = declared_names(&d);
                (d, n)
            }
            Token::KwConst => {
                let d = self.var_decl(VarKind::Const)?;
                let n = declared_names(&d);
                (d, n)
            }
            Token::KwFunction => {
                let d = self.fn_decl(false)?;
                let n = decl_name(&d);
                (d, n)
            }
            Token::KwAsync if matches!(self.peek(), Token::KwFunction) => {
                self.adv();
                let d = self.fn_decl(true)?;
                let n = decl_name(&d);
                (d, n)
            }
            Token::KwClass => {
                let d = self.class_decl()?;
                let n = decl_name(&d);
                (d, n)
            }
            _ => {
                return Some(Statement::ExportNamed {
                    specifiers: vec![],
                    source: None,
                })
            }
        };
        let specifiers: Vec<(String, String)> = names.into_iter().map(|n| (n.clone(), n)).collect();
        Some(Statement::Block(vec![
            decl,
            Statement::ExportNamed {
                specifiers,
                source: None,
            },
        ]))
    }

    pub(super) fn import(&mut self) -> Option<Statement> {
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
}
