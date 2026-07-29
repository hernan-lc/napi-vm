use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt;

use napi_derive::napi;

#[derive(Debug)]
pub(crate) enum VmErr { Ret(Value), Throw(String), Msg(String) }
impl fmt::Display for VmErr { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { match self { VmErr::Msg(s) => write!(f, "{}", s), VmErr::Throw(s) => write!(f, "{}", s), VmErr::Ret(_) => write!(f, "return") } } }
fn vm_ret(v: Value) -> Result<Value, VmErr> { Err(VmErr::Ret(v)) }
fn vm_throw<T: Into<String>>(msg: T) -> Result<Value, VmErr> { Err(VmErr::Throw(msg.into())) }
fn vm_err<T: Into<String>>(msg: T) -> Result<Value, VmErr> { Err(VmErr::Msg(msg.into())) }

// ============================================================
// VALUE
// ============================================================

#[derive(Debug, Clone)]
pub enum Value {
    Undefined, Null, Bool(bool), Number(f64), String(String),
    Object(Vec<(String, Value)>), Array(Vec<Value>),
    Function { name: Option<String>, params: Vec<String>, body: Vec<Statement>, closure: Option<Env> },
    NativeFunction { name: String, callable: fn(Vec<Value>) -> Result<Value, VmErr> },
}

// ============================================================
// LEXER
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64), String(String), Identifier(String),
    Plus, Minus, Star, Slash, Percent, PlusPlus, MinusMinus,
    Equal, PlusEqual, MinusEqual, StarEqual, SlashEqual,
    EqualEqual, NotEqual, EqualEqualEqual, NotEqualEqual,
    Less, Greater, LessEqual, GreaterEqual,
    And, Or, Not,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Semicolon, Comma, Dot, Colon, Question, Arrow, DotDotDot,
    KwVar, KwLet, KwConst, KwFunction, KwReturn,
    KwIf, KwElse, KwFor, KwWhile, KwDo, KwSwitch, KwCase, KwDefault, KwBreak, KwContinue,
    KwClass, KwExtends, KwNew, KwThis, KwSuper,
    KwImport, KwExport, KwFrom, KwAs,
    KwAsync, KwAwait, KwTry, KwCatch, KwFinally, KwThrow,
    KwTypeof, KwInstanceof, KwIn, KwOf,
    KwTrue, KwFalse, KwNull, KwUndefined, KwDelete, KwVoid,
    KwStatic, KwGet, KwSet, KwConstructor,
    EOF,
}

pub struct Lexer { src: Vec<char>, pos: usize }
impl Lexer {
    pub fn new(s: &str) -> Self { Self { src: s.chars().collect(), pos: 0 } }
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut toks = Vec::new();
        while self.pos < self.src.len() {
            self.skip_ws();
            if self.pos >= self.src.len() { break; }
            if let Some(t) = self.next() { toks.push(t); }
        }
        toks.push(Token::EOF); toks
    }
    fn skip_ws(&mut self) {
        while self.pos < self.src.len() {
            let c = self.src[self.pos];
            if c.is_whitespace() { self.pos += 1; }
            else if c == '/' && self.pos + 1 < self.src.len() {
                let n = self.src[self.pos + 1];
                if n == '/' { self.pos += 2; while self.pos < self.src.len() && self.src[self.pos] != '\n' { self.pos += 1; } }
                else if n == '*' { self.pos += 2; while self.pos + 1 < self.src.len() { if self.src[self.pos] == '*' && self.src[self.pos + 1] == '/' { self.pos += 2; break; } self.pos += 1; } }
                else { break; }
            } else { break; }
        }
    }
    fn next(&mut self) -> Option<Token> {
        let c = *self.src.get(self.pos)?;
        Some(match c {
            '(' => { self.pos += 1; Token::LParen } ')' => { self.pos += 1; Token::RParen }
            '{' => { self.pos += 1; Token::LBrace } '}' => { self.pos += 1; Token::RBrace }
            '[' => { self.pos += 1; Token::LBracket } ']' => { self.pos += 1; Token::RBracket }
            ';' => { self.pos += 1; Token::Semicolon } ',' => { self.pos += 1; Token::Comma }
            ':' => { self.pos += 1; Token::Colon } '?' => { self.pos += 1; Token::Question }
            '.' => {
                if self.pos + 2 < self.src.len() && self.src[self.pos + 1] == '.' && self.src[self.pos + 2] == '.' {
                    self.pos += 3; Token::DotDotDot
                } else { self.pos += 1; Token::Dot }
            }
            '+' => match self.src.get(self.pos + 1) { Some('+') => { self.pos += 2; Token::PlusPlus }, Some('=') => { self.pos += 2; Token::PlusEqual }, _ => { self.pos += 1; Token::Plus } }
            '-' => match self.src.get(self.pos + 1) { Some('-') => { self.pos += 2; Token::MinusMinus }, Some('=') => { self.pos += 2; Token::MinusEqual }, _ => { self.pos += 1; Token::Minus } }
            '*' => match self.src.get(self.pos + 1) { Some('=') => { self.pos += 2; Token::StarEqual }, _ => { self.pos += 1; Token::Star } }
            '/' => match self.src.get(self.pos + 1) { Some('=') => { self.pos += 2; Token::SlashEqual }, _ => { self.pos += 1; Token::Slash } }
            '%' => { self.pos += 1; Token::Percent }
            '=' => match (self.src.get(self.pos + 1), self.src.get(self.pos + 2)) {
                (Some('='), Some('=')) => { self.pos += 3; Token::EqualEqualEqual }
                (Some('='), _) => { self.pos += 2; Token::EqualEqual }
                (Some('>'), _) => { self.pos += 2; Token::Arrow }
                _ => { self.pos += 1; Token::Equal }
            }
            '!' => match (self.src.get(self.pos + 1), self.src.get(self.pos + 2)) {
                (Some('='), Some('=')) => { self.pos += 3; Token::NotEqualEqual }
                (Some('='), _) => { self.pos += 2; Token::NotEqual }
                _ => { self.pos += 1; Token::Not }
            }
            '<' => match self.src.get(self.pos + 1) { Some('<') => { self.pos += 2; Token::Less }, Some('=') => { self.pos += 2; Token::LessEqual }, _ => { self.pos += 1; Token::Less } }
            '>' => match self.src.get(self.pos + 1) { Some('>') => { self.pos += 2; Token::Greater }, Some('=') => { self.pos += 2; Token::GreaterEqual }, _ => { self.pos += 1; Token::Greater } }
            '&' => match self.src.get(self.pos + 1) { Some('&') => { self.pos += 2; Token::And }, _ => { self.pos += 1; Token::And } }
            '|' => match self.src.get(self.pos + 1) { Some('|') => { self.pos += 2; Token::Or }, _ => { self.pos += 1; Token::Or } }
            '"' | '\'' => self.read_str(c),
            c if c.is_ascii_digit() => self.read_num(),
            c if c.is_ascii_alphabetic() || c == '_' || c == '$' => self.read_ident(),
            _ => { self.pos += 1; return None; }
        })
    }
    fn read_str(&mut self, q: char) -> Token {
        self.pos += 1; let mut s = String::new();
        while self.pos < self.src.len() {
            let c = self.src[self.pos];
            if c == q { self.pos += 1; break; }
            if c == '\\' && self.pos + 1 < self.src.len() { self.pos += 1; match self.src[self.pos] { 'n' => s.push('\n'), 't' => s.push('\t'), 'r' => s.push('\r'), '\\' => s.push('\\'), '"' => s.push('"'), '\'' => s.push('\''), '0' => s.push('\0'), o => s.push(o) } } else { s.push(c); }
            self.pos += 1;
        }
        Token::String(s)
    }
    fn read_num(&mut self) -> Token {
        let s = self.pos;
        while self.pos < self.src.len() && (self.src[self.pos].is_ascii_digit() || self.src[self.pos] == '_') { self.pos += 1; }
        if self.pos < self.src.len() && self.src[self.pos] == '.' { self.pos += 1; while self.pos < self.src.len() && (self.src[self.pos].is_ascii_digit() || self.src[self.pos] == '_') { self.pos += 1; } }
        let n: String = self.src[s..self.pos].iter().filter(|c| **c != '_').collect();
        Token::Number(n.parse().unwrap_or(0.0))
    }
    fn read_ident(&mut self) -> Token {
        let s = self.pos;
        while self.pos < self.src.len() { let c = self.src[self.pos]; if !c.is_ascii_alphanumeric() && c != '_' && c != '$' { break; } self.pos += 1; }
        let i: String = self.src[s..self.pos].iter().collect();
        match i.as_str() {
            "var" => Token::KwVar, "let" => Token::KwLet, "const" => Token::KwConst, "function" => Token::KwFunction, "return" => Token::KwReturn,
            "if" => Token::KwIf, "else" => Token::KwElse, "for" => Token::KwFor, "while" => Token::KwWhile, "do" => Token::KwDo,
            "switch" => Token::KwSwitch, "case" => Token::KwCase, "default" => Token::KwDefault, "break" => Token::KwBreak, "continue" => Token::KwContinue,
            "class" => Token::KwClass, "extends" => Token::KwExtends, "new" => Token::KwNew, "this" => Token::KwThis, "super" => Token::KwSuper,
            "import" => Token::KwImport, "export" => Token::KwExport, "from" => Token::KwFrom, "as" => Token::KwAs,
            "async" => Token::KwAsync, "await" => Token::KwAwait, "try" => Token::KwTry, "catch" => Token::KwCatch, "finally" => Token::KwFinally, "throw" => Token::KwThrow,
            "typeof" => Token::KwTypeof, "instanceof" => Token::KwInstanceof, "in" => Token::KwIn, "of" => Token::KwOf,
            "true" => Token::KwTrue, "false" => Token::KwFalse, "null" => Token::KwNull, "undefined" => Token::KwUndefined,
            "delete" => Token::KwDelete, "void" => Token::KwVoid, "static" => Token::KwStatic, "get" => Token::KwGet, "set" => Token::KwSet,
            "constructor" => Token::KwConstructor, _ => Token::Identifier(i),
        }
    }
}

