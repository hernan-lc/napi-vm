use napi_derive::napi;

use crate::lang::{
    CompletionKind, HostFunctionInfo, HostFunctionParameter, LanguageService as CoreLanguageService,
};

#[napi(object)]
pub struct NapiHostFunctionParameter {
    pub name: String,
    pub type_name: String,
}

#[napi(object)]
pub struct NapiCompletion {
    pub label: String,
    pub kind: String,
    pub detail: Option<String>,
}

#[napi(object)]
pub struct NapiHover {
    pub detail: String,
    pub documentation: Option<String>,
}

#[napi(object)]
pub struct NapiDiagnostic {
    pub line: u32,
    pub col: u32,
    pub message: String,
    pub severity: String,
}

#[napi]
pub struct LanguageService {
    inner: CoreLanguageService,
}

impl Default for LanguageService {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl LanguageService {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreLanguageService::new(),
        }
    }

    #[napi]
    pub fn open(&mut self, uri: String, source: String) {
        self.inner.open(uri, &source);
    }

    #[napi]
    pub fn update(&mut self, uri: String, source: String) -> bool {
        self.inner.update(&uri, &source)
    }

    #[napi]
    pub fn close(&mut self, uri: String) -> bool {
        self.inner.close(&uri)
    }

    #[napi(js_name = "registerModule")]
    pub fn register_module(&mut self, name: String, source: String) {
        self.inner.register_module(name, &source);
    }

    #[napi(js_name = "registerHostFunction")]
    pub fn register_host_function(
        &mut self,
        name: String,
        params: Vec<NapiHostFunctionParameter>,
        returns: String,
        documentation: Option<String>,
        async_fn: Option<bool>,
    ) {
        self.inner.register_host_function(HostFunctionInfo {
            name,
            params: params
                .into_iter()
                .map(|param| HostFunctionParameter {
                    name: param.name,
                    type_name: param.type_name,
                })
                .collect(),
            return_type: returns,
            documentation,
            async_fn: async_fn.unwrap_or(false),
        });
    }

    #[napi]
    pub fn complete(&self, uri: String, offset: u32) -> Vec<NapiCompletion> {
        let context = self.inner.context();
        self.inner
            .complete(&uri, offset as usize, &context)
            .unwrap_or_default()
            .into_iter()
            .map(|item| NapiCompletion {
                label: item.label,
                kind: completion_kind(item.kind).into(),
                detail: item.detail,
            })
            .collect()
    }

    #[napi]
    pub fn hover(&self, uri: String, offset: u32) -> Option<NapiHover> {
        self.inner
            .hover(&uri, offset as usize)
            .map(|info| NapiHover {
                detail: info.detail,
                documentation: info.documentation,
            })
    }

    #[napi]
    pub fn diagnostics(&self, uri: String) -> Vec<NapiDiagnostic> {
        self.inner
            .diagnostics(&uri)
            .unwrap_or_default()
            .into_iter()
            .map(|diagnostic| NapiDiagnostic {
                line: diagnostic.line as u32,
                col: diagnostic.col as u32,
                message: diagnostic.message,
                severity: severity(diagnostic.severity).into(),
            })
            .collect()
    }
}

fn completion_kind(kind: CompletionKind) -> &'static str {
    match kind {
        CompletionKind::Variable => "variable",
        CompletionKind::Function => "function",
        CompletionKind::Method => "method",
        CompletionKind::Property => "property",
        CompletionKind::Class => "class",
        CompletionKind::Module => "module",
        CompletionKind::Keyword => "keyword",
        CompletionKind::Global => "global",
        CompletionKind::ExposedFn => "exposed",
    }
}

fn severity(value: crate::lang::DiagnosticSeverity) -> &'static str {
    match value {
        crate::lang::DiagnosticSeverity::Error => "error",
        crate::lang::DiagnosticSeverity::Warning => "warning",
        crate::lang::DiagnosticSeverity::Hint => "hint",
    }
}
