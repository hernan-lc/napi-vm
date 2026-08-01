//! A small compiler-style document model shared by all editor frontends.
//!
//! The model is intentionally conservative: unknown values stay `unknown`,
//! while literal objects, functions, promises, arrays, and imports retain the
//! information needed by hover and completion. It never executes guest code.

use std::collections::{BTreeMap, HashMap};

use crate::lexer::Token;
use crate::parser::{BinOp, Expr, ExprOrBlock, ObjectProp, Statement, VarKind};
use crate::span::SpannedToken;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Any,
    Unknown,
    Number,
    String,
    Boolean,
    Null,
    Undefined,
    Object(BTreeMap<String, Type>),
    Array(Box<Type>),
    Promise(Box<Type>),
    Function {
        params: Vec<String>,
        result: Box<Type>,
        async_fn: bool,
    },
}

impl Type {
    fn property(&self, name: &str) -> Type {
        match self {
            Type::Object(fields) => fields.get(name).cloned().unwrap_or(Type::Unknown),
            Type::Array(_) if name == "length" => Type::Number,
            Type::Promise(value) if matches!(name, "then" | "catch" | "finally") => {
                Type::Function {
                    params: vec![],
                    result: Box::new(Type::Promise(value.clone())),
                    async_fn: false,
                }
            }
            Type::String if name == "length" => Type::Number,
            Type::Function { .. } if name == "name" => Type::String,
            _ => Type::Unknown,
        }
    }

    fn unwrap_promise(&self) -> Type {
        match self {
            Type::Promise(value) => value.as_ref().clone(),
            other => other.clone(),
        }
    }

