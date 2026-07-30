#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Undefined,
    Identifier(String),
    Array(Vec<Expr>),
    Object(Vec<ObjectProp>),
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
        is_async: bool,
        is_generator: bool,
    },
    New {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Spread(Box<Expr>),
    This,
    Super,
    ImportMeta,
    Template {
        quasis: Vec<String>,
        exprs: Vec<Expr>,
    },
    OptionalChain {
        object: Box<Expr>,
        property: Box<Expr>,
        computed: bool,
    },
    Await(Box<Expr>),
    Yield(Option<Box<Expr>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectProp {
    Shorthand(String),
    KeyValue(String, Expr),
    Computed(Expr, Expr),
    Method {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    Getter {
        name: String,
        body: Vec<Statement>,
    },
    Setter {
        name: String,
        param: String,
        body: Vec<Statement>,
    },
    Spread(Expr),
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
        destructuring: Option<Box<Pattern>>,
    },
    FnDecl {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        is_async: bool,
        is_generator: bool,
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
    Labeled {
        label: String,
        body: Box<Statement>,
    },
    Break,
    Continue,
    LabeledBreak(String),
    LabeledContinue(String),
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
        decls: Vec<(String, Option<Expr>)>,
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
    Getter {
        name: String,
        is_static: bool,
        body: Vec<Statement>,
    },
    Setter {
        name: String,
        param: String,
        is_static: bool,
        body: Vec<Statement>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Ident(String),
    Array(Vec<Pattern>),
    Object(Vec<(String, Option<Pattern>)>),
    Rest(Box<Pattern>),
    Default(Box<Pattern>, Box<Expr>),
}

impl Pattern {
    pub fn is_rest(&self) -> bool {
        matches!(self, Pattern::Rest(_))
    }
}

/// Returns true if any expression within `stmts` references the identifier
/// `name`. Used to decide whether a function frame needs an `arguments` object.
///
/// Deliberately over-approximates: nested (non-arrow) function bodies are
/// included in the scan even though their `name` references bind to their own
/// frame. That only causes an unneeded object to be built — never a missing
/// one — so callers remain correct.
pub fn stmts_reference(stmts: &[Statement], name: &str) -> bool {
    stmts.iter().any(|s| stmt_references(s, name))
}

/// `stmts_reference` for an arrow-function body (expression or block).
pub fn arrow_body_references(body: &ExprOrBlock, name: &str) -> bool {
    match body {
        ExprOrBlock::Expr(e) => expr_references(e, name),
        ExprOrBlock::Block(s) => stmts_reference(s, name),
    }
}

fn stmt_references(s: &Statement, name: &str) -> bool {
    match s {
        Statement::Expr(e) => expr_references(e, name),
        Statement::VarDecl {
            init, destructuring, ..
        } => {
            init.as_ref().map(|e| expr_references(e, name)).unwrap_or(false)
                || destructuring
                    .as_ref()
                    .map(|p| pattern_references(p, name))
                    .unwrap_or(false)
        }
        Statement::FnDecl { body, .. } => stmts_reference(body, name),
        Statement::ClassDecl { superclass, body, .. } => {
            superclass.as_ref().map(|e| expr_references(e, name)).unwrap_or(false)
                || body.iter().any(|m| match m {
                    ClassMember::Method { body, .. } => stmts_reference(body, name),
                    ClassMember::Field { init, .. } => {
                        init.as_ref().map(|e| expr_references(e, name)).unwrap_or(false)
                    }
                    ClassMember::Getter { body, .. } => stmts_reference(body, name),
                    ClassMember::Setter { body, .. } => stmts_reference(body, name),
                })
        }
        Statement::Return(e) => e.as_ref().map(|e| expr_references(e, name)).unwrap_or(false),
        Statement::If { test, then, else_ } => {
            expr_references(test, name)
                || stmts_reference(then, name)
                || else_.as_ref().map(|b| stmts_reference(b, name)).unwrap_or(false)
        }
        Statement::While { test, body } | Statement::DoWhile { test, body } => {
            expr_references(test, name) || stmts_reference(body, name)
        }
        Statement::For {
            init,
            test,
            update,
            body,
        } => {
            init.as_ref()
                .map(|i| match i.as_ref() {
                    ForInit::Var { decls, .. } => decls.iter().any(|(_, e)| {
                        e.as_ref().map(|e| expr_references(e, name)).unwrap_or(false)
                    }),
                    ForInit::Expr(e) => expr_references(e, name),
                })
                .unwrap_or(false)
                || test.as_ref().map(|e| expr_references(e, name)).unwrap_or(false)
                || update.as_ref().map(|e| expr_references(e, name)).unwrap_or(false)
                || stmts_reference(body, name)
        }
        Statement::ForIn { obj, body, .. } => {
            expr_references(obj, name) || stmts_reference(body, name)
        }
        Statement::ForOf { iter, body, .. } => {
            expr_references(iter, name) || stmts_reference(body, name)
        }
        Statement::Block(b) => stmts_reference(b, name),
        Statement::Labeled { body, .. } => stmt_references(body, name),
        Statement::Throw(e) => expr_references(e, name),
        Statement::Try {
            body,
            catch,
            finally,
        } => {
            stmts_reference(body, name)
                || catch.as_ref().map(|(_, b)| stmts_reference(b, name)).unwrap_or(false)
                || finally.as_ref().map(|b| stmts_reference(b, name)).unwrap_or(false)
        }
        Statement::Switch { disc, cases } => {
            expr_references(disc, name)
                || cases.iter().any(|c| {
                    c.test.as_ref().map(|e| expr_references(e, name)).unwrap_or(false)
                        || stmts_reference(&c.body, name)
                })
        }
        Statement::ExportDefault(e) => expr_references(e, name),
        Statement::Break
        | Statement::Continue
        | Statement::LabeledBreak(_)
        | Statement::LabeledContinue(_)
        | Statement::ExportNamed { .. }
        | Statement::Import { .. }
        | Statement::Empty => false,
    }
}

fn expr_references(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Identifier(n) => n == name,
        Expr::Array(items) => items.iter().any(|x| expr_references(x, name)),
        Expr::Object(props) => props.iter().any(|p| match p {
            ObjectProp::Shorthand(n) => n == name,
            ObjectProp::KeyValue(_, v) => expr_references(v, name),
            ObjectProp::Computed(k, v) => expr_references(k, name) || expr_references(v, name),
            ObjectProp::Method { body, .. } => stmts_reference(body, name),
            ObjectProp::Getter { body, .. } => stmts_reference(body, name),
            ObjectProp::Setter { body, .. } => stmts_reference(body, name),
            ObjectProp::Spread(x) => expr_references(x, name),
        }),
        Expr::Binary { left, right, .. } => {
            expr_references(left, name) || expr_references(right, name)
        }
        Expr::Unary { operand, .. } => expr_references(operand, name),
        Expr::Call { callee, args } => {
            expr_references(callee, name) || args.iter().any(|a| expr_references(a, name))
        }
        Expr::Member {
            object, property, ..
        } => expr_references(object, name) || expr_references(property, name),
        Expr::OptionalChain {
            object, property, ..
        } => expr_references(object, name) || expr_references(property, name),
        Expr::Assignment { target, value, .. } => {
            expr_references(target, name) || expr_references(value, name)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
        } => {
            expr_references(test, name)
                || expr_references(consequent, name)
                || expr_references(alternate, name)
        }
        Expr::ArrowFn { body, .. } => match body.as_ref() {
            ExprOrBlock::Expr(x) => expr_references(x, name),
            ExprOrBlock::Block(s) => stmts_reference(s, name),
        },
        Expr::FnExpr { body, .. } => stmts_reference(body, name),
        Expr::New { callee, args } => {
            expr_references(callee, name) || args.iter().any(|a| expr_references(a, name))
        }
        Expr::Spread(x) => expr_references(x, name),
        Expr::Template { exprs, .. } => exprs.iter().any(|x| expr_references(x, name)),
        Expr::Await(x) => expr_references(x, name),
        Expr::Yield(x) => x.as_ref().map(|x| expr_references(x, name)).unwrap_or(false),
        Expr::Number(_)
        | Expr::String(_)
        | Expr::Bool(_)
        | Expr::Null
        | Expr::Undefined
        | Expr::This
        | Expr::Super
        | Expr::ImportMeta => false,
    }
}

fn pattern_references(p: &Pattern, name: &str) -> bool {
    match p {
        Pattern::Ident(_) | Pattern::Rest(_) => false,
        Pattern::Array(elems) => elems.iter().any(|e| pattern_references(e, name)),
        Pattern::Object(props) => props
            .iter()
            .any(|(_, p)| p.as_ref().map(|p| pattern_references(p, name)).unwrap_or(false)),
        Pattern::Default(inner, default) => {
            pattern_references(inner, name) || expr_references(default, name)
        }
    }
}
