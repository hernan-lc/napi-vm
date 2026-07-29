mod ast;
mod expr;
mod stmt;

pub use ast::*;

use crate::lexer::Token;

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

    pub(crate) fn cur(&self) -> &Token {
        self.toks.get(self.pos).unwrap_or(&Token::EOF)
    }

    pub(crate) fn adv(&mut self) -> &Token {
        if self.pos < self.toks.len() {
            self.pos += 1;
        }
        self.toks.get(self.pos - 1).unwrap_or(&Token::EOF)
    }

    pub(crate) fn eat(&mut self, t: &Token) -> bool {
        if self.cur() == t {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    pub(crate) fn eof(&self) -> bool {
        matches!(self.cur(), Token::EOF)
    }

    pub(crate) fn semi(&mut self) {
        if matches!(self.cur(), Token::Semicolon) {
            self.pos += 1;
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
        assert!(
            matches!(&stmts[0], Statement::VarDecl { kind: VarKind::Const, name, .. } if name == "x")
        );
    }

    #[test]
    fn test_fn_decl() {
        let stmts = parse("function add(a, b) { return a + b; }");
        assert_eq!(stmts.len(), 1);
        assert!(
            matches!(&stmts[0], Statement::FnDecl { name, params, .. } if name == "add" && params.len() == 2)
        );
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
        if let Statement::VarDecl {
            init: Some(init), ..
        } = &stmts[0]
        {
            assert!(matches!(init.as_ref(), Expr::ArrowFn { .. }));
        } else {
            panic!("expected var decl with init");
        }
    }

    #[test]
    fn test_class_decl() {
        let stmts = parse("class Foo { constructor() {} bar() {} }");
        assert_eq!(stmts.len(), 1);
        assert!(
            matches!(&stmts[0], Statement::ClassDecl { name, body, .. } if name == "Foo" && body.len() == 2)
        );
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
        assert!(
            matches!(&stmts[0], Statement::Import { module, named, .. } if module == "bar" && named.len() == 1)
        );
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
        assert!(matches!(
            &stmts[0],
            Statement::Expr(Expr::Conditional { .. })
        ));
    }

    #[test]
    fn test_member_access() {
        let stmts = parse("obj.prop;");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(
            &stmts[0],
            Statement::Expr(Expr::Member {
                computed: false,
                ..
            })
        ));
    }

    #[test]
    fn test_computed_member() {
        let stmts = parse("arr[0];");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(
            &stmts[0],
            Statement::Expr(Expr::Member { computed: true, .. })
        ));
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
