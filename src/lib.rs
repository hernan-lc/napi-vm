use napi::{bindgen_prelude::*, Error as NapiError};
use napi_derive::napi;
use swc::Compiler;
use swc_common::{FileName, SourceMap, sync::Lrc, errors::Handler, GLOBALS, Globals};
use swc_ecma_ast::*;
use swc_ecma_parser::{Syntax, TsSyntax, EsSyntax};
use swc_ecma_transforms::typescript::strip;
use swc_ecma_ast::noop_pass;

fn compile_with_syntax(source: String, syntax: Syntax) -> std::result::Result<String, NapiError> {
    let cm: Lrc<SourceMap> = Lrc::new(SourceMap::default());
    let compiler = Compiler::new(cm.clone());

    let fm = cm.new_source_file(Lrc::new(FileName::Custom("input.ts".into())), source);

    let handler = Handler::with_emitter(
        true,
        false,
        Box::new(swc_common::errors::EmitterWriter::new(
            Box::new(std::io::sink()),
            Some(cm.clone()),
            false,
            false,
        )),
    );

    GLOBALS.set(&Globals::new(), || {
        let unresolved_mark = swc_common::Mark::new();
        let top_level_mark = swc_common::Mark::new();

        let program = compiler
            .parse_js(
                fm.clone(),
                &handler,
                EsVersion::EsNext,
                syntax.clone(),
                swc::config::IsModule::Unknown,
                None,
            )
            .map_err(|e| NapiError::new(Status::GenericFailure, format!("Parse error: {}", e)))?;

        let output = compiler.process_js_with_custom_pass(
            fm,
            Some(program),
            &handler,
            &swc::config::Options {
                config: swc::config::Config {
                    jsc: swc::config::JscConfig {
                        syntax: Some(syntax),
                        target: Some(EsVersion::EsNext),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            swc_common::comments::SingleThreadedComments::default(),
            |_| strip(unresolved_mark, top_level_mark),
            |_| noop_pass(),
        )
        .map_err(|e| NapiError::new(Status::GenericFailure, format!("Transform error: {}", e)))?;

        Ok(output.code)
    })
}

#[napi]
pub struct VM {
    _compiler: Compiler,
}

#[napi]
impl VM {
    #[napi(constructor)]
    pub fn new() -> Self {
        let cm: Lrc<SourceMap> = Lrc::new(SourceMap::default());
        Self {
            _compiler: Compiler::new(cm),
        }
    }

    #[napi]
    pub fn compile_ts(&self, source: String) -> std::result::Result<String, NapiError> {
        compile_with_syntax(source, Syntax::Typescript(TsSyntax::default()))
    }

    #[napi]
    pub fn compile_js(&self, source: String) -> std::result::Result<String, NapiError> {
        compile_with_syntax(source, Syntax::Es(EsSyntax::default()))
    }
}

#[napi]
pub fn compile_ts(source: String) -> std::result::Result<String, NapiError> {
    compile_with_syntax(source, Syntax::Typescript(TsSyntax::default()))
}

#[napi]
pub fn compile_js(source: String) -> std::result::Result<String, NapiError> {
    compile_with_syntax(source, Syntax::Es(EsSyntax::default()))
}
