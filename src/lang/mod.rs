//! Language services for the VM: completion, diagnostics, and document
//! symbols, built directly on the lexer/parser/AST.
//!
//! This module is the single, frontend-agnostic implementation of editor
//! intelligence. Every surface — the in-browser WASM playground, the LSP
//! server, and native GUIs (egui/eframe/slint) — calls these same functions,
//! so completion and analysis logic is written once and never recreated per
//! frontend.
//!
//! All entry points are pure functions of their inputs (source text + offset +
//! an [`AnalysisContext`] describing host-exposed functions and registered
//! modules), which keeps them trivially testable and safe to call from any
//! host.

pub mod catalog;
mod complete;
mod diagnostics;
mod document;
mod scope;
mod service;
mod symbols;

pub use complete::complete;
#[cfg(feature = "wasm")]
pub(crate) use complete::member_trigger;
pub use diagnostics::diagnose;
pub use document::{Document, HoverInfo, Type};
pub use service::LanguageService;

use crate::lexer::Lexer;
use crate::parser::Parser;

/// The kind of a completion candidate or symbol, for icon/badge rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Variable,
    Function,
    Method,
    Property,
    Class,
    Module,
    Keyword,
    Global,
    /// A host function exposed via `exposeFunction` / `exposeAsyncFunction`.
    ExposedFn,
}

/// One completion candidate.
#[derive(Debug, Clone)]
pub struct Completion {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
}

/// Runtime/workspace knowledge the static analyzer cannot derive from source
/// alone. The host fills this in: the playground knows which functions it
/// exposed and which modules it registered.
#[derive(Debug, Clone, Default)]
pub struct AnalysisContext {
    /// Names exposed to the VM as callable globals.
    pub exposed_functions: Vec<String>,
    /// Registered modules and their export names.
    pub modules: Vec<ModuleInfo>,
}

/// A registered module: its specifier and the names it exports.
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Hint,
}

/// A single diagnostic, positioned by 1-based line/column.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub line: usize,
    pub col: usize,
    pub message: String,
    pub severity: DiagnosticSeverity,
}

/// A top-level document symbol.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
}

/// Document symbols for a source program.
pub fn symbols(source: &str) -> Vec<Symbol> {
    let toks = Lexer::new(source).tokenize_with_spans();
    let mut parser = Parser::new_with_spans(toks);
    let stmts = parser.parse();
    symbols::symbols(&stmts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(v: &[Completion]) -> Vec<&str> {
        v.iter().map(|c| c.label.as_str()).collect()
    }

    #[test]
    fn ident_offers_globals() {
        let r = complete("Ma", 2, &AnalysisContext::default());
        assert!(labels(&r).contains(&"Math"));
    }

    #[test]
    fn ident_offers_scope_decls() {
        // Trailing newline → empty prefix, so every in-scope name is offered.
        let src = "const counter = 1;\nfunction bump() {}\n";
        let r = complete(src, src.len(), &AnalysisContext::default());
        let l = labels(&r);
        assert!(l.contains(&"counter") && l.contains(&"bump"));
    }

    #[test]
    fn ident_offers_exposed_functions() {
        let ctx = AnalysisContext {
            exposed_functions: vec!["add".into()],
            ..Default::default()
        };
        let r = complete("ad", 2, &ctx);
        let add = r.iter().find(|c| c.label == "add").expect("add offered");
        assert_eq!(add.kind, CompletionKind::ExposedFn);
    }

    #[test]
    fn ident_offers_module_names() {
        let ctx = AnalysisContext {
            modules: vec![ModuleInfo {
                name: "utils".into(),
                exports: vec![],
            }],
            ..Default::default()
        };
        let r = complete("ut", 2, &ctx);
        assert!(labels(&r).contains(&"utils"));
    }

    #[test]
    fn member_builtin_math() {
        let src = "Math.fl";
        let r = complete(src, src.len(), &AnalysisContext::default());
        assert_eq!(labels(&r), vec!["floor"]);
    }

    #[test]
    fn member_object_literal_keys() {
        let src = "const user = { name: 1, age: 2, greet() {} };\nuser.";
        let r = complete(src, src.len(), &AnalysisContext::default());
        let l = labels(&r);
        assert!(l.contains(&"name") && l.contains(&"age") && l.contains(&"greet"));
    }

    #[test]
    fn member_array_literal() {
        let src = "[1, 2, 3].fi";
        let r = complete(src, src.len(), &AnalysisContext::default());
        let l = labels(&r);
        assert!(l.contains(&"filter") && l.contains(&"find"));
    }

    #[test]
    fn member_array_var() {
        let src = "const xs = [1, 2];\nxs.ma";
        let r = complete(src, src.len(), &AnalysisContext::default());
        assert!(labels(&r).contains(&"map"));
    }

    #[test]
    fn member_string_literal() {
        let src = "\"abc\".to";
        let r = complete(src, src.len(), &AnalysisContext::default());
        assert!(labels(&r).contains(&"toUpperCase"));
    }

    #[test]
    fn member_namespace_import_exports() {
        let src = "import * as u from 'utils';\nu.";
        let ctx = AnalysisContext {
            modules: vec![ModuleInfo {
                name: "utils".into(),
                exports: vec!["format".into(), "parse".into()],
            }],
            ..Default::default()
        };
        let r = complete(src, src.len(), &ctx);
        let l = labels(&r);
        assert!(l.contains(&"format") && l.contains(&"parse"));
    }

    #[test]
    fn diagnostics_balanced_ok() {
        assert!(diagnose("const x = [1, 2, { a: (3) }];").is_empty());
    }

    #[test]
    fn diagnostics_unbalanced() {
        assert!(!diagnose("const x = [1, 2;").is_empty());
    }

    #[test]
    fn diagnostics_template_balances() {
        let d = diagnose("const s = `a${x}b`;");
        assert!(d.is_empty(), "template should balance: {:?}", d);
    }

    #[test]
    fn symbols_top_level() {
        let s = symbols("function f() {} class C {} const x = 1;");
        let names: Vec<&str> = s.iter().map(|x| x.name.as_str()).collect();
        assert!(names.contains(&"f") && names.contains(&"C") && names.contains(&"x"));
    }

    #[test]
    fn ident_playground_prefix() {
        let ctx = AnalysisContext {
            modules: vec![
                ModuleInfo {
                    name: "utils".into(),
                    exports: vec!["format".into(), "parse".into()],
                },
                ModuleInfo {
                    name: "math".into(),
                    exports: vec!["PI".into()],
                },
            ],
            ..Default::default()
        };
        let r = complete("@playground/u", "@playground/u".len(), &ctx);
        let labels: Vec<&str> = r.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"@playground/utils"));
    }

    #[test]
    fn member_playground_namespace_exports() {
        let ctx = AnalysisContext {
            modules: vec![ModuleInfo {
                name: "utils".into(),
                exports: vec!["format".into(), "parse".into()],
            }],
            ..Default::default()
        };
        let r = complete("@playground/utils.", "@playground/utils.".len(), &ctx);
        let labels: Vec<&str> = r.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"format") && labels.contains(&"parse"));
    }
}
