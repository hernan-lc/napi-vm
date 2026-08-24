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
mod metadata;
mod scope;
mod service;
mod symbols;

pub use complete::complete;
#[cfg(feature = "wasm")]
pub(crate) use complete::member_trigger;
pub use diagnostics::diagnose;
pub use document::{Document, HoverInfo, Type};
pub use metadata::{GlobalInfo, ParameterInfo, PropertyInfo, Shape};
pub(crate) use metadata::{MAX_MANIFEST_BYTES, parse_globals};
pub use service::LanguageService;

use crate::lexer::Lexer;
use crate::parser::Parser;
use std::collections::HashMap;

/// Virtual module namespace used by playground completion.
///
/// This is a language-level specifier, not a filesystem path. Keeping it in
/// the language module prevents completion code from coupling itself to the
/// playground's current directory layout.
pub const MODULE_NAMESPACE_PREFIX: &str = "@playground/";

/// Extract a module name from a virtual playground module specifier.
pub fn playground_module_name(specifier: &str) -> Option<&str> {
    specifier
        .strip_prefix(MODULE_NAMESPACE_PREFIX)
        .filter(|name| !name.is_empty())
}

/// Build the virtual specifier used by completion for a playground module.
pub fn playground_module_specifier(module_name: &str) -> String {
    format!("{MODULE_NAMESPACE_PREFIX}{module_name}")
}

/// Return the text being typed after the last playground namespace prefix.
pub fn playground_completion_module_prefix(source: &str) -> Option<&str> {
    source
        .rsplit_once(MODULE_NAMESPACE_PREFIX)
        .map(|(_, rest)| rest)
}

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

/// A parameter description supplied by the host for a JavaScript function
/// exposed to the VM. JavaScript does not retain TypeScript annotations at
/// runtime, so hosts provide this metadata explicitly when they want rich
/// hover and completion information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFunctionParameter {
    pub name: String,
    pub type_name: String,
}

/// Language-service metadata for a host-exposed function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFunctionInfo {
    pub name: String,
    pub params: Vec<HostFunctionParameter>,
    pub return_type: String,
    pub documentation: Option<String>,
    pub async_fn: bool,
}

impl HostFunctionInfo {
    pub fn global_info(&self) -> GlobalInfo {
        GlobalInfo {
            name: self.name.clone(),
            shape: Shape::Function {
                params: self
                    .params
                    .iter()
                    .map(|parameter| ParameterInfo {
                        name: parameter.name.clone(),
                        shape: metadata_shape_from_string(&parameter.type_name),
                    })
                    .collect(),
                returns: Box::new(metadata_shape_from_string(&self.return_type)),
                async_fn: self.async_fn,
            },
            documentation: self.documentation.clone(),
        }
    }
}

impl HostFunctionInfo {
    pub fn unknown(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            return_type: "unknown".into(),
            documentation: None,
            async_fn: false,
        }
    }

    pub fn signature(&self) -> String {
        let params = if self.params.is_empty() {
            // An explicitly typed zero-argument function is different from
            // an untyped host callback. The latter keeps the conservative
            // `...args` fallback; metadata with a concrete return type (or
            // documentation) can accurately advertise `()`.
            if self.return_type == "unknown" && self.documentation.is_none() {
                "...args".into()
            } else {
                String::new()
            }
        } else {
            self.params
                .iter()
                .map(|param| format!("{}: {}", param.name, param.type_name))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let return_type = if self.async_fn && !self.return_type.starts_with("Promise<") {
            format!("Promise<{}>", self.return_type)
        } else {
            self.return_type.clone()
        };
        format!("({}) => {}", params, return_type)
    }
}

fn metadata_shape_from_string(value: &str) -> Shape {
    let value = value.trim();
    if let Some(inner) = value.strip_suffix("[]") {
        return Shape::Array(Box::new(metadata_shape_from_string(inner)));
    }
    if let Some(inner) = value
        .strip_prefix("Promise<")
        .and_then(|value| value.strip_suffix('>'))
    {
        return Shape::Promise(Box::new(metadata_shape_from_string(inner)));
    }
    match value {
        "any" => Shape::Any,
        "void" => Shape::Void,
        "undefined" => Shape::Undefined,
        "null" => Shape::Null,
        "boolean" => Shape::Boolean,
        "number" => Shape::Number,
        "string" => Shape::String,
        "object" => Shape::Object(std::collections::BTreeMap::new()),
        "function" => Shape::Function {
            params: Vec::new(),
            returns: Box::new(Shape::Unknown),
            async_fn: false,
        },
        _ => Shape::Unknown,
    }
}

/// Runtime/workspace knowledge the static analyzer cannot derive from source
/// alone. The host fills this in: the playground knows which functions it
/// exposed and which modules it registered.
#[derive(Debug, Clone, Default)]
pub struct AnalysisContext {
    /// Functions exposed to the VM as callable globals, including optional
    /// parameter, return-type, and documentation metadata.
    pub exposed_functions: Vec<HostFunctionInfo>,
    /// Registered modules and their export names.
    pub modules: Vec<ModuleInfo>,
    /// Shapes observed for VM event handlers. The key is the handler function
    /// name and the value is assigned to its first parameter for completion.
    pub runtime_handlers: HashMap<String, Type>,
    /// Generic globals published by the currently connected runtime.
    pub runtime_globals: HashMap<String, GlobalInfo>,
    /// Generic globals loaded from the optional static LSP manifest.
    pub manifest_globals: HashMap<String, GlobalInfo>,
}