// ============================================================
// PARSER
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64), String(String), Bool(bool), Null, Undefined, Identifier(String),
    Array(Vec<Expr>), Object(Vec<(String, Expr)>),
    Binary { op: String, left: Box<Expr>, right: Box<Expr> },
    Unary { op: String, operand: Box<Expr>, prefix: bool },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Member { object: Box<Expr>, property: Box<Expr>, computed: bool },
    Assignment { target: Box<Expr>, op: String, value: Box<Expr> },
    Conditional { test: Box<Expr>, consequent: Box<Expr>, alternate: Box<Expr> },
    ArrowFn { params: Vec<String>, body: Box<ExprOrBlock> },
    FnExpr { name: Option<String>, params: Vec<String>, body: Vec<Statement> },
    New { callee: Box<Expr>, args: Vec<Expr> }, Spread(Box<Expr>), This, ImportMeta,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ExprOrBlock { Expr(Box<Expr>), Block(Vec<Statement>) }

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expr(Expr), VarDecl { kind: VarKind, name: String, init: Option<Box<Expr>> },
    FnDecl { name: String, params: Vec<String>, body: Vec<Statement> },
    ClassDecl { name: String, superclass: Option<Box<Expr>>, body: Vec<ClassMember> },
    Return(Option<Box<Expr>>), If { test: Box<Expr>, then: Vec<Statement>, else_: Option<Vec<Statement>> },
    While { test: Box<Expr>, body: Vec<Statement> },
    For { init: Option<Box<ForInit>>, test: Option<Box<Expr>>, update: Option<Box<Expr>>, body: Vec<Statement> },
    ForIn { name: String, obj: Box<Expr>, body: Vec<Statement> }, ForOf { name: String, iter: Box<Expr>, body: Vec<Statement> },
    Block(Vec<Statement>), Break, Continue, Throw(Box<Expr>),
    Try { body: Vec<Statement>, catch: Option<(String, Vec<Statement>)>, finally: Option<Vec<Statement>> },
    Switch { disc: Box<Expr>, cases: Vec<SwitchCase> },
    ExportDefault(Box<Expr>), ExportNamed { specifiers: Vec<(String, String)>, source: Option<String> },
    Import { module: String, default: Option<String>, named: Vec<(String, String)>, namespace: Option<String> }, Empty,
}
#[derive(Debug, Clone, PartialEq)]
pub enum VarKind { Var, Let, Const }
#[derive(Debug, Clone, PartialEq)]
pub enum ForInit { Var { kind: VarKind, name: String, init: Option<Box<Expr>> }, Expr(Expr) }
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase { pub test: Option<Expr>, pub body: Vec<Statement> }
#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember { Method { name: String, is_static: bool, params: Vec<String>, body: Vec<Statement> }, Field { name: String, is_static: bool, init: Option<Expr> } }