    fn display(&self) -> String {
        match self {
            Type::Any => "any".into(),
            Type::Unknown => "unknown".into(),
            Type::Number => "number".into(),
            Type::String => "string".into(),
            Type::Boolean => "boolean".into(),
            Type::Null => "null".into(),
            Type::Undefined => "undefined".into(),
            Type::Array(item) => format!("{}[]", item.display()),
            Type::Promise(value) => format!("Promise<{}>", value.display()),
            Type::Function { params, result, .. } => {
                format!("({}) => {}", params.join(", "), result.display())
            }
            Type::Object(fields) => {
                if fields.is_empty() {
                    return "object".into();
                }
                let body = fields
                    .iter()
                    .map(|(name, ty)| format!("  {}: {};", name, ty.display()))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{{\n{}\n}}", body)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Binding {
    kind: String,
    ty: Type,
}

#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct Document {
    source: String,
    tokens: Vec<SpannedToken>,
    bindings: HashMap<String, Binding>,
    properties: HashMap<String, Type>,
}

impl Document {
    pub fn parse(source: &str) -> Self {
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_spans();
        let mut parser = crate::parser::Parser::new_with_spans(tokens.clone());
        let statements = parser.parse();
        let mut builder = Builder {
            bindings: HashMap::new(),
            properties: HashMap::new(),
        };
        builder.statements(&statements, &HashMap::new());
        Self {
            source: source.to_string(),
            tokens,
            bindings: builder.bindings,
            properties: builder.properties,
        }
    }

    pub fn hover(&self, offset: usize) -> Option<HoverInfo> {
        let token_index = self.token_at(offset)?;
        let (token, _) = &self.tokens[token_index];
        let name = match token {
            Token::Identifier(name) => name,
            _ => return None,
        };

        if token_index > 0 && matches!(self.tokens[token_index - 1].0, Token::Dot | Token::QuestionDot) {
            let receiver = self.receiver_name(token_index - 2)?;
            let ty = self
                .bindings
                .get(&receiver)
                .map(|binding| binding.ty.property(name))
                .or_else(|| self.properties.get(name).cloned())
                .unwrap_or(Type::Unknown);
            return Some(HoverInfo {
                detail: format!("(property) {}: {}", name, ty.display()),
            });
        }

        let binding = self.bindings.get(name)?;
        let detail = match &binding.kind[..] {
            "function" => format!("(function) {}: {}", name, binding.ty.display()),
            "parameter" => format!("(parameter) {}: {}", name, binding.ty.display()),
            "import" => format!("(import) {}: {}", name, binding.ty.display()),
            kind => format!("{} {}: {}", kind, name, binding.ty.display()),
        };
        Some(HoverInfo { detail })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    fn token_at(&self, offset: usize) -> Option<usize> {
        let offset = offset.min(self.source.len());
        let (line, col) = position_at(&self.source, offset);
        self.tokens.iter().enumerate().find_map(|(index, (token, span))| {
            let Token::Identifier(name) = token else { return None };
            if span.line == line && col >= span.col && col <= span.col + name.chars().count() {
                Some(index)
            } else {
                None
            }
        })
    }

    fn receiver_name(&self, index: usize) -> Option<String> {
        match self.tokens.get(index)?.0 {
            Token::Identifier(ref name) => Some(name.clone()),
            _ => None,
        }
    }
}

struct Builder {
    bindings: HashMap<String, Binding>,
    properties: HashMap<String, Type>,
}

impl Builder {
    fn statements(&mut self, statements: &[Statement], outer: &HashMap<String, Type>) {
        let mut env = outer.clone();
        for statement in statements {
            self.statement(statement, &mut env);
        }
    }

    fn statement(&mut self, statement: &Statement, env: &mut HashMap<String, Type>) {
        match statement {
            Statement::VarDecl { kind, name, init, .. } => {
                let ty = init.as_deref().map(|expr| self.expr(expr, env)).unwrap_or(Type::Unknown);
                let kind = match kind {
                    VarKind::Var => "var",
                    VarKind::Let => "let",
                    VarKind::Const => "const",
                };
                self.bindings.insert(name.clone(), Binding { kind: kind.into(), ty: ty.clone() });
                env.insert(name.clone(), ty);
            }
            Statement::FnDecl { name, params, body, is_async, .. } => {
                let result = self.function_result(params, body, env);
                let ty = Type::Function { params: params.clone(), result: Box::new(if *is_async { Type::Promise(Box::new(result)) } else { result }), async_fn: *is_async };
                self.bindings.insert(name.clone(), Binding { kind: "function".into(), ty: ty.clone() });
                env.insert(name.clone(), ty);
            }
            Statement::Import { default, named, namespace, .. } => {
                for name in default.iter().chain(namespace.iter()) {
                    self.bindings.insert(name.clone(), Binding { kind: "import".into(), ty: Type::Unknown });
                    env.insert(name.clone(), Type::Unknown);
                }
                for (_, name) in named {
                    self.bindings.insert(name.clone(), Binding { kind: "import".into(), ty: Type::Unknown });
                    env.insert(name.clone(), Type::Unknown);
                }
            }
            Statement::Expr(expr) => { self.expr(expr, env); }
            Statement::Block(body) => self.statements(body, env),
            Statement::If { then, else_, .. } => {
                self.statements(then, env);
                if let Some(body) = else_ { self.statements(body, env); }
            }
            Statement::While { body, .. } | Statement::DoWhile { body, .. } => self.statements(body, env),
            Statement::For { body, .. } => self.statements(body, env),
            Statement::ForIn { body, .. } | Statement::ForOf { body, .. } => self.statements(body, env),
            Statement::Try { body, catch, finally } => {
                self.statements(body, env);
                if let Some((name, body)) = catch {
                    self.bindings.insert(name.clone(), Binding { kind: "parameter".into(), ty: Type::Any });
                    self.statements(body, env);
                }
                if let Some(body) = finally { self.statements(body, env); }
            }
            _ => {}
        }
    }

    fn function_result(&mut self, params: &[String], body: &[Statement], outer: &HashMap<String, Type>) -> Type {
        let mut env = outer.clone();
        for param in params {
            env.insert(param.clone(), Type::Any);
            self.bindings.insert(param.clone(), Binding { kind: "parameter".into(), ty: Type::Any });
        }
        let mut result = Type::Unknown;
        for statement in body {
            if let Statement::Return(value) = statement {
                result = value.as_deref().map(|expr| self.expr(expr, &mut env)).unwrap_or(Type::Undefined);
            } else {
                self.statement(statement, &mut env);
            }
        }
        result
    }

    fn expr(&mut self, expr: &Expr, env: &mut HashMap<String, Type>) -> Type {
        match expr {
            Expr::Number(_) => Type::Number,
            Expr::String(_) | Expr::Template { .. } => Type::String,
            Expr::Bool(_) => Type::Boolean,
            Expr::Null => Type::Null,
            Expr::Undefined => Type::Undefined,
            Expr::Identifier(name) => env.get(name).cloned().unwrap_or(Type::Unknown),
            Expr::Object(props) => {
                let mut fields = BTreeMap::new();
                for prop in props {
                    match prop {
                        ObjectProp::Shorthand(name) => { fields.insert(name.clone(), Type::Any); }
                        ObjectProp::KeyValue(name, value) => {
                            let ty = self.expr(value, env);
                            self.properties.insert(name.clone(), ty.clone());
                            fields.insert(name.clone(), ty);
                        }
                        ObjectProp::Method { name, params, body } => {
                            fields.insert(name.clone(), Type::Function { params: params.clone(), result: Box::new(self.function_result(params, body, env)), async_fn: false });
                        }
                        ObjectProp::Getter { name, body } => { fields.insert(name.clone(), self.function_result(&[], body, env)); }
                        ObjectProp::Setter { name, param, body } => {
                            fields.insert(name.clone(), Type::Function { params: vec![param.clone()], result: Box::new(self.function_result(std::slice::from_ref(param), body, env)), async_fn: false });
                        }
                        ObjectProp::Computed(_, _) | ObjectProp::Spread(_) => {}
                    }
                }
                Type::Object(fields)
            }
            Expr::Array(items) => Type::Array(Box::new(items.first().map(|item| self.expr(item, env)).unwrap_or(Type::Unknown))),
            Expr::Await(value) => self.expr(value, env).unwrap_promise(),
            Expr::Call { callee, args } => {
                if let Expr::Member { object, property, .. } = callee.as_ref() {
                    if let Some(method) = expression_property_name(property) {
                        let object_ty = self.expr(object, env);
                        if method == "resolve" && matches!(object.as_ref(), Expr::Identifier(name) if name == "Promise") {
                            return Type::Promise(Box::new(args.first().map(|arg| self.expr(arg, env)).unwrap_or(Type::Undefined)));
                        }
                        if method == "then" {
                            if let Some(Expr::ArrowFn { params, body }) = args.first() {
                                let mut arrow_env = env.clone();
                                let value_ty = object_ty.unwrap_promise();
                                for param in params {
                                    arrow_env.insert(param.clone(), value_ty.clone());
                                    self.bindings.insert(param.clone(), Binding { kind: "parameter".into(), ty: value_ty.clone() });
                                }
                                let result = match body.as_ref() {
                                    ExprOrBlock::Expr(value) => self.expr(value, &mut arrow_env),
                                    ExprOrBlock::Block(body) => self.function_result(params, body, &arrow_env),
                                };
                                for param in params {
                                    self.bindings.insert(param.clone(), Binding { kind: "parameter".into(), ty: value_ty.clone() });
                                }
                                return Type::Promise(Box::new(result));
                            }
                        }
                    }
                }
                match self.expr(callee, env) {
                    Type::Function { result, .. } => result.as_ref().clone(),
                    _ => Type::Unknown,
                }
            }
            Expr::Member { object, property, .. } | Expr::OptionalChain { object, property, .. } => {
                let object_ty = self.expr(object, env);
                let name = match property.as_ref() { Expr::Identifier(name) | Expr::String(name) => name, _ => return Type::Unknown };
                object_ty.property(name)
            }
            Expr::ArrowFn { params, body } => {
                let result = match body.as_ref() {
                    ExprOrBlock::Expr(value) => self.expr(value, env),
                    ExprOrBlock::Block(body) => self.function_result(params, body, env),
                };
                Type::Function { params: params.clone(), result: Box::new(result), async_fn: false }
            }
            Expr::FnExpr { name: _, params, body, is_async, .. } => {
                let result = self.function_result(params, body, env);
                Type::Function {
                    params: params.clone(),
                    result: Box::new(if *is_async { Type::Promise(Box::new(result)) } else { result }),
                    async_fn: *is_async,
                }
            }
            Expr::Binary { op, left, right } => {
                let left_ty = self.expr(left, env);
                let right_ty = self.expr(right, env);
                if matches!(op, BinOp::Add) && (left_ty == Type::String || right_ty == Type::String) { Type::String } else { Type::Number }
            }
            Expr::New { .. } => Type::Object(BTreeMap::new()),
            Expr::Conditional { consequent, .. } => self.expr(consequent, env),
            Expr::Unary { .. } => Type::Number,
            Expr::Assignment { value, .. } => self.expr(value, env),
            Expr::Spread(value) => self.expr(value, env),
            Expr::This | Expr::Super | Expr::ImportMeta | Expr::Yield(_) => Type::Unknown,
        }
    }
}

fn position_at(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for ch in source[..offset].chars() {
        if ch == '\n' { line += 1; col = 1; } else { col += 1; }
    }
    (line, col)
}

fn expression_property_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Identifier(name) | Expr::String(name) => Some(name),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hover(source: &str, word: &str) -> String {
        let offset = source.rfind(word).unwrap() + 1;
        Document::parse(source)
            .hover(offset)
            .unwrap_or_else(|| panic!("no hover for {word} at {offset}"))
            .detail
    }

    #[test]
    fn infers_literal_variable() {
        assert_eq!(hover("let total = 0; total;", "total"), "let total: number");
    }

    #[test]
    fn infers_promise_callback_and_property() {
        let source = "async function loadUser(id) { const response = await Promise.resolve({ id, name: \"Ada\" }); return response; } loadUser(42).then((user) => { user.name; });";
        let user_hover = hover(source, "user");
        assert!(user_hover.contains("(parameter) user: {"));
        assert_eq!(hover(source, "name"), "(property) name: string");
    }
}
