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
    Binary {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: String,
        operand: Box<Expr>,
        prefix: bool,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Member {
        object: Box<Expr>,
        property: Box<Expr>,
        computed: bool,
    },
    Assignment {
        target: Box<Expr>,
        op: String,
        value: Box<Expr>,
    },
    Conditional {
        test: Box<Expr>,
        consequent: Box<Expr>,
        alternate: Box<Expr>,
    },
    ArrowFn {
        params: Vec<String>,
        body: Box<ExprOrBlock>,
    },
    FnExpr {
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    New {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
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
    VarDecl {
        kind: VarKind,
        name: String,
        init: Option<Box<Expr>>,
    },
    FnDecl {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    ClassDecl {
        name: String,
        superclass: Option<Box<Expr>>,
        body: Vec<ClassMember>,
    },
    Return(Option<Box<Expr>>),
    If {
        test: Box<Expr>,
        then: Vec<Statement>,
        else_: Option<Vec<Statement>>,
    },
    While {
        test: Box<Expr>,
        body: Vec<Statement>,
    },
    DoWhile {
        test: Box<Expr>,
        body: Vec<Statement>,
    },
    For {
        init: Option<Box<ForInit>>,
        test: Option<Box<Expr>>,
        update: Option<Box<Expr>>,
        body: Vec<Statement>,
    },
    ForIn {
        name: String,
        obj: Box<Expr>,
        body: Vec<Statement>,
    },
    ForOf {
        name: String,
        iter: Box<Expr>,
        body: Vec<Statement>,
    },
    Block(Vec<Statement>),
    Break,
    Continue,
    Throw(Box<Expr>),
    Try {
        body: Vec<Statement>,
        catch: Option<(String, Vec<Statement>)>,
        finally: Option<Vec<Statement>>,
    },
    Switch {
        disc: Box<Expr>,
        cases: Vec<SwitchCase>,
    },
    ExportDefault(Box<Expr>),
    ExportNamed {
        specifiers: Vec<(String, String)>,
        source: Option<String>,
    },
    Import {
        module: String,
        default: Option<String>,
        named: Vec<(String, String)>,
        namespace: Option<String>,
    },
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
    Var {
        kind: VarKind,
        name: String,
        init: Option<Box<Expr>>,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub test: Option<Expr>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Method {
        name: String,
        is_static: bool,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    Field {
        name: String,
        is_static: bool,
        init: Option<Expr>,
    },
}