pub struct Parser { toks: Vec<Token>, pos: usize }
impl Parser {
    pub fn new(t: Vec<Token>) -> Self { Self { toks: t, pos: 0 } }
    pub fn parse(&mut self) -> Vec<Statement> {
        let mut s = Vec::new();
        while !self.eof() { if let Some(st) = self.stmt() { s.push(st); } else { self.adv(); } } s
    }
    fn cur(&self) -> &Token { self.toks.get(self.pos).unwrap_or(&Token::EOF) }
    fn adv(&mut self) -> &Token { if self.pos < self.toks.len() { self.pos += 1; } self.toks.get(self.pos - 1).unwrap_or(&Token::EOF) }
    fn eat(&mut self, t: &Token) -> bool { if self.cur() == t { self.pos += 1; true } else { false } }
    fn eof(&self) -> bool { matches!(self.cur(), Token::EOF) }
    fn semi(&mut self) { if matches!(self.cur(), Token::Semicolon) { self.pos += 1; } }

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
        self.adv(); let n = self.ident()?; let i = if self.eat(&Token::Equal) { Some(Box::new(self.expr()?)) } else { None }; self.semi();
        Some(Statement::VarDecl { kind: k, name: n, init: i })
    }
    fn fn_decl(&mut self) -> Option<Statement> {
        self.adv(); let n = self.ident()?; self.eat(&Token::LParen); let p = self.params(); self.eat(&Token::RParen); self.eat(&Token::LBrace); let b = self.block_body(); self.eat(&Token::RBrace);
        Some(Statement::FnDecl { name: n, params: p, body: b })
    }
    fn class_decl(&mut self) -> Option<Statement> {
        self.adv(); let n = self.ident()?;
        let sc = if self.eat(&Token::KwExtends) { Some(Box::new(self.expr()?)) } else { None };
        self.eat(&Token::LBrace); let mut b = Vec::new();
        while !matches!(self.cur(), Token::RBrace) {
            if self.eof() { break; }
            let st = if self.eat(&Token::KwStatic) { true } else { false };
            let mn = match self.cur() { Token::Identifier(x) => { let v = x.clone(); self.adv(); v }, Token::KwConstructor => { self.adv(); "constructor".to_string() }, _ => return None };
            if self.eat(&Token::LParen) {
                let p = self.params(); self.eat(&Token::RParen); self.eat(&Token::LBrace); let bd = self.block_body(); self.eat(&Token::RBrace);
                b.push(ClassMember::Method { name: mn, is_static: st, params: p, body: bd });
            } else {
                let i = if self.eat(&Token::Equal) { Some(self.expr()?) } else { None }; self.semi();
                b.push(ClassMember::Field { name: mn, is_static: st, init: i });
            }
        }
        self.eat(&Token::RBrace); Some(Statement::ClassDecl { name: n, superclass: sc, body: b })
    }
    fn ret(&mut self) -> Option<Statement> { self.adv(); let e = if matches!(self.cur(), Token::Semicolon) || matches!(self.cur(), Token::RBrace) { None } else { Some(Box::new(self.expr()?)) }; self.semi(); Some(Statement::Return(e)) }
    fn if_(&mut self) -> Option<Statement> {
        self.adv(); self.eat(&Token::LParen); let t = Box::new(self.expr()?); self.eat(&Token::RParen); self.eat(&Token::LBrace); let c = self.block_body(); self.eat(&Token::RBrace);
        let a = if self.eat(&Token::KwElse) { if matches!(self.cur(), Token::KwIf) { Some(vec![self.if_()?]) } else { self.eat(&Token::LBrace); let b = self.block_body(); self.eat(&Token::RBrace); Some(b) } } else { None };
        Some(Statement::If { test: t, then: c, else_: a })
    }
    fn while_(&mut self) -> Option<Statement> { self.adv(); self.eat(&Token::LParen); let t = Box::new(self.expr()?); self.eat(&Token::RParen); self.eat(&Token::LBrace); let b = self.block_body(); self.eat(&Token::RBrace); Some(Statement::While { test: t, body: b }) }
    fn for_(&mut self) -> Option<Statement> {
        self.adv(); self.eat(&Token::LParen);
        let init = if self.eat(&Token::KwVar) { let n = self.ident()?; let i = if self.eat(&Token::Equal) { Some(Box::new(self.expr()?)) } else { None }; Some(Box::new(ForInit::Var { kind: VarKind::Var, name: n, init: i })) }
        else if self.eat(&Token::KwLet) { let n = self.ident()?; let i = if self.eat(&Token::Equal) { Some(Box::new(self.expr()?)) } else { None }; Some(Box::new(ForInit::Var { kind: VarKind::Let, name: n, init: i })) }
        else if self.eat(&Token::KwConst) { let n = self.ident()?; let i = if self.eat(&Token::Equal) { Some(Box::new(self.expr()?)) } else { None }; Some(Box::new(ForInit::Var { kind: VarKind::Const, name: n, init: i })) }
        else if matches!(self.cur(), Token::Semicolon) { None } else { Some(Box::new(ForInit::Expr(self.expr()?))) };
        if init.is_some() && !matches!(self.cur(), Token::Semicolon) {
            if self.eat(&Token::KwIn) { let o = Box::new(self.expr()?); self.eat(&Token::RParen); self.eat(&Token::LBrace); let b = self.block_body(); self.eat(&Token::RBrace); let n = match init.unwrap().as_ref() { ForInit::Var { name, .. } => name.clone(), _ => return None }; return Some(Statement::ForIn { name: n, obj: o, body: b }); }
            if self.eat(&Token::KwOf) { let i = Box::new(self.expr()?); self.eat(&Token::RParen); self.eat(&Token::LBrace); let b = self.block_body(); self.eat(&Token::RBrace); let n = match init.unwrap().as_ref() { ForInit::Var { name, .. } => name.clone(), _ => return None }; return Some(Statement::ForOf { name: n, iter: i, body: b }); }
        }
        self.semi(); let t = if !matches!(self.cur(), Token::Semicolon) { Some(Box::new(self.expr()?)) } else { None }; self.semi();
        let u = if !matches!(self.cur(), Token::RParen) { Some(Box::new(self.expr()?)) } else { None }; self.eat(&Token::RParen); self.eat(&Token::LBrace); let b = self.block_body(); self.eat(&Token::RBrace);
        Some(Statement::For { init, test: t, update: u, body: b })
    }
    fn throw(&mut self) -> Option<Statement> { self.adv(); let e = self.expr()?; self.semi(); Some(Statement::Throw(Box::new(e))) }
    fn try_(&mut self) -> Option<Statement> {
        self.adv(); self.eat(&Token::LBrace); let b = self.block_body(); self.eat(&Token::RBrace);
        let c = if self.eat(&Token::KwCatch) {
            let p = if self.eat(&Token::LParen) { let x = self.ident()?; self.eat(&Token::RParen); x } else { String::new() };
            self.eat(&Token::LBrace); let cb = self.block_body(); self.eat(&Token::RBrace); Some((p, cb))
        } else { None };
        let f = if self.eat(&Token::KwFinally) { self.eat(&Token::LBrace); let fb = self.block_body(); self.eat(&Token::RBrace); Some(fb) } else { None };
        Some(Statement::Try { body: b, catch: c, finally: f })
    }
    fn switch(&mut self) -> Option<Statement> {
        self.adv(); self.eat(&Token::LParen); let d = Box::new(self.expr()?); self.eat(&Token::RParen); self.eat(&Token::LBrace);
        let mut cs = Vec::new();
        while !matches!(self.cur(), Token::RBrace) {
            if self.eof() { break; }
            let t = if self.eat(&Token::KwCase) { let e = self.expr()?; self.eat(&Token::Colon); Some(e) } else if self.eat(&Token::KwDefault) { self.eat(&Token::Colon); None } else { break; };
            let mut b = Vec::new();
            while !matches!(self.cur(), Token::KwCase) && !matches!(self.cur(), Token::KwDefault) && !matches!(self.cur(), Token::RBrace) { if self.eof() { break; } b.push(self.stmt()?); }
            cs.push(SwitchCase { test: t, body: b });
        }
        self.eat(&Token::RBrace); Some(Statement::Switch { disc: d, cases: cs })
    }
    fn export(&mut self) -> Option<Statement> {
        self.adv();
        if self.eat(&Token::KwDefault) { let e = self.expr()?; self.semi(); Some(Statement::ExportDefault(Box::new(e))) }
        else if self.eat(&Token::LBrace) {
            let mut sp = Vec::new();
            while !matches!(self.cur(), Token::RBrace) { let l = self.ident()?; let e = if self.eat(&Token::KwAs) { self.ident()? } else { l.clone() }; sp.push((l, e)); if !matches!(self.cur(), Token::RBrace) { self.eat(&Token::Comma); } }
            self.eat(&Token::RBrace); let s = if self.eat(&Token::KwFrom) { match self.cur() { Token::String(x) => { let v = x.clone(); self.adv(); Some(v) }, _ => None } } else { None }; self.semi();
            Some(Statement::ExportNamed { specifiers: sp, source: s })
        } else { Some(Statement::ExportNamed { specifiers: vec![], source: None }) }
    }
    fn import(&mut self) -> Option<Statement> {
        self.adv();
        let def = if let Token::Identifier(n) = self.cur() { let nm = n.clone(); self.adv();
            if self.eat(&Token::Comma) {
                if self.eat(&Token::LBrace) { let mut nd = Vec::new(); while !matches!(self.cur(), Token::RBrace) { let l = self.ident()?; let i = if self.eat(&Token::KwAs) { self.ident()? } else { l.clone() }; nd.push((l, i)); if !matches!(self.cur(), Token::RBrace) { self.eat(&Token::Comma); } } self.eat(&Token::RBrace); let m = self.from()?; Some(Statement::Import { module: m, default: Some(nm), named: nd, namespace: None }) } else { None }
            } else if self.eat(&Token::KwFrom) { let m = self.from()?; Some(Statement::Import { module: m, default: Some(nm), named: vec![], namespace: None }) } else { None }
        } else if self.eat(&Token::Star) { self.eat(&Token::KwAs); let ns = self.ident()?; let m = self.from()?; Some(Statement::Import { module: m, default: None, named: vec![], namespace: Some(ns) }) }
        else if self.eat(&Token::LBrace) { let mut nd = Vec::new(); while !matches!(self.cur(), Token::RBrace) { let l = self.ident()?; let i = if self.eat(&Token::KwAs) { self.ident()? } else { l.clone() }; nd.push((l, i)); if !matches!(self.cur(), Token::RBrace) { self.eat(&Token::Comma); } } self.eat(&Token::RBrace); let m = self.from()?; Some(Statement::Import { module: m, default: None, named: nd, namespace: None }) }
        else if let Token::String(s) = self.cur() { let m = s.clone(); self.adv(); self.semi(); Some(Statement::Import { module: m, default: None, named: vec![], namespace: None }) } else { None };
        self.semi(); def
    }
    fn from(&mut self) -> Option<String> { self.eat(&Token::KwFrom); match self.cur() { Token::String(s) => { let v = s.clone(); self.adv(); Some(v) }, _ => None } }
    fn block_body(&mut self) -> Vec<Statement> { let mut s = Vec::new(); while !matches!(self.cur(), Token::RBrace) { if self.eof() { break; } if let Some(st) = self.stmt() { s.push(st); } else { break; } } s }
    fn params(&mut self) -> Vec<String> { let mut p = Vec::new(); while !matches!(self.cur(), Token::RParen) { if let Token::Identifier(n) = self.cur() { p.push(n.clone()); self.adv(); } if !matches!(self.cur(), Token::RParen) { self.eat(&Token::Comma); } } p }
    fn ident(&mut self) -> Option<String> { match self.cur() { Token::Identifier(n) => { let v = n.clone(); self.adv(); Some(v) }, _ => None } }
    fn expr(&mut self) -> Option<Expr> { self.assign() }
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
    fn cond(&mut self) -> Option<Expr> { let t = self.or()?; if self.eat(&Token::Question) { let c = self.expr()?; self.eat(&Token::Colon); let a = self.expr()?; Some(Expr::Conditional { test: Box::new(t), consequent: Box::new(c), alternate: Box::new(a) }) } else { Some(t) } }
    fn or(&mut self) -> Option<Expr> { let mut l = self.and()?; while self.eat(&Token::Or) { let r = self.and()?; l = Expr::Binary { op: "||".to_string(), left: Box::new(l), right: Box::new(r) }; } Some(l) }
    fn and(&mut self) -> Option<Expr> { let mut l = self.eq()?; while self.eat(&Token::And) { let r = self.eq()?; l = Expr::Binary { op: "&&".to_string(), left: Box::new(l), right: Box::new(r) }; } Some(l) }
    fn eq(&mut self) -> Option<Expr> { let mut l = self.cmp()?;
        loop { match self.cur() {
            Token::EqualEqual => { self.adv(); let r = self.cmp()?; l = Expr::Binary { op: "==".to_string(), left: Box::new(l), right: Box::new(r) }; }
            Token::NotEqual => { self.adv(); let r = self.cmp()?; l = Expr::Binary { op: "!=".to_string(), left: Box::new(l), right: Box::new(r) }; }
            Token::EqualEqualEqual => { self.adv(); let r = self.cmp()?; l = Expr::Binary { op: "===".to_string(), left: Box::new(l), right: Box::new(r) }; }
            Token::NotEqualEqual => { self.adv(); let r = self.cmp()?; l = Expr::Binary { op: "!==".to_string(), left: Box::new(l), right: Box::new(r) }; }
            _ => break,
        } } Some(l)
    }
    fn cmp(&mut self) -> Option<Expr> { let mut l = self.add()?;
        loop { match self.cur() {
            Token::Less => { self.adv(); let r = self.add()?; l = Expr::Binary { op: "<".to_string(), left: Box::new(l), right: Box::new(r) }; }
            Token::Greater => { self.adv(); let r = self.add()?; l = Expr::Binary { op: ">".to_string(), left: Box::new(l), right: Box::new(r) }; }
            Token::LessEqual => { self.adv(); let r = self.add()?; l = Expr::Binary { op: "<=".to_string(), left: Box::new(l), right: Box::new(r) }; }
            Token::GreaterEqual => { self.adv(); let r = self.add()?; l = Expr::Binary { op: ">=".to_string(), left: Box::new(l), right: Box::new(r) }; }
            Token::KwInstanceof => { self.adv(); let r = self.add()?; l = Expr::Binary { op: "instanceof".to_string(), left: Box::new(l), right: Box::new(r) }; }
            Token::KwIn => { self.adv(); let r = self.add()?; l = Expr::Binary { op: "in".to_string(), left: Box::new(l), right: Box::new(r) }; }
            _ => break,
        } } Some(l)
    }
    fn add(&mut self) -> Option<Expr> { let mut l = self.mul()?;
        loop { match self.cur() {
            Token::Plus => { self.adv(); let r = self.mul()?; l = Expr::Binary { op: "+".to_string(), left: Box::new(l), right: Box::new(r) }; }
            Token::Minus => { self.adv(); let r = self.mul()?; l = Expr::Binary { op: "-".to_string(), left: Box::new(l), right: Box::new(r) }; }
            _ => break,
        } } Some(l)
    }
    fn mul(&mut self) -> Option<Expr> { let mut l = self.unary()?;
        loop { match self.cur() {
            Token::Star => { self.adv(); let r = self.unary()?; l = Expr::Binary { op: "*".to_string(), left: Box::new(l), right: Box::new(r) }; }
            Token::Slash => { self.adv(); let r = self.unary()?; l = Expr::Binary { op: "/".to_string(), left: Box::new(l), right: Box::new(r) }; }
            Token::Percent => { self.adv(); let r = self.unary()?; l = Expr::Binary { op: "%".to_string(), left: Box::new(l), right: Box::new(r) }; }
            _ => break,
        } } Some(l)
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
        loop { match self.cur() {
            Token::LParen => { self.adv(); let mut a = Vec::new(); while !matches!(self.cur(), Token::RParen) { a.push(self.expr().unwrap_or(Expr::Undefined)); if !matches!(self.cur(), Token::RParen) { self.eat(&Token::Comma); } } self.eat(&Token::RParen); e = Expr::Call { callee: Box::new(e), args: a }; }
            Token::Dot => { self.adv(); let p = self.ident()?; e = Expr::Member { object: Box::new(e), property: Box::new(Expr::String(p)), computed: false }; }
            Token::LBracket => { self.adv(); let p = self.expr()?; self.eat(&Token::RBracket); e = Expr::Member { object: Box::new(e), property: Box::new(p), computed: true }; }
            Token::PlusPlus => { self.adv(); e = Expr::Unary { op: "++".to_string(), operand: Box::new(e), prefix: false }; }
            Token::MinusMinus => { self.adv(); e = Expr::Unary { op: "--".to_string(), operand: Box::new(e), prefix: false }; }
            _ => break,
        } } Some(e)
    }
    fn primary(&mut self) -> Option<Expr> {
        match self.cur() {
            Token::Number(n) => { let v = *n; self.adv(); Some(Expr::Number(v)) }
            Token::String(s) => { let v = s.clone(); self.adv(); Some(Expr::String(v)) }
            Token::KwTrue => { self.adv(); Some(Expr::Bool(true)) } Token::KwFalse => { self.adv(); Some(Expr::Bool(false)) }
            Token::KwNull => { self.adv(); Some(Expr::Null) } Token::KwUndefined => { self.adv(); Some(Expr::Undefined) }
            Token::KwThis => { self.adv(); Some(Expr::This) }
            Token::LParen => {
                self.adv();
                if self.eat(&Token::RParen) { if self.eat(&Token::Arrow) { return Some(self.arrow_body(&vec![])); } return Some(Expr::Undefined); }
                let f = self.expr()?;
                if self.eat(&Token::RParen) { if self.eat(&Token::Arrow) { let n = match &f { Expr::Identifier(x) => x.clone(), _ => return None }; return Some(self.arrow_body(&vec![n])); } return Some(f); }
                if self.eat(&Token::Comma) {
                    let mut p = vec![]; if let Expr::Identifier(n) = f { p.push(n); }
                    while self.eat(&Token::Comma) { if let Token::Identifier(x) = self.cur() { p.push(x.clone()); self.adv(); } }
                    self.eat(&Token::RParen); if self.eat(&Token::Arrow) { return Some(self.arrow_body(&p)); } return None;
                }
                let e = self.assign()?; self.eat(&Token::RParen); Some(e)
            }
            Token::LBracket => { self.adv(); let mut i = Vec::new(); while !matches!(self.cur(), Token::RBracket) { if self.eat(&Token::Comma) { i.push(Expr::Undefined); continue; } i.push(self.expr()?); if !matches!(self.cur(), Token::RBracket) { self.eat(&Token::Comma); } } self.eat(&Token::RBracket); Some(Expr::Array(i)) }
            Token::LBrace => {
                self.adv(); let mut p = Vec::new();
                while !matches!(self.cur(), Token::RBrace) {
                    let k = match self.cur() { Token::Identifier(n) => { let v = n.clone(); self.adv(); v }, Token::String(s) => { let v = s.clone(); self.adv(); v }, Token::Number(n) => { let v = n.to_string(); self.adv(); v }, _ => break };
                    self.eat(&Token::Colon); let v = self.expr()?; p.push((k, v)); if !matches!(self.cur(), Token::RBrace) { self.eat(&Token::Comma); }
                }
                self.eat(&Token::RBrace); Some(Expr::Object(p))
            }
            Token::KwFunction => { self.adv(); let n = if let Token::Identifier(x) = self.cur() { let v = x.clone(); self.adv(); Some(v) } else { None }; self.eat(&Token::LParen); let p = self.params(); self.eat(&Token::RParen); self.eat(&Token::LBrace); let b = self.block_body(); self.eat(&Token::RBrace); Some(Expr::FnExpr { name: n, params: p, body: b }) }
            Token::KwNew => { self.adv(); let c = self.expr()?; let a = if self.eat(&Token::LParen) { let mut ag = Vec::new(); while !matches!(self.cur(), Token::RParen) { ag.push(self.expr().unwrap_or(Expr::Undefined)); if !matches!(self.cur(), Token::RParen) { self.eat(&Token::Comma); } } self.eat(&Token::RParen); ag } else { vec![] }; Some(Expr::New { callee: Box::new(c), args: a }) }
            Token::KwImport => { self.adv(); if self.eat(&Token::Dot) { if let Token::Identifier(m) = self.cur() { if m == "meta" { self.adv(); return Some(Expr::ImportMeta); } } } self.semi(); Some(Expr::Undefined) }
            Token::Identifier(n) => { let nm = n.clone(); self.adv(); Some(Expr::Identifier(nm)) }
            Token::DotDotDot => { self.adv(); let i = self.expr()?; Some(Expr::Spread(Box::new(i))) }
            _ => None,
        }
    }
    fn arrow_body(&mut self, p: &[String]) -> Expr {
        if self.eat(&Token::LBrace) { let b = self.block_body(); self.eat(&Token::RBrace); Expr::ArrowFn { params: p.to_vec(), body: Box::new(ExprOrBlock::Block(b)) } }
        else { let e = self.expr().unwrap_or(Expr::Undefined); Expr::ArrowFn { params: p.to_vec(), body: Box::new(ExprOrBlock::Expr(Box::new(e))) } }
    }
}