impl AnalysisContext {
    /// Resolve custom metadata using the one service-wide precedence order.
    /// Legacy host functions remain a separate compatibility layer; callers
    /// merge them after this map so explicit generic runtime metadata wins.
    pub(crate) fn resolved_globals(&self) -> HashMap<String, GlobalInfo> {
        let mut globals = self.manifest_globals.clone();
        globals.extend(self.runtime_globals.clone());
        globals
    }
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
            exposed_functions: vec![HostFunctionInfo::unknown("add")],
            ..Default::default()
        };
        let r = complete("ad", 2, &ctx);
        let add = r.iter().find(|c| c.label == "add").expect("add offered");
        assert_eq!(add.kind, CompletionKind::ExposedFn);
    }

    #[test]
    fn exposed_function_metadata_is_available_to_completion() {
        let ctx = AnalysisContext {
            exposed_functions: vec![HostFunctionInfo {
                name: "alert".into(),
                params: vec![HostFunctionParameter {
                    name: "message".into(),
                    type_name: "string".into(),
                }],
                return_type: "void".into(),
                documentation: Some("Displays a message.".into()),
                async_fn: false,
            }],
            ..Default::default()
        };
        let alert = complete("al", 2, &ctx)
            .into_iter()
            .find(|item| item.label == "alert")
            .expect("alert offered");
        assert_eq!(alert.detail.as_deref(), Some("(message: string) => void"));
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
    fn member_generic_global() {
        let src = "ipc.in";
        let ctx = AnalysisContext {
            runtime_globals: std::collections::BTreeMap::from([("ipc".into(), ipc_global())])
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let r = complete(src, src.len(), &ctx);
        let l = labels(&r);
        assert!(l.contains(&"invoke") && l.contains(&"invokeAsync"));
    }

    #[test]
    fn unknown_local_binding_still_shadows_builtin_members() {
        let source = "let Math;\nMath.";
        let r = complete(source, source.len(), &AnalysisContext::default());
        assert!(r.is_empty());
    }

    #[test]
    fn member_global_alias_includes_generic_global() {
        let src = "window.ipc.";
        let ctx = AnalysisContext {
            runtime_globals: std::collections::BTreeMap::from([("ipc".into(), ipc_global())])
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let r = complete(src, src.len(), &ctx);
        let l = labels(&r);
        assert!(l.contains(&"invoke") && l.contains(&"commands"));
    }

    #[test]
    fn ipc_is_absent_without_metadata() {
        let r = complete("ipc.", 4, &AnalysisContext::default());
        assert!(r.is_empty());
    }

    fn ipc_global() -> GlobalInfo {
        GlobalInfo {
            name: "ipc".into(),
            shape: Shape::Object(std::collections::BTreeMap::from([
                (
                    "invoke".into(),
                    PropertyInfo {
                        shape: Shape::Function {
                            params: vec![ParameterInfo {
                                name: "command".into(),
                                shape: Shape::String,
                            }],
                            returns: Box::new(Shape::Unknown),
                            async_fn: false,
                        },
                        documentation: Some("Invoke a command.".into()),
                    },
                ),
                (
                    "invokeAsync".into(),
                    PropertyInfo {
                        shape: Shape::Function {
                            params: vec![],
                            returns: Box::new(Shape::Unknown),
                            async_fn: true,
                        },
                        documentation: None,
                    },
                ),
                (
                    "commands".into(),
                    PropertyInfo {
                        shape: Shape::Function {
                            params: vec![],
                            returns: Box::new(Shape::Array(Box::new(Shape::String))),
                            async_fn: false,
                        },
                        documentation: None,
                    },
                ),
            ])),
            documentation: None,
        }
    }

    #[test]
    fn member_global_alias_includes_exposed_host_functions() {
        let ctx = AnalysisContext {
            exposed_functions: vec![HostFunctionInfo::unknown("hostNow")],
            ..Default::default()
        };
        let src = "window.host";
        let r = complete(src, src.len(), &ctx);
        assert!(labels(&r).contains(&"hostNow"));
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
        let source = format!("{MODULE_NAMESPACE_PREFIX}u");
        let r = complete(&source, source.len(), &ctx);
        let labels: Vec<&str> = r.iter().map(|c| c.label.as_str()).collect();
        let expected = format!("{MODULE_NAMESPACE_PREFIX}utils");
        assert!(labels.contains(&expected.as_str()));
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
        let source = format!("{MODULE_NAMESPACE_PREFIX}utils.");
        let r = complete(&source, source.len(), &ctx);
        let labels: Vec<&str> = r.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"format") && labels.contains(&"parse"));
    }
}
