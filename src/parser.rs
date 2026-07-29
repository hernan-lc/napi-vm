use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Undefined,
    Identifier(String),
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    Binary { op: String, left: Box<Expr>, right: Box<Expr> },
    Unary { op: String, operand: Box<Expr>, prefix: bool },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Member { object: Box<Expr>, property: Box<Expr>, computed: bool },
    Assignment { target: Box<Expr>, op: String, value: Box<Expr> },
    Conditional { test: Box<Expr>, consequent: Box<Expr>, alternate: Box<Expr> },
    ArrowFn { params: Vec<String>, body: Box<ExprOrBlock> },
    FnExpr { name: Option<String>, params: Vec<String>, body: Vec<Statement> },
    New { callee: Box<Expr>, args: Vec<Expr> },
    Spread(Box<Expr>),
    This,
    ImportMeta,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprOrBlock {
    Expr(Box<Expr>),
    Block(Vec<Statement>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expr(Expr),
    VarDecl { kind: VarKind, name: String, init: Option<Box<Expr>> },
    FnDecl { name: String, params: Vec<String>, body: Vec<Statement> },
    ClassDecl { name: String, superclass: Option<Box<Expr>>, body: Vec<ClassMember> },
    Return(Option<Box<Expr>>),
    If { test: Box<Expr>, then: Vec<Statement>, else_: Option<Vec<Statement>> },
    While { test: Box<Expr>, body: Vec<Statement> },
    For { init: Option<Box<ForInit>>, test: Option<Box<Expr>>, update: Option<Box<Expr>>, body: Vec<Statement> },
    ForIn { name: String, obj: Box<Expr>, body: Vec<Statement> },
    ForOf { name: String, iter: Box<Expr>, body: Vec<Statement> },
    Block(Vec<Statement>),
    Break,
    Continue,
    Throw(Box<Expr>),
    Try { body: Vec<Statement>, catch: Option<(String, Vec<Statement>)>, finally: Option<Vec<Statement>> },
    Switch { disc: Box<Expr>, cases: Vec<SwitchCase> },
    ExportDefault(Box<Expr>),
    ExportNamed { specifiers: Vec<(String, String)>, source: Option<String> },
    Import { module: String, default: Option<String>, named: Vec<(String, String)>, namespace: Option<String> },
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VarKind {
    Var,
    Let,
    Const,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForInit {
    Var { kind: VarKind, name: String, init: Option<Box<Expr>> },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub test: Option<Expr>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Method { name: String, is_static: bool, params: Vec<String>, body: Vec<Statement> },
    Field { name: String, is_static: bool, init: Option<Expr> },
}

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(t: Vec<Token>) -> Self {
        Self { toks: t, pos: 0 }
    }

    pub fn parse(&mut self) -> Vec<Statement> {
        let mut s = Vec::new();
        while !self.eof() {
            if let Some(st) = self.stmt() {
                s.push(st);
            } else {
                self.adv();
            }
        }
        s
    }

    fn cur(&self) -> &Token {
        self.toks.get(self.pos).unwrap_or(&Token::EOF)
    }

    fn adv(&mut self) -> &Token {
        if self.pos < self.toks.len() {
            self.pos += 1;
        }
        self.toks.get(self.pos - 1).unwrap_or(&Token::EOF)
    }

    fn eat(&mut self, t: &Token) -> bool {
        if self.cur() == t {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn eof(&self) -> bool {
        matches!(self.cur(), Token::EOF)
    }

    fn semi(&mut self) {
        if matches!(self.cur(), Token::Semicolon) {
            self.pos += 1;
        }
    }

    fn stmt(&mut self) -> Option<Statement> {
        match self.cur() {
            Token::KwVar => self.var_decl(VarKind::Var),
            Token::KwLet => self.var_decl(VarKind::Let),
            Token::KwConst => self.var_decl(VarKind::Const),
            Token::KwFunction => self.fn_decl(),
            Token::KwClass => self.class_decl(),
            Token::KwReturn => self.ret(),
            Token::KwIf => self.if_(),
            Token::KwWhile => self.while_(),
            Token::KwFor => self.for_(),
            Token::KwBreak => { self.adv(); self.semi(); Some(Statement::Break) }
            Token::KwContinue => { self.adv(); self.semi(); Some(Statement::Continue) }
            Token::KwThrow => self.throw(),
            Token::KwTry => self.try_(),
            Token::KwSwitch => self.switch(),
            Token::KwExport => self.export(),
            Token::KwImport => {
                let saved_pos = self.pos;
                self.adv();
                if self.eat(&Token::Dot) {
                    if let Token::Identifier(m) = self.cur() {
                        if m == "meta" {
                            self.adv();
                            let mut expr = Expr::ImportMeta;
                            while self.eat(&Token::Dot) {
                                let prop = self.ident()?;
                                expr = Expr::Member { object: Box::new(expr), property: Box::new(Expr::String(prop)), computed: false };
                            }
                            self.semi();
                            return Some(Statement::Expr(expr));
                        }
                    }
                }
                self.pos = saved_pos;
                self.import()
            }
            Token::LBrace => { self.adv(); let b = self.block_body(); self.eat(&Token::RBrace); Some(Statement::Block(b)) }
            Token::Semicolon => { self.adv(); Some(Statement::Empty) }
            _ => { let e = self.expr()?; self.semi(); Some(Statement::Expr(e)) }
        }
    }

    fn var_decl(&mut self, k: VarKind) -> Option<Statement> {
        self.adv();
        let n = self.ident()?;
        let i = if self.eat(&Token::Equal) { Some(Box::new(self.expr()?)) } else { None };
        self.semi();
        Some(Statement::VarDecl { kind: k, name: n, init: i })
    }

    fn fn_decl(&mut self) -> Option<Statement> {
        self.adv();
        let n = self.ident()?;
        self.eat(&Token::LParen);
        let p = self.params();
        self.eat(&Token::RParen);
        self.eat(&Token::LBrace);
        let b = self.block_body();
        self.eat(&Token::RBrace);
        Some(Statement::FnDecl { name: n, params: p, body: b })
    }

    fn class_decl(&mut self) -> Option<Statement> {
        self.adv();
        let n = self.ident()?;
        let sc = if self.eat(&Token::KwExtends) { Some(Box::new(self.expr()?)) } else { None };
        self.eat(&Token::LBrace);
        let mut b = Vec::new();
        while !matches!(self.cur(), Token::RBrace) {
            if self.eof() { break; }
            let st = self.eat(&Token::KwStatic);
            let mn = match self.cur() {
                Token::Identifier(x) => { let v = x.clone(); self.adv(); v }
                Token::KwConstructor => { self.adv(); "constructor".to_string() }
                _ => return None,
            };
            if self.eat(&Token::LParen) {
                let p = self.params();
                self.eat(&Token::RParen);
                self.eat(&Token::LBrace);
                let bd = self.block_body();
                self.eat(&Token::RBrace);
                b.push(ClassMember::Method { name: mn, is_static: st, params: p, body: bd });
            } else {
                let i = if self.eat(&Token::Equal) { Some(self.expr()?) } else { None };
                self.semi();
                b.push(ClassMember::Field { name: mn, is_static: st, init: i });
            }
        }
        self.eat(&Token::RBrace);
        Some(Statement::ClassDecl { name: n, superclass: sc, body: b })
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
        self.eat(&Token::LBrace);
        let c = self.block_body();
        self.eat(&Token::RBrace);
        let a = if self.eat(&Token::KwElse) {
            if matches!(self.cur(), Token::KwIf) {
                Some(vec![self.if_()?])
            } else {
                self.eat(&Token::LBrace);
                let b = self.block_body();
                self.eat(&Token::RBrace);
                Some(b)
            }
        } else {
            None
        };
        Some(Statement::If { test: t, then: c, else_: a })
    }

    fn while_(&mut self) -> Option<Statement> {
        self.adv();
        self.eat(&Token::LParen);
        let t = Box::new(self.expr()?);
        self.eat(&Token::RParen);
        self.eat(&Token::LBrace);
        let b = self.block_body();
        self.eat(&Token::RBrace);
        Some(Statement::While { test: t, body: b })
    }

    fn for_(&mut self) -> Option<Statement> {
        self.adv();
        self.eat(&Token::LParen);
        let init = if self.eat(&Token::KwVar) {
            let n = self.ident()?;
            let i = if self.eat(&Token::Equal) { Some(Box::new(self.expr()?)) } else { None };
            Some(Box::new(ForInit::Var { kind: VarKind::Var, name: n, init: i }))
        } else if self.eat(&Token::KwLet) {
            let n = self.ident()?;
            let i = if self.eat(&Token::Equal) { Some(Box::new(self.expr()?)) } else { None };
            Some(Box::new(ForInit::Var { kind: VarKind::Let, name: n, init: i }))
        } else if self.eat(&Token::KwConst) {
            let n = self.ident()?;
            let i = if self.eat(&Token::Equal) { Some(Box::new(self.expr()?)) } else { None };
            Some(Box::new(ForInit::Var { kind: VarKind::Const, name: n, init: i }))
        } else if matches!(self.cur(), Token::Semicolon) {
            None
        } else {
            Some(Box::new(ForInit::Expr(self.expr()?)))
        };
        if init.is_some() && !matches!(self.cur(), Token::Semicolon) {
            if self.eat(&Token::KwIn) {
                let o = Box::new(self.expr()?);
                self.eat(&Token::RParen);
                self.eat(&Token::LBrace);
                let b = self.block_body();
                self.eat(&Token::RBrace);
                let n = match init.unwrap().as_ref() { ForInit::Var { name, .. } => name.clone(), _ => return None };
                return Some(Statement::ForIn { name: n, obj: o, body: b });
            }
            if self.eat(&Token::KwOf) {
                let i = Box::new(self.expr()?);
                self.eat(&Token::RParen);
                self.eat(&Token::LBrace);
                let b = self.block_body();
                self.eat(&Token::RBrace);
                let n = match init.unwrap().as_ref() { ForInit::Var { name, .. } => name.clone(), _ => return None };
                return Some(Statement::ForOf { name: n, iter: i, body: b });
            }
        }
        self.semi();
        let t = if !matches!(self.cur(), Token::Semicolon) { Some(Box::new(self.expr()?)) } else { None };
        self.semi();
        let u = if !matches!(self.cur(), Token::RParen) { Some(Box::new(self.expr()?)) } else { None };
        self.eat(&Token::RParen);
        self.eat(&Token::LBrace);
        let b = self.block_body();
        self.eat(&Token::RBrace);
        Some(Statement::For { init, test: t, update: u, body: b })
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
            let p = if self.eat(&Token::LParen) { let x = self.ident()?; self.eat(&Token::RParen); x } else { String::new() };
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
        Some(Statement::Try { body: b, catch: c, finally: f })
    }

    fn switch(&mut self) -> Option<Statement> {
        self.adv();
        self.eat(&Token::LParen);
        let d = Box::new(self.expr()?);
        self.eat(&Token::RParen);
        self.eat(&Token::LBrace);
        let mut cs = Vec::new();
        while !matches!(self.cur(), Token::RBrace) {
            if self.eof() { break; }
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
            while !matches!(self.cur(), Token::KwCase) && !matches!(self.cur(), Token::KwDefault) && !matches!(self.cur(), Token::RBrace) {
                if self.eof() { break; }
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
                let e = if self.eat(&Token::KwAs) { self.ident()? } else { l.clone() };
                sp.push((l, e));
                if !matches!(self.cur(), Token::RBrace) { self.eat(&Token::Comma); }
            }
            self.eat(&Token::RBrace);
            let s = if self.eat(&Token::KwFrom) {
                match self.cur() { Token::String(x) => { let v = x.clone(); self.adv(); Some(v) }, _ => None }
            } else {
                None
            };
            self.semi();
            Some(Statement::ExportNamed { specifiers: sp, source: s })
        } else {
            Some(Statement::ExportNamed { specifiers: vec![], source: None })
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
                        let i = if self.eat(&Token::KwAs) { self.ident()? } else { l.clone() };
                        nd.push((l, i));
                        if !matches!(self.cur(), Token::RBrace) { self.eat(&Token::Comma); }
                    }
                    self.eat(&Token::RBrace);
                    let m = self.from()?;
                    Some(Statement::Import { module: m, default: Some(nm), named: nd, namespace: None })
                } else {
                    None
                }
            } else if self.eat(&Token::KwFrom) {
                let m = self.from()?;
                Some(Statement::Import { module: m, default: Some(nm), named: vec![], namespace: None })
            } else {
                None
            }
        } else if self.eat(&Token::Star) {
            self.eat(&Token::KwAs);
            let ns = self.ident()?;
            let m = self.from()?;
            Some(Statement::Import { module: m, default: None, named: vec![], namespace: Some(ns) })
        } else if self.eat(&Token::LBrace) {
            let mut nd = Vec::new();
            while !matches!(self.cur(), Token::RBrace) {
                let l = self.ident()?;
                let i = if self.eat(&Token::KwAs) { self.ident()? } else { l.clone() };
                nd.push((l, i));
                if !matches!(self.cur(), Token::RBrace) { self.eat(&Token::Comma); }
            }
            self.eat(&Token::RBrace);
            let m = self.from()?;
            Some(Statement::Import { module: m, default: None, named: nd, namespace: None })
        } else if let Token::String(s) = self.cur() {
            let m = s.clone();
            self.adv();
            self.semi();
            Some(Statement::Import { module: m, default: None, named: vec![], namespace: None })
        } else {
            None
        };
        self.semi();
        def
    }

    fn from(&mut self) -> Option<String> {
        self.eat(&Token::KwFrom);
        match self.cur() {
            Token::String(s) => { let v = s.clone(); self.adv(); Some(v) }
            _ => None,
        }
    }

    fn block_body(&mut self) -> Vec<Statement> {
        let mut s = Vec::new();
        while !matches!(self.cur(), Token::RBrace) {
            if self.eof() { break; }
            if let Some(st) = self.stmt() { s.push(st); } else { break; }
        }
        s
    }

    fn params(&mut self) -> Vec<String> {
        let mut p = Vec::new();
        while !matches!(self.cur(), Token::RParen) {
            if let Token::Identifier(n) = self.cur() {
                p.push(n.clone());
                self.adv();
            }
            if !matches!(self.cur(), Token::RParen) { self.eat(&Token::Comma); }
        }
        p
    }

    fn ident(&mut self) -> Option<String> {
        match self.cur() {
            Token::Identifier(n) => { let v = n.clone(); self.adv(); Some(v) }
            _ => None,
        }
    }

    fn expr(&mut self) -> Option<Expr> {
        self.assign()
    }

    fn assign(&mut self) -> Option<Expr> {
        let l = self.cond()?;
        match self.cur() {
            Token::Equal => { self.adv(); let v = self.assign()?; Some(Expr::Assignment { target: Box::new(l), op: "=".to_string(), value: Box::new(v) }) }
            Token::PlusEqual => { self.adv(); let v = self.assign()?; Some(Expr::Assignment { target: Box::new(l), op: "+=".to_string(), value: Box::new(v) }) }
            Token::MinusEqual => { self.adv(); let v = self.assign()?; Some(Expr::Assignment { target: Box::new(l), op: "-=".to_string(), value: Box::new(v) }) }
            Token::StarEqual => { self.adv(); let v = self.assign()?; Some(Expr::Assignment { target: Box::new(l), op: "*=".to_string(), value: Box::new(v) }) }
            Token::SlashEqual => { self.adv(); let v = self.assign()?; Some(Expr::Assignment { target: Box::new(l), op: "/=".to_string(), value: Box::new(v) }) }
            _ => Some(l),
        }
    }

    fn cond(&mut self) -> Option<Expr> {
        let t = self.or()?;
        if self.eat(&Token::Question) {
            let c = self.expr()?;
            self.eat(&Token::Colon);
            let a = self.expr()?;
            Some(Expr::Conditional { test: Box::new(t), consequent: Box::new(c), alternate: Box::new(a) })
        } else {
            Some(t)
        }
    }

    fn or(&mut self) -> Option<Expr> {
        let mut l = self.and()?;
        while self.eat(&Token::Or) {
            let r = self.and()?;
            l = Expr::Binary { op: "||".to_string(), left: Box::new(l), right: Box::new(r) };
        }
        Some(l)
    }

    fn and(&mut self) -> Option<Expr> {
        let mut l = self.eq()?;
        while self.eat(&Token::And) {
            let r = self.eq()?;
            l = Expr::Binary { op: "&&".to_string(), left: Box::new(l), right: Box::new(r) };
        }
        Some(l)
    }

    fn eq(&mut self) -> Option<Expr> {
        let mut l = self.cmp()?;
        loop {
            match self.cur() {
                Token::EqualEqual => { self.adv(); let r = self.cmp()?; l = Expr::Binary { op: "==".to_string(), left: Box::new(l), right: Box::new(r) }; }
                Token::NotEqual => { self.adv(); let r = self.cmp()?; l = Expr::Binary { op: "!=".to_string(), left: Box::new(l), right: Box::new(r) }; }
                Token::EqualEqualEqual => { self.adv(); let r = self.cmp()?; l = Expr::Binary { op: "===".to_string(), left: Box::new(l), right: Box::new(r) }; }
                Token::NotEqualEqual => { self.adv(); let r = self.cmp()?; l = Expr::Binary { op: "!==".to_string(), left: Box::new(l), right: Box::new(r) }; }
                _ => break,
            }
        }
        Some(l)
    }

    fn cmp(&mut self) -> Option<Expr> {
        let mut l = self.add()?;
        loop {
            match self.cur() {
                Token::Less => { self.adv(); let r = self.add()?; l = Expr::Binary { op: "<".to_string(), left: Box::new(l), right: Box::new(r) }; }
                Token::Greater => { self.adv(); let r = self.add()?; l = Expr::Binary { op: ">".to_string(), left: Box::new(l), right: Box::new(r) }; }
                Token::LessEqual => { self.adv(); let r = self.add()?; l = Expr::Binary { op: "<=".to_string(), left: Box::new(l), right: Box::new(r) }; }
                Token::GreaterEqual => { self.adv(); let r = self.add()?; l = Expr::Binary { op: ">=".to_string(), left: Box::new(l), right: Box::new(r) }; }
                Token::KwInstanceof => { self.adv(); let r = self.add()?; l = Expr::Binary { op: "instanceof".to_string(), left: Box::new(l), right: Box::new(r) }; }
                Token::KwIn => { self.adv(); let r = self.add()?; l = Expr::Binary { op: "in".to_string(), left: Box::new(l), right: Box::new(r) }; }
                _ => break,
            }
        }
        Some(l)
    }

    fn add(&mut self) -> Option<Expr> {
        let mut l = self.mul()?;
        loop {
            match self.cur() {
                Token::Plus => { self.adv(); let r = self.mul()?; l = Expr::Binary { op: "+".to_string(), left: Box::new(l), right: Box::new(r) }; }
                Token::Minus => { self.adv(); let r = self.mul()?; l = Expr::Binary { op: "-".to_string(), left: Box::new(l), right: Box::new(r) }; }
                _ => break,
            }
        }
        Some(l)
    }

    fn mul(&mut self) -> Option<Expr> {
        let mut l = self.unary()?;
        loop {
            match self.cur() {
                Token::Star => { self.adv(); let r = self.unary()?; l = Expr::Binary { op: "*".to_string(), left: Box::new(l), right: Box::new(r) }; }
                Token::Slash => { self.adv(); let r = self.unary()?; l = Expr::Binary { op: "/".to_string(), left: Box::new(l), right: Box::new(r) }; }
                Token::Percent => { self.adv(); let r = self.unary()?; l = Expr::Binary { op: "%".to_string(), left: Box::new(l), right: Box::new(r) }; }
                _ => break,
            }
        }
        Some(l)
    }

    fn unary(&mut self) -> Option<Expr> {
        match self.cur() {
            Token::Not => { self.adv(); let o = self.unary()?; Some(Expr::Unary { op: "!".to_string(), operand: Box::new(o), prefix: true }) }
            Token::Minus => { self.adv(); let o = self.unary()?; Some(Expr::Unary { op: "-".to_string(), operand: Box::new(o), prefix: true }) }
            Token::Plus => { self.adv(); let o = self.unary()?; Some(Expr::Unary { op: "+".to_string(), operand: Box::new(o), prefix: true }) }
            Token::KwTypeof => { self.adv(); let o = self.unary()?; Some(Expr::Unary { op: "typeof".to_string(), operand: Box::new(o), prefix: true }) }
            Token::KwVoid => { self.adv(); let o = self.unary()?; Some(Expr::Unary { op: "void".to_string(), operand: Box::new(o), prefix: true }) }
            Token::KwDelete => { self.adv(); let o = self.unary()?; Some(Expr::Unary { op: "delete".to_string(), operand: Box::new(o), prefix: true }) }
            Token::PlusPlus => { self.adv(); let o = self.unary()?; Some(Expr::Unary { op: "++".to_string(), operand: Box::new(o), prefix: true }) }
            Token::MinusMinus => { self.adv(); let o = self.unary()?; Some(Expr::Unary { op: "--".to_string(), operand: Box::new(o), prefix: true }) }
            _ => self.postfix(),
        }
    }

    fn postfix(&mut self) -> Option<Expr> {
        let mut e = self.primary()?;
        loop {
            match self.cur() {
                Token::LParen => {
                    self.adv();
                    let mut a = Vec::new();
                    while !matches!(self.cur(), Token::RParen) {
                        a.push(self.expr().unwrap_or(Expr::Undefined));
                        if !matches!(self.cur(), Token::RParen) { self.eat(&Token::Comma); }
                    }
                    self.eat(&Token::RParen);
                    e = Expr::Call { callee: Box::new(e), args: a };
                }
                Token::Dot => { self.adv(); let p = self.ident()?; e = Expr::Member { object: Box::new(e), property: Box::new(Expr::String(p)), computed: false }; }
                Token::LBracket => { self.adv(); let p = self.expr()?; self.eat(&Token::RBracket); e = Expr::Member { object: Box::new(e), property: Box::new(p), computed: true }; }
                Token::PlusPlus => { self.adv(); e = Expr::Unary { op: "++".to_string(), operand: Box::new(e), prefix: false }; }
                Token::MinusMinus => { self.adv(); e = Expr::Unary { op: "--".to_string(), operand: Box::new(e), prefix: false }; }
                _ => break,
            }
        }
        Some(e)
    }

    fn primary(&mut self) -> Option<Expr> {
        match self.cur() {
            Token::Number(n) => { let v = *n; self.adv(); Some(Expr::Number(v)) }
            Token::String(s) => { let v = s.clone(); self.adv(); Some(Expr::String(v)) }
            Token::KwTrue => { self.adv(); Some(Expr::Bool(true)) }
            Token::KwFalse => { self.adv(); Some(Expr::Bool(false)) }
            Token::KwNull => { self.adv(); Some(Expr::Null) }
            Token::KwUndefined => { self.adv(); Some(Expr::Undefined) }
            Token::KwThis => { self.adv(); Some(Expr::This) }
            Token::LParen => {
                self.adv();
                if self.eat(&Token::RParen) {
                    if self.eat(&Token::Arrow) { return Some(self.arrow_body(&vec![])); }
                    return Some(Expr::Undefined);
                }
                let f = self.expr()?;
                if self.eat(&Token::RParen) {
                    if self.eat(&Token::Arrow) {
                        let n = match &f { Expr::Identifier(x) => x.clone(), _ => return None };
                        return Some(self.arrow_body(&vec![n]));
                    }
                    return Some(f);
                }
                if self.eat(&Token::Comma) {
                    let mut p = vec![];
                    if let Expr::Identifier(n) = f { p.push(n); }
                    while self.eat(&Token::Comma) {
                        if let Token::Identifier(x) = self.cur() { p.push(x.clone()); self.adv(); }
                    }
                    self.eat(&Token::RParen);
                    if self.eat(&Token::Arrow) { return Some(self.arrow_body(&p)); }
                    return None;
                }
                let e = self.assign()?;
                self.eat(&Token::RParen);
                Some(e)
            }
            Token::LBracket => {
                self.adv();
                let mut i = Vec::new();
                while !matches!(self.cur(), Token::RBracket) {
                    if self.eat(&Token::Comma) { i.push(Expr::Undefined); continue; }
                    i.push(self.expr()?);
                    if !matches!(self.cur(), Token::RBracket) { self.eat(&Token::Comma); }
                }
                self.eat(&Token::RBracket);
                Some(Expr::Array(i))
            }
            Token::LBrace => {
                self.adv();
                let mut p = Vec::new();
                while !matches!(self.cur(), Token::RBrace) {
                    let k = match self.cur() {
                        Token::Identifier(n) => { let v = n.clone(); self.adv(); v }
                        Token::String(s) => { let v = s.clone(); self.adv(); v }
                        Token::Number(n) => { let v = n.to_string(); self.adv(); v }
                        _ => break,
                    };
                    self.eat(&Token::Colon);
                    let v = self.expr()?;
                    p.push((k, v));
                    if !matches!(self.cur(), Token::RBrace) { self.eat(&Token::Comma); }
                }
                self.eat(&Token::RBrace);
                Some(Expr::Object(p))
            }
            Token::KwFunction => {
                self.adv();
                let n = if let Token::Identifier(x) = self.cur() { let v = x.clone(); self.adv(); Some(v) } else { None };
                self.eat(&Token::LParen);
                let p = self.params();
                self.eat(&Token::RParen);
                self.eat(&Token::LBrace);
                let b = self.block_body();
                self.eat(&Token::RBrace);
                Some(Expr::FnExpr { name: n, params: p, body: b })
            }
            Token::KwNew => {
                self.adv();
                let c = self.expr()?;
                let a = if self.eat(&Token::LParen) {
                    let mut ag = Vec::new();
                    while !matches!(self.cur(), Token::RParen) {
                        ag.push(self.expr().unwrap_or(Expr::Undefined));
                        if !matches!(self.cur(), Token::RParen) { self.eat(&Token::Comma); }
                    }
                    self.eat(&Token::RParen);
                    ag
                } else {
                    vec![]
                };
                Some(Expr::New { callee: Box::new(c), args: a })
            }
            Token::KwImport => {
                self.adv();
                if self.eat(&Token::Dot) {
                    if let Token::Identifier(m) = self.cur() {
                        if m == "meta" { self.adv(); return Some(Expr::ImportMeta); }
                    }
                }
                self.semi();
                Some(Expr::Undefined)
            }
            Token::Identifier(n) => { let nm = n.clone(); self.adv(); Some(Expr::Identifier(nm)) }
            Token::DotDotDot => { self.adv(); let i = self.expr()?; Some(Expr::Spread(Box::new(i))) }
            _ => None,
        }
    }

    fn arrow_body(&mut self, p: &[String]) -> Expr {
        if self.eat(&Token::LBrace) {
            let b = self.block_body();
            self.eat(&Token::RBrace);
            Expr::ArrowFn { params: p.to_vec(), body: Box::new(ExprOrBlock::Block(b)) }
        } else {
            let e = self.expr().unwrap_or(Expr::Undefined);
            Expr::ArrowFn { params: p.to_vec(), body: Box::new(ExprOrBlock::Expr(Box::new(e))) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(src: &str) -> Vec<Statement> {
        let mut lex = Lexer::new(src);
        let toks = lex.tokenize();
        let mut parser = Parser::new(toks);
        parser.parse()
    }

    #[test]
    fn test_var_decl() {
        let stmts = parse("const x = 42;");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::VarDecl { kind: VarKind::Const, name, .. } if name == "x"));
    }

    #[test]
    fn test_fn_decl() {
        let stmts = parse("function add(a, b) { return a + b; }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::FnDecl { name, params, .. } if name == "add" && params.len() == 2));
    }

    #[test]
    fn test_if_else() {
        let stmts = parse("if (true) { 1; } else { 2; }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::If { else_: Some(_), .. }));
    }

    #[test]
    fn test_for_loop() {
        let stmts = parse("for (let i = 0; i < 10; i++) { i; }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::For { .. }));
    }

    #[test]
    fn test_while_loop() {
        let stmts = parse("while (true) { break; }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::While { .. }));
    }

    #[test]
    fn test_arrow_fn() {
        let stmts = parse("const f = (x) => x * 2;");
        assert_eq!(stmts.len(), 1);
        if let Statement::VarDecl { init: Some(init), .. } = &stmts[0] {
            assert!(matches!(init.as_ref(), Expr::ArrowFn { .. }));
        } else {
            panic!("expected var decl with init");
        }
    }

    #[test]
    fn test_class_decl() {
        let stmts = parse("class Foo { constructor() {} bar() {} }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::ClassDecl { name, body, .. } if name == "Foo" && body.len() == 2));
    }

    #[test]
    fn test_try_catch() {
        let stmts = parse("try { throw 'x'; } catch(e) { e; }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::Try { catch: Some(_), .. }));
    }

    #[test]
    fn test_switch() {
        let stmts = parse("switch (x) { case 1: break; default: break; }");
        assert_eq!(stmts.len(), 1);
        if let Statement::Switch { cases, .. } = &stmts[0] {
            assert_eq!(cases.len(), 2);
        } else {
            panic!("expected switch");
        }
    }

    #[test]
    fn test_import() {
        let stmts = parse("import { foo } from 'bar';");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::Import { module, named, .. } if module == "bar" && named.len() == 1));
    }

    #[test]
    fn test_export_default() {
        let stmts = parse("export default 42;");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::ExportDefault(_)));
    }

    #[test]
    fn test_binary_precedence() {
        let stmts = parse("1 + 2 * 3;");
        assert_eq!(stmts.len(), 1);
        if let Statement::Expr(Expr::Binary { op, right, .. }) = &stmts[0] {
            assert_eq!(op, "+");
            assert!(matches!(right.as_ref(), Expr::Binary { op, .. } if op == "*"));
        } else {
            panic!("expected binary expr");
        }
    }

    #[test]
    fn test_ternary() {
        let stmts = parse("true ? 1 : 2;");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::Expr(Expr::Conditional { .. })));
    }

    #[test]
    fn test_member_access() {
        let stmts = parse("obj.prop;");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::Expr(Expr::Member { computed: false, .. })));
    }

    #[test]
    fn test_computed_member() {
        let stmts = parse("arr[0];");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::Expr(Expr::Member { computed: true, .. })));
    }

    #[test]
    fn test_new_expr() {
        let stmts = parse("new Foo(1, 2);");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::Expr(Expr::New { .. })));
    }

    #[test]
    fn test_for_of() {
        let stmts = parse("for (const x of arr) { x; }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::ForOf { .. }));
    }

    #[test]
    fn test_for_in() {
        let stmts = parse("for (const k in obj) { k; }");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Statement::ForIn { .. }));
    }
}