// ============================================================
// INTERPRETER
// ============================================================

pub type Env = Rc<RefCell<Environment>>;
#[derive(Clone)]
pub struct Environment { vars: HashMap<String, Value>, parent: Option<Env> }
impl std::fmt::Debug for Environment { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "Env({} vars)", self.vars.len()) } }
impl Environment {
    fn new() -> Self { Self { vars: HashMap::new(), parent: None } }
    fn child(p: Env) -> Self { Self { vars: HashMap::new(), parent: Some(p) } }
    fn get(&self, n: &str) -> Option<Value> { if let Some(v) = self.vars.get(n) { Some(v.clone()) } else if let Some(ref p) = self.parent { p.borrow().get(n) } else { None } }
    fn set(&mut self, n: &str, v: Value) { self.vars.insert(n.to_string(), v); }
    fn assign(&mut self, n: &str, v: Value) -> bool { if self.vars.contains_key(n) { self.vars.insert(n.to_string(), v); true } else if let Some(ref p) = self.parent { p.borrow_mut().assign(n, v) } else { false } }
}

#[derive(Clone)]
pub struct Module { pub exports: HashMap<String, Value>, pub default: Option<Value> }

pub struct Interpreter { pub global: Env, pub modules: HashMap<String, Module>, pub cur_mod: Option<String>, pub is_main: bool }
impl Interpreter {
    fn new() -> Self { Self { global: Rc::new(RefCell::new(Environment::new())), modules: HashMap::new(), cur_mod: None, is_main: false } }

