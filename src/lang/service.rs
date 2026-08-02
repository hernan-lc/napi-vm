//! Cached document service for editor and future LSP adapters.
//!
//! This is deliberately transport-neutral. A browser adapter can call it
//! directly through WASM, while an LSP adapter can map `didOpen`, `didChange`,
//! and request messages onto these methods without duplicating analysis.

use std::collections::HashMap;

use super::{
    AnalysisContext, Completion, Diagnostic, Document, HostFunctionInfo, HoverInfo, ModuleInfo,
    Symbol, Type, complete, diagnose, symbols,
};

#[derive(Debug, Default)]
pub struct LanguageService {
    documents: HashMap<String, Document>,
    modules: HashMap<String, ModuleInfo>,
    module_sources: HashMap<String, String>,
    host_functions: HashMap<String, HostFunctionInfo>,
    runtime_handlers: HashMap<String, Type>,
}

impl LanguageService {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open or replace a document, equivalent to LSP `textDocument/didOpen`.
    pub fn open(&mut self, uri: impl Into<String>, source: &str) {
        let uri = uri.into();
        self.documents.insert(uri, self.parse(source));
    }

    /// Replace a document snapshot, equivalent to LSP `textDocument/didChange`.
    pub fn update(&mut self, uri: &str, source: &str) -> bool {
        if !self.documents.contains_key(uri) {
            return false;
        }
        self.documents.insert(uri.to_string(), self.parse(source));
        true
    }

    pub fn register_module(&mut self, name: impl Into<String>, source: &str) {
        let name = name.into();
        let document = Document::parse_with_context(source, &self.module_sources, &self.hosts());
        self.modules.insert(
            name.clone(),
            ModuleInfo {
                name: name.clone(),
                exports: document.export_names(),
            },
        );
        self.module_sources.insert(name, source.to_string());
        self.rebuild_documents();
    }

    pub fn register_host_function(&mut self, function: HostFunctionInfo) {
        self.host_functions.insert(function.name.clone(), function);
        self.rebuild_documents();
    }

    /// Register a JSON shape observed for a VM event handler. The shape is
    /// intentionally metadata-only: it is never executed and only improves
    /// completion/hover for the handler's first parameter.
    pub fn register_runtime_shape(&mut self, name: impl Into<String>, shape_json: &str) -> bool {
        let Ok(shape) = serde_json::from_str::<serde_json::Value>(shape_json) else {
            return false;
        };
        self.runtime_handlers
            .insert(name.into(), Type::from_runtime_shape(&shape));
        self.rebuild_documents();
        true
    }

    /// Close and release a document, equivalent to LSP `didClose`.
    pub fn close(&mut self, uri: &str) -> bool {
        self.documents.remove(uri).is_some()
    }

    pub fn source(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(Document::source)
    }

    pub fn hover(&self, uri: &str, offset: usize) -> Option<HoverInfo> {
        self.documents.get(uri)?.hover(offset)
    }

    pub fn complete(
        &self,
        uri: &str,
        offset: usize,
        context: &AnalysisContext,
    ) -> Option<Vec<Completion>> {
        Some(complete(self.documents.get(uri)?.source(), offset, context))
    }

    pub fn diagnostics(&self, uri: &str) -> Option<Vec<Diagnostic>> {
        Some(diagnose(self.documents.get(uri)?.source()))
    }

    pub fn symbols(&self, uri: &str) -> Option<Vec<Symbol>> {
        Some(symbols(self.documents.get(uri)?.source()))
    }

    pub fn context(&self) -> AnalysisContext {
        AnalysisContext {
            exposed_functions: self.hosts(),
            modules: self.modules.values().cloned().collect(),
            runtime_handlers: self.runtime_handlers.clone(),
        }
    }

    fn hosts(&self) -> Vec<HostFunctionInfo> {
        let mut hosts: Vec<_> = self.host_functions.values().cloned().collect();
        hosts.sort_by(|a, b| a.name.cmp(&b.name));
        hosts
    }

