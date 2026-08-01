//! Cached document service for editor and future LSP adapters.
//!
//! This is deliberately transport-neutral. A browser adapter can call it
//! directly through WASM, while an LSP adapter can map `didOpen`, `didChange`,
//! and request messages onto these methods without duplicating analysis.

use std::collections::HashMap;

use super::{AnalysisContext, Completion, Diagnostic, Document, HoverInfo, Symbol, complete, diagnose, symbols};

#[derive(Debug, Default)]
pub struct LanguageService {
    documents: HashMap<String, Document>,
}

impl LanguageService {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open or replace a document, equivalent to LSP `textDocument/didOpen`.
    pub fn open(&mut self, uri: impl Into<String>, source: &str) {
        self.documents.insert(uri.into(), Document::parse(source));
    }

    /// Replace a document snapshot, equivalent to LSP `textDocument/didChange`.
    pub fn update(&mut self, uri: &str, source: &str) -> bool {
        if !self.documents.contains_key(uri) {
            return false;
        }
        self.documents.insert(uri.to_string(), Document::parse(source));
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

    pub fn complete(&self, uri: &str, offset: usize, context: &AnalysisContext) -> Option<Vec<Completion>> {
        Some(complete(self.documents.get(uri)?.source(), offset, context))
    }

    pub fn diagnostics(&self, uri: &str) -> Option<Vec<Diagnostic>> {
        Some(diagnose(self.documents.get(uri)?.source()))
    }

    pub fn symbols(&self, uri: &str) -> Option<Vec<Symbol>> {
        Some(symbols(self.documents.get(uri)?.source()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_lifecycle_rebuilds_analysis() {
        let mut service = LanguageService::new();
        service.open("file:///async.js", "const total = 1; total;");
        let offset = service.source("file:///async.js").unwrap().rfind("total").unwrap() + 1;
        assert_eq!(service.hover("file:///async.js", offset).unwrap().detail, "const total: number");
        assert!(service.update("file:///async.js", "const total = \"ready\"; total;"));
        let offset = service.source("file:///async.js").unwrap().rfind("total").unwrap() + 1;
        assert_eq!(service.hover("file:///async.js", offset).unwrap().detail, "const total: string");
        assert!(service.close("file:///async.js"));
        assert!(service.hover("file:///async.js", 0).is_none());
    }
}