    pub(crate) fn run(&mut self, stmts: &[Statement]) -> Result<Value, VmErr> {
        let mut r = Value::Undefined;
        for s in stmts { r = self.eval_stmt(s)?; } Ok(r)
    }

    fn eval_stmt(&mut self, s: &Statement) -> Result<Value, VmErr> {
        match s {
            Statement::Expr(e) => self.eval_expr(e),
            Statement::VarDecl { name, init, .. } => { let v = match init { Some(e) => self.eval_expr(e)?, None => Value::Undefined }; self.global.borrow_mut().set(name, v.clone()); Ok(v) }
            Statement::FnDecl { name, params, body } => { self.global.borrow_mut().set(name, Value::Function { name: Some(name.clone()), params: params.clone(), body: body.clone(), closure: Some(self.global.clone()) }); Ok(Value::Undefined) }
            Statement::ClassDecl { name, .. } => { self.global.borrow_mut().set(name, Value::NativeFunction { name: name.clone(), callable: |_| Ok(Value::Object(vec![])) }); Ok(Value::Undefined) }
            Statement::Return(e) => { let v = match e { Some(ex) => self.eval_expr(ex)?, None => Value::Undefined }; vm_ret(v) }
            Statement::If { test, then, else_ } => { let __test = self.eval_expr(test)?; if self.truthy(&__test) { self.run(then) } else if let Some(a) = else_ { self.run(a) } else { Ok(Value::Undefined) } },
            Statement::While { test, body } => { let mut r = Value::Undefined; loop { let __test = self.eval_expr(test)?; if !self.truthy(&__test) { break; } r = self.run(body)?; } Ok(r) },
            Statement::For { init, test, update, body } => {
                if let Some(i) = init { match i.as_ref() { ForInit::Var { name, init, .. } => { let v = match init { Some(e) => self.eval_expr(e)?, None => Value::Undefined }; self.global.borrow_mut().set(name, v); }, ForInit::Expr(e) => { self.eval_expr(e)?; } } }
                let mut r = Value::Undefined;
                loop { if let Some(t) = test { let __test = self.eval_expr(t)?; if !self.truthy(&__test) { break; } } r = self.run(body)?; if let Some(u) = update { self.eval_expr(u)?; } }
                Ok(r)
            }
            Statement::ForIn { name, obj, body } => { let o = self.eval_expr(obj)?; let ks = self.keys(&o); let mut r = Value::Undefined; for k in ks { self.global.borrow_mut().set(name, Value::String(k)); r = self.run(body)?; } Ok(r) }
            Statement::ForOf { name, iter, body } => { let a = self.eval_expr(iter)?; let items = match &a { Value::Array(i) => i.clone(), _ => return vm_err("for...of needs iterable") }; let mut r = Value::Undefined; for i in items { self.global.borrow_mut().set(name, i); r = self.run(body)?; } Ok(r) }
            Statement::Block(s) => self.run(s),
            Statement::Break => vm_err("__BREAK__"), Statement::Continue => vm_err("__CONTINUE__"),
            Statement::Throw(e) => { let v = self.eval_expr(e)?; vm_throw(self.vs(&v)) }
            Statement::Try { body, catch, finally: finally_ } => {
                match self.run(body) {
                    Err(VmErr::Throw(msg)) => { if let Some((p, cb)) = catch { let ce = Rc::new(RefCell::new(Environment::child(self.global.clone()))); ce.borrow_mut().set(p, Value::String(msg)); let s = self.global.clone(); self.global = ce; let r = self.run(cb); self.global = s; r } else { Ok(Value::Undefined) } }
                    Err(VmErr::Ret(v)) => Err(VmErr::Ret(v)),
                    other => { if let Some(f) = finally_ { self.run(f)?; } other }
                }
            }
            Statement::Switch { disc, cases } => { let d = self.eval_expr(disc)?; let mut r = Value::Undefined; let mut m = false; for c in cases { if let Some(ref t) = c.test { let __t = self.eval_expr(t)?; if self.seq(&d, &__t) { m = true; } } else { m = true; } if m { match self.run(&c.body) { Err(vm_err) => { let s = format!("{}", vm_err); if s == "__BREAK__" { break; } else { return Err(vm_err); } }, Ok(v) => { r = v; } } } } Ok(r) }
            Statement::ExportDefault(e) => { let v = self.eval_expr(e)?; let mn = self.cur_mod.clone().unwrap_or_default(); let mo = self.modules.entry(mn).or_insert_with(|| Module { exports: HashMap::new(), default: None }); mo.default = Some(v); Ok(Value::Undefined) }
            Statement::ExportNamed { specifiers, source: _ } => { let mn = self.cur_mod.clone().unwrap_or_default(); let mo = self.modules.entry(mn).or_insert_with(|| Module { exports: HashMap::new(), default: None }); for (l, e) in specifiers { if let Some(v) = self.global.borrow().get(l) { mo.exports.insert(e.clone(), v); } } Ok(Value::Undefined) }
            Statement::Import { module, default, named, namespace } => {
                if let Some(md) = self.modules.get(module) {
                    if let Some(d) = default { let v = md.default.clone().unwrap_or(Value::Undefined); self.global.borrow_mut().set(d, v); }
                    for (l, i) in named { let v = md.exports.get(i).cloned().unwrap_or(Value::Undefined); self.global.borrow_mut().set(l, v); }
                    if let Some(ns) = namespace { let mut p: Vec<(String, Value)> = md.exports.iter().map(|(k, v)| (k.clone(), v.clone())).collect(); if let Some(ref def) = md.default { p.push(("_default".to_string(), def.clone())); } self.global.borrow_mut().set(ns, Value::Object(p)); }
                    Ok(Value::Undefined)
                } else { vm_err(format!("Module not found: {}", module)) }
            }
            Statement::Empty => Ok(Value::Undefined),
        }
    }