    fn parse(&self, source: &str) -> Document {
        Document::parse_with_context_and_runtime(
            source,
            &self.module_sources,
            &self.hosts(),
            &self.runtime_handlers,
        )
    }

    fn rebuild_documents(&mut self) {
        let sources: Vec<_> = self
            .documents
            .iter()
            .map(|(uri, document)| (uri.clone(), document.source().to_string()))
            .collect();
        for (uri, source) in sources {
            self.documents.insert(uri, self.parse(&source));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::HostFunctionParameter;

    #[test]
    fn document_lifecycle_rebuilds_analysis() {
        let mut service = LanguageService::new();
        service.open("file:///async.js", "const total = 1; total;");
        let offset = service
            .source("file:///async.js")
            .unwrap()
            .rfind("total")
            .unwrap()
            + 1;
        assert_eq!(
            service.hover("file:///async.js", offset).unwrap().detail,
            "const total: number"
        );
        assert!(service.update("file:///async.js", "const total = \"ready\"; total;"));
        let offset = service
            .source("file:///async.js")
            .unwrap()
            .rfind("total")
            .unwrap()
            + 1;
        assert_eq!(
            service.hover("file:///async.js", offset).unwrap().detail,
            "const total: string"
        );
        assert!(service.close("file:///async.js"));
        assert!(service.hover("file:///async.js", 0).is_none());
    }

    #[test]
    fn host_metadata_rebuilds_open_documents() {
        let mut service = LanguageService::new();
        service.open("file:///main.js", "alert(\"hello\");");
        service.register_host_function(HostFunctionInfo {
            name: "alert".into(),
            params: vec![HostFunctionParameter {
                name: "message".into(),
                type_name: "string".into(),
            }],
            return_type: "void".into(),
            documentation: Some("Displays a message.".into()),
            async_fn: false,
        });

        let hover = service.hover("file:///main.js", 2).unwrap();
        assert_eq!(hover.detail, "(function) alert: (message: string) => void");
        assert_eq!(hover.documentation.as_deref(), Some("Displays a message."));
        let completions = service
            .complete("file:///main.js", 2, &service.context())
            .unwrap();
        assert!(completions.iter().any(|item| item.label == "alert"));
    }

    #[test]
    fn registered_modules_feed_import_inference() {
        let mut service = LanguageService::new();
        service.register_module(
            "math",
            "export function double(value) { return value * 2; }",
        );
        service.open(
            "file:///main.js",
            "import { double } from \"math\";\ndouble;",
        );

        let source = service.source("file:///main.js").unwrap();
        let offset = source.rfind("double").unwrap() + 1;
        let hover = service.hover("file:///main.js", offset).unwrap();
        assert!(hover.detail.contains("double"));
        assert!(hover.detail.contains("=> number"));
    }

    #[test]
    fn runtime_json_shape_feeds_handler_completion() {
        let mut service = LanguageService::new();
        service.register_runtime_shape(
            "handleChat",
            r#"{
                "kind":"object",
                "properties":{
                    "platform":{"kind":"string"},
                    "data":{"kind":"object","properties":{
                        "nickname":{"kind":"string"},
                        "comment":{"kind":"string"}
                    }}
                }
            }"#,
        );
        let source = "function handleChat(event) { event.data.";
        service.open("file:///chat.js", source);

        let completions = service
            .complete("file:///chat.js", source.len(), &service.context())
            .unwrap();
        let labels: Vec<_> = completions.iter().map(|item| item.label.as_str()).collect();
        assert!(labels.contains(&"nickname"));
        assert!(labels.contains(&"comment"));

        let event_offset = source.find("event").unwrap() + 2;
        let hover = service.hover("file:///chat.js", event_offset).unwrap();
        assert!(hover.detail.contains("parameter"));
        assert!(hover.detail.contains("platform"));
    }
}