    fn eval_expr(&mut self, e: &Expr) -> Result<Value, VmErr> {
        match e {
            Expr::Number(n) => Ok(Value::Number(*n)), Expr::String(s) => Ok(Value::String(s.clone())), Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Null => Ok(Value::Null), Expr::Undefined => Ok(Value::Undefined),
            Expr::Identifier(n) => self.global.borrow().get(n).ok_or_else(|| VmErr::Msg(format!("Undefined: {}", n))),
            Expr::Array(i) => { let mut v = Vec::new(); for x in i { v.push(self.eval_expr(x)?); } Ok(Value::Array(v)) }
            Expr::Object(p) => { let mut o = Vec::new(); for (k, v) in p { o.push((k.clone(), self.eval_expr(v)?)); } Ok(Value::Object(o)) }
            Expr::Binary { op, left, right } => { let l = self.eval_expr(left)?; let r = self.eval_expr(right)?; self.bin_op(op, &l, &r) }
            Expr::Unary { op, operand, prefix } => {
                if (op == "++" || op == "--") && matches!(operand.as_ref(), Expr::Identifier(_)) {
                    if let Expr::Identifier(n) = operand.as_ref() {
                        let (cur, new_val) = {
                            let env = self.global.borrow();
                            let cur = env.get(n).ok_or_else(|| VmErr::Msg(format!("Undefined: {}", n)))?;
                            let nv = if op == "++" { Value::Number(self.tn(&cur) + 1.0) } else { Value::Number(self.tn(&cur) - 1.0) };
                            (cur, nv)
                        };
                        self.global.borrow_mut().assign(n, new_val.clone());
                        if *prefix { Ok(new_val) } else { Ok(cur) }
                    } else { unreachable!() }
                } else {
                    let v = self.eval_expr(operand)?;
                    self.un_op(op, &v)
                }
            }
            Expr::Call { callee, args } => { let mut a = Vec::new(); for x in args { a.push(self.eval_expr(x)?); } let c = self.eval_expr(callee)?; self.call(&c, a) }
            Expr::Member { object, property, computed: _ } => { let o = self.eval_expr(object)?; let p = self.eval_expr(property)?; self.prop(&o, &p) }
            Expr::Assignment { target, op, value } => { let v = self.eval_expr(value)?; match target.as_ref() { Expr::Identifier(n) => { let fv = if *op != "=" { let c = self.global.borrow().get(n).ok_or_else(|| VmErr::Msg(format!("Undefined: {}", n)))?; let bin_op = op.trim_end_matches('='); self.bin_op(bin_op, &c, &v)? } else { v }; if !self.global.borrow_mut().assign(n, fv.clone()) { self.global.borrow_mut().set(n, fv.clone()); } Ok(fv) }, _ => vm_err("Invalid assignment target") } }
            Expr::Conditional { test, consequent, alternate } => { let __test = self.eval_expr(test)?; if self.truthy(&__test) { self.eval_expr(consequent) } else { self.eval_expr(alternate) } },
            Expr::ArrowFn { params, body } => Ok(Value::Function { name: None, params: params.clone(), closure: Some(self.global.clone()), body: match body.as_ref() { ExprOrBlock::Block(s) => s.clone(), ExprOrBlock::Expr(e) => vec![Statement::Return(Some(e.clone()))] } }),
            Expr::FnExpr { name, params, body } => Ok(Value::Function { name: name.clone(), params: params.clone(), body: body.clone(), closure: Some(self.global.clone()) }),
            Expr::New { callee, args } => { let mut a = Vec::new(); for x in args { a.push(self.eval_expr(x)?); } let c = self.eval_expr(callee)?; self.ctor(&c, a) }
            Expr::Spread(i) => self.eval_expr(i), Expr::This => Ok(self.global.borrow().get("this").unwrap_or(Value::Undefined)),
            Expr::ImportMeta => { let mut o = vec![]; o.push(("url".to_string(), Value::String("vm://module".to_string()))); o.push(("main".to_string(), Value::Bool(self.is_main))); Ok(Value::Object(o)) }
        }
    }

    fn bin_op(&self, op: &str, l: &Value, r: &Value) -> Result<Value, VmErr> {
        Ok(match op {
            "+" => match (l, r) { (Value::Number(a), Value::Number(b)) => Value::Number(a + b), (Value::String(a), _) => Value::String(format!("{}{}", a, self.vs(r))), (_, Value::String(b)) => Value::String(format!("{}{}", self.vs(l), b)), _ => Value::String(format!("{}{}", self.vs(l), self.vs(r))) },
            "-" => Value::Number(self.tn(l) - self.tn(r)), "*" => Value::Number(self.tn(l) * self.tn(r)), "/" => Value::Number(self.tn(l) / self.tn(r)), "%" => Value::Number(self.tn(l) % self.tn(r)),
            "==" => Value::Bool(self.leq(l, r)), "!=" => Value::Bool(!self.leq(l, r)), "===" => Value::Bool(self.seq(l, r)), "!==" => Value::Bool(!self.seq(l, r)),
            "<" => Value::Bool(self.tn(l) < self.tn(r)), ">" => Value::Bool(self.tn(l) > self.tn(r)), "<=" => Value::Bool(self.tn(l) <= self.tn(r)), ">=" => Value::Bool(self.tn(l) >= self.tn(r)),
            "&&" => if self.truthy(l) { r.clone() } else { l.clone() }, "||" => if self.truthy(l) { l.clone() } else { r.clone() },
            "instanceof" => Value::Bool(false),
            "in" => if let (Value::String(k), Value::Object(p)) = (l, r) { Value::Bool(p.iter().any(|(x, _)| x == k)) } else { Value::Bool(false) },
            _ => return vm_err(format!("Unknown op: {}", op)),
        })
    }
    fn un_op(&self, op: &str, v: &Value) -> Result<Value, VmErr> {
        Ok(match op { "!" => Value::Bool(!self.truthy(v)), "-" => Value::Number(-self.tn(v)), "+" => Value::Number(self.tn(v)), "typeof" => Value::String(match v { Value::Undefined => "undefined", Value::Null => "object", Value::Bool(_) => "boolean", Value::Number(_) => "number", Value::String(_) => "string", Value::Object(_) | Value::Array(_) => "object", Value::Function { .. } | Value::NativeFunction { .. } => "function" }.to_string()), "void" => Value::Undefined, "delete" => Value::Bool(true), "++" => Value::Number(self.tn(v) + 1.0), "--" => Value::Number(self.tn(v) - 1.0), _ => return vm_err(format!("Unknown unary: {}", op)) })
    }
    fn call(&mut self, f: &Value, args: Vec<Value>) -> Result<Value, VmErr> {
        match f {
            Value::Function { params, body, closure, .. } => {
                let parent_env = closure.clone().unwrap_or_else(|| self.global.clone());
                let fe = Rc::new(RefCell::new(Environment::child(parent_env)));
                for (i, p) in params.iter().enumerate() { fe.borrow_mut().set(p, args.get(i).cloned().unwrap_or(Value::Undefined)); }
                let s = self.global.clone(); self.global = fe; let r = self.run(body); self.global = s;
                match r { Err(VmErr::Ret(v)) => Ok(v), other => other }
            }
            Value::NativeFunction { callable, .. } => callable(args),
            _ => vm_err("Not a function"),
        }
    }
    fn ctor(&mut self, f: &Value, args: Vec<Value>) -> Result<Value, VmErr> {
        let inst = Value::Object(vec![]);
        if let Value::Function { params, body, closure, .. } = f {
            let parent_env = closure.clone().unwrap_or_else(|| self.global.clone());
            let fe = Rc::new(RefCell::new(Environment::child(parent_env)));
            fe.borrow_mut().set("this", inst.clone());
            for (i, p) in params.iter().enumerate() { fe.borrow_mut().set(p, args.get(i).cloned().unwrap_or(Value::Undefined)); }
            let s = self.global.clone(); self.global = fe; let r = self.run(body); self.global = s;
            match r { Err(VmErr::Ret(v)) => match v { Value::Object(_) => Ok(v), _ => Ok(inst) }, _ => Ok(inst) }
        } else { vm_err("Not a constructor") }
    }
    fn prop(&self, o: &Value, p: &Value) -> Result<Value, VmErr> {
        match (o, p) {
            (Value::Object(props), Value::String(k)) => { for (xk, xv) in props { if xk == k { return Ok(xv.clone()); } } Ok(Value::Undefined) }
            (Value::Array(items), Value::Number(i)) => { let idx = *i as usize; if idx < items.len() { Ok(items[idx].clone()) } else { Ok(Value::Undefined) } }
            (Value::Array(items), Value::String(k)) => if k == "length" { Ok(Value::Number(items.len() as f64)) } else { Ok(Value::Undefined) },
            (Value::String(s), Value::String(k)) => if k == "length" { Ok(Value::Number(s.len() as f64)) } else { Ok(Value::Undefined) },
            _ => Ok(Value::Undefined),
        }
    }
    fn keys(&self, o: &Value) -> Vec<String> { match o { Value::Object(p) => p.iter().map(|(k, _)| k.clone()).collect(), Value::Array(i) => (0..i.len()).map(|x| x.to_string()).collect(), _ => vec![] } }
    fn truthy(&self, v: &Value) -> bool { match v { Value::Bool(b) => *b, Value::Number(n) => *n != 0.0 && !n.is_nan(), Value::String(s) => !s.is_empty(), Value::Null | Value::Undefined => false, _ => true } }
    fn tn(&self, v: &Value) -> f64 { match v { Value::Number(n) => *n, Value::Bool(b) => if *b { 1.0 } else { 0.0 }, Value::String(s) => s.parse().unwrap_or(0.0), Value::Null => 0.0, Value::Undefined => f64::NAN, _ => 0.0 } }
    fn leq(&self, a: &Value, b: &Value) -> bool { match (a, b) { (Value::Number(a), Value::Number(b)) => a == b, (Value::String(a), Value::String(b)) => a == b, (Value::Bool(a), Value::Bool(b)) => a == b, (Value::Null, Value::Undefined) | (Value::Undefined, Value::Null) => true, _ => false } }
    fn seq(&self, a: &Value, b: &Value) -> bool { match (a, b) { (Value::Number(a), Value::Number(b)) => a == b, (Value::String(a), Value::String(b)) => a == b, (Value::Bool(a), Value::Bool(b)) => a == b, (Value::Null, Value::Null) | (Value::Undefined, Value::Undefined) => true, _ => false } }
    fn vs(&self, v: &Value) -> String { match v { Value::Undefined => "undefined".to_string(), Value::Null => "null".to_string(), Value::Bool(b) => b.to_string(), Value::Number(n) => if n.fract() == 0.0 && n.abs() < 1e15 { format!("{:.0}", n) } else { n.to_string() }, Value::String(s) => s.clone(), Value::Object(_) => "[object Object]".to_string(), Value::Array(i) => i.iter().map(|x| self.vs(x)).collect::<Vec<_>>().join(","), Value::Function { name, .. } => format!("function {}", name.as_deref().unwrap_or("")), Value::NativeFunction { name, .. } => format!("function {} [native]", name) } }
    fn sv(&self, s: &str) -> Value { if s == "undefined" { Value::Undefined } else if s == "null" { Value::Null } else if s == "true" { Value::Bool(true) } else if s == "false" { Value::Bool(false) } else if let Ok(n) = s.parse::<f64>() { Value::Number(n) } else { Value::String(s.to_string()) } }
}

// ============================================================
// BUILTINS
// ============================================================

fn setup_builtins(env: &Env) {
    let mut e = env.borrow_mut();

    let simple: &[&str] = &[
        "Boolean", "Error", "TypeError", "RangeError", "SyntaxError", "ReferenceError",
        "Map", "Set", "WeakMap", "WeakSet", "DataView", "RegExp", "Function",
        "globalThis", "self", "window", "fetch", "URLSearchParams", "Headers", "Request",
        "Event", "EventTarget", "CustomEvent", "AbortController", "AbortSignal",
        "TextEncoder", "TextDecoder", "ReadableStream", "WritableStream", "TransformStream",
        "Blob", "File", "FormData", "queueMicrotask", "setTimeout", "setInterval",
        "clearTimeout", "clearInterval", "structuredClone", "Proxy", "undefined", "isNaN", "isFinite",
        "parseInt", "parseFloat", "encodeURI", "decodeURI", "encodeURIComponent", "decodeURIComponent",
        "escape", "unescape", "eval", "require", "exports", "__dirname", "__filename",
        "Worker", "SharedWorker", "MessageChannel", "MessagePort", "BroadcastChannel",
        "EventSource", "ByteLengthQueuingStrategy", "CountQueuingStrategy",
        "CompressionStream", "DecompressionStream", "DOMException",
        "Lock", "LockManager", "Navigation", "Navigator",
        "Notification", "PermissionStatus", "Permissions",
        "PushManager", "PushSubscription", "PushSubscriptionOptions",
        "Scheduler", "StorageManager", "Worklet",
        "CryptoKey", "GPU", "GPUAdapter", "GPUBindGroup",
        "GPUBuffer", "GPUCanvasContext", "GPUCommandBuffer",
        "GPUCommandEncoder", "GPUComputePassEncoder", "GPUComputePipeline",
        "GPUDevice", "GPUExternalTexture", "GPUPipelineLayout",
        "GPUQuerySet", "GPUQueue", "GPURenderBundle",
        "GPURenderBundleEncoder", "GPURenderPassEncoder", "GPURenderPipeline",
        "GPUSampler", "GPUShaderModule", "GPUTexture", "GPUTextureView",
        "WGSLLanguageFeatures", "importScripts", "close", "postMessage",
        "parentPort", "threadId", "workerData", "isMainThread",
        "WritableStreamDefaultWriter", "WritableStreamDefaultController",
        "ReadableStreamDefaultReader", "ReadableStreamBYOBReader",
        "ReadableStreamDefaultController", "ReadableByteStreamController",
        "TransformStreamDefaultController", "AudioData", "EncodedAudioChunk",
        "EncodedVideoChunk", "ImageBitmap", "OffscreenCanvas", "VideoFrame",
        "WebSocketStream", "Serial", "USB", "HID", "Bluetooth",
        "Clipboard", "Credential", "CredentialsContainer",
        "Geolocation", "GeolocationPosition", "GeolocationCoordinates",
        "GeolocationPositionError", "ServiceWorker", "ServiceWorkerContainer",
        "ServiceWorkerRegistration", "ServiceWorkerGlobalScope",
        "DedicatedWorkerGlobalScope", "SharedWorkerGlobalScope", "WorkerGlobalScope",
        "UnloadEvent",
    ];
    for name in simple {
        e.set(name, Value::Object(vec![]));
    }

    let with_members: &[(&str, &[&str])] = &[
        ("console", &["log", "error", "warn", "info", "debug"]),
        ("Object", &["keys", "values", "entries", "assign"]),
        ("Array", &["isArray", "from", "of"]), ("String", &["fromCharCode"]),
        ("Number", &["isNaN", "isFinite", "parseInt", "parseFloat"]),
        ("Symbol", &["iterator"]),
        ("Promise", &["resolve", "reject", "all", "race"]),
        ("ArrayBuffer", &["isView"]),
        ("Date", &["now", "parse", "UTC"]),
        ("URL", &["createObjectURL", "revokeObjectURL"]),
        ("Response", &["json", "text", "redirect"]),
        ("WebSocket", &["CONNECTING", "OPEN", "CLOSING", "CLOSED"]),
        ("crypto", &["getRandomValues", "randomUUID", "subtle"]),
        ("navigator", &["userAgent", "language", "platform"]),
        ("performance", &["now"]),
        ("BigInt", &["asIntN", "asUintN"]),
        ("Reflect", &["apply", "construct", "defineProperty", "deleteProperty", "get", "has", "set"]),
        ("Intl", &["DateTimeFormat", "NumberFormat"]),
        ("module", &["exports"]),
        ("process", &["env", "argv", "cwd", "pid", "platform", "version"]),
        ("Buffer", &["alloc", "from", "concat", "isBuffer"]),
        ("location", &["href", "protocol", "host", "pathname", "search", "hash", "origin"]),
        ("history", &["length", "go", "back", "forward", "pushState", "replaceState"]),
        ("screen", &["width", "height"]),
        ("localStorage", &["getItem", "setItem", "removeItem", "clear"]),
        ("sessionStorage", &["getItem", "setItem", "removeItem", "clear"]),
        ("indexedDB", &["open", "deleteDatabase"]),
        ("caches", &["open", "has", "delete", "keys", "match"]),
        ("Cache", &["match", "add", "put", "delete", "keys"]),
        ("CacheStorage", &["open", "has", "delete", "keys"]),
        ("SubtleCrypto", &["encrypt", "decrypt", "sign", "verify", "digest", "generateKey", "deriveKey", "deriveBits", "importKey", "exportKey", "wrapKey", "unwrapKey"]),
        ("MessageEvent", &["data", "origin", "lastEventId", "source", "ports"]),
        ("ErrorEvent", &["message", "filename", "lineno", "colno", "error"]),
        ("PromiseRejectionEvent", &["promise", "reason"]),
        ("CloseEvent", &["code", "reason", "wasClean"]),
        ("HashChangeEvent", &["oldURL", "newURL"]), ("PopStateEvent", &["state"]),
        ("StorageEvent", &["key", "oldValue", "newValue", "url", "storageArea"]),
        ("SubmitEvent", &["submitter"]), ("FormDataEvent", &["formData"]),
        ("ProgressEvent", &["lengthComputable", "loaded", "total"]),
        ("PageTransitionEvent", &["persisted"]), ("BeforeUnloadEvent", &["returnValue"]),
        ("UIEvent", &["detail", "view", "which"]),
        ("MouseEvent", &["screenX", "screenY", "clientX", "clientY", "ctrlKey", "shiftKey", "altKey", "metaKey", "button", "buttons", "relatedTarget"]),
        ("KeyboardEvent", &["key", "code", "location", "ctrlKey", "shiftKey", "altKey", "metaKey", "repeat", "isComposing"]),
        ("TouchEvent", &["touches", "targetTouches", "changedTouches"]),
        ("Touch", &["identifier", "target", "screenX", "screenY", "clientX", "clientY", "pageX", "pageY"]),
        ("WheelEvent", &["deltaX", "deltaY", "deltaZ", "deltaMode"]),
        ("DragEvent", &["dataTransfer"]), ("FocusEvent", &["relatedTarget"]),
        ("InputEvent", &["data", "inputType", "isComposing"]), ("CompositionEvent", &["data"]),
        ("PointerEvent", &["pointerId", "width", "height", "pressure", "pointerType", "isPrimary"]),
        ("AnimationEvent", &["animationName", "elapsedTime", "pseudoElement"]),
        ("TransitionEvent", &["propertyName", "elapsedTime", "pseudoElement"]),
        ("ClipboardEvent", &["clipboardData"]),
        ("SecurityPolicyViolationEvent", &["documentURI", "referrer", "blockedURI", "violatedDirective"]),
        ("JSON", &["parse", "stringify"]),
    ];
    for (name, members) in with_members {
        let props: Vec<(String, Value)> = members.iter().map(|m| (m.to_string(), Value::Undefined)).collect();
        e.set(name, Value::Object(props));
    }

    e.set("Math", Value::Object(vec![
        ("PI".to_string(), Value::Number(3.141592653589793)),
        ("E".to_string(), Value::Number(2.718281828459045)),
        ("LN2".to_string(), Value::Number(0.6931471805599453)),
        ("LN10".to_string(), Value::Number(2.302585092994046)),
        ("LOG2E".to_string(), Value::Number(1.4426950408889634)),
        ("LOG10E".to_string(), Value::Number(0.4342944819032518)),
        ("SQRT1_2".to_string(), Value::Number(0.7071067811865476)),
        ("SQRT2".to_string(), Value::Number(1.4142135623730951)),
        ("abs".to_string(), Value::Undefined),
        ("floor".to_string(), Value::Undefined),
        ("ceil".to_string(), Value::Undefined),
        ("round".to_string(), Value::Undefined),
        ("sqrt".to_string(), Value::Undefined),
        ("pow".to_string(), Value::Undefined),
        ("min".to_string(), Value::Undefined),
        ("max".to_string(), Value::Undefined),
        ("random".to_string(), Value::Undefined),
    ]));

    e.set("Infinity", Value::Number(f64::INFINITY));
    e.set("NaN", Value::Number(f64::NAN));
}

// ============================================================
// NAPI
// ============================================================

fn to_string(val: &Value) -> String {
    fn vs(v: &Value) -> String {
        match v {
            Value::Undefined => "undefined".to_string(), Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => if n.fract() == 0.0 && n.abs() < 1e15 { format!("{:.0}", n) } else { n.to_string() },
            Value::String(s) => s.clone(),
            Value::Object(p) => format!("{{{}}}", p.iter().map(|(k, v)| format!("{}: {}", k, vs(v))).collect::<Vec<_>>().join(", ")),
            Value::Array(i) => format!("[{}]", i.iter().map(|v| vs(v)).collect::<Vec<_>>().join(", ")),
            Value::Function { name, .. } => format!("[Function: {}]", name.as_deref().unwrap_or("anonymous")),
            Value::NativeFunction { name, .. } => format!("[Function: {} [native]]", name),
        }
    }
    vs(val)
}

fn run_source(source: &str, is_main: bool) -> Result<String, VmErr> {
    let mut interp = Interpreter::new();
    interp.is_main = is_main;
    setup_builtins(&interp.global);
    let mut lex = Lexer::new(source);
    let toks = lex.tokenize();
    let mut parser = Parser::new(toks);
    let stmts = parser.parse();
    let val = interp.run(&stmts)?;
    Ok(to_string(&val))
}

#[napi]
pub struct VM {
    interp: Interpreter,
    modules: HashMap<String, String>,
}

#[napi]
impl VM {
    #[napi(constructor)]
    pub fn new() -> Self {
        let mut interp = Interpreter::new();
        setup_builtins(&interp.global);
        Self { interp, modules: HashMap::new() }
    }

    #[napi]
    pub fn run(&mut self, source: String) -> napi::Result<String> {
        let mut lex = Lexer::new(&source);
        let toks = lex.tokenize();
        let mut parser = Parser::new(toks);
        let stmts = parser.parse();
        Ok(to_string(&self.interp.run(&stmts).map_err(|e| napi::Error::from_reason(e.to_string()))?))
    }

    #[napi]
    pub fn register_module(&mut self, name: String, source: String) -> napi::Result<()> {
        let mut interp = Interpreter::new();
        setup_builtins(&interp.global);
        interp.cur_mod = Some(name.clone());
        interp.is_main = false;
        let mut lex = Lexer::new(&source);
        let toks = lex.tokenize();
        let mut parser = Parser::new(toks);
        let stmts = parser.parse();
        interp.run(&stmts).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.modules.insert(name.clone(), source);
        self.interp.modules.insert(name, Module { exports: HashMap::new(), default: None });
        Ok(())
    }

    #[napi]
    pub fn set_import_meta_main(&mut self, is_main: bool) {
        self.interp.is_main = is_main;
    }

    #[napi]
    pub fn get_global(&self, name: String) -> napi::Result<String> {
        match self.interp.global.borrow().get(&name) {
            Some(val) => Ok(to_string(&val)),
            None => Ok("undefined".to_string()),
        }
    }
}

#[napi]
pub fn create_vm() -> VM { VM::new() }

#[napi]
pub fn run_code(source: String) -> napi::Result<String> {
    run_source(&source, false).map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn debug_parse(source: String) -> napi::Result<String> {
    let mut lex = Lexer::new(&source);
    let toks = lex.tokenize();
    let mut parser = Parser::new(toks);
    let stmts = parser.parse();
    Ok(format!("{:?}", stmts))
}
