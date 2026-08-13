use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use serde_json::{Value, json};

use crate::lang::{
    CompletionKind, DiagnosticSeverity, HostFunctionInfo, HostFunctionParameter, LanguageService,
};

use super::runtime_client::{RuntimeClient, RuntimeEvent};
use super::text::{diagnostic_position, position_to_offset, uri_path};

const SERVER_NAME: &str = "napi-vm-lsp";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

struct Server {
    service: LanguageService,
    documents: HashMap<String, String>,
    root: PathBuf,
    shutdown: bool,
    static_manifest: Option<Value>,
    runtime_snapshot: Option<Value>,
    runtime_client: Option<RuntimeClient>,
    runtime_rx: Option<mpsc::Receiver<RuntimeEvent>>,
}

impl Server {
    fn new() -> Self {
        Self {
            service: LanguageService::new(),
            documents: HashMap::new(),
            root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            shutdown: false,
            static_manifest: None,
            runtime_snapshot: None,
            runtime_client: None,
            runtime_rx: None,
        }
    }

    fn argument(name: &str) -> Option<String> {
        let mut args = std::env::args();
        while let Some(arg) = args.next() {
            if arg == name {
                return args.next();
            }
        }
        None
    }

    fn manifest_path(&self) -> Option<PathBuf> {
        Self::argument("--config").map(|path| self.root.join(path))
    }

    fn load_manifest(&self) -> Option<Value> {
        let path = self.manifest_path()?;
        let text = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn register_manifest(target: &mut LanguageService, root: &Path, manifest: &Value) {
        if let Some(hosts) = manifest.get("hostFunctions").and_then(Value::as_array) {
            for host in hosts {
                register_host(target, host);
            }
        }
        if let Some(modules) = manifest.get("modules").and_then(Value::as_array) {
            for module in modules {
                let Some(name) = module.get("name").and_then(Value::as_str) else {
                    continue;
                };
                let Some(rel) = module.get("path").and_then(Value::as_str) else {
                    continue;
                };
                let path = root.join(rel);
                if let Ok(source) = std::fs::read_to_string(path) {
                    target.register_module(name, &source);
                }
            }
        }
    }

    fn register_runtime(target: &mut LanguageService, snapshot: &Value) {
        if let Some(hosts) = snapshot.get("functions").and_then(Value::as_array) {
            for host in hosts {
                register_host(target, host);
            }
        }
        if let Some(modules) = snapshot.get("modules").and_then(Value::as_array) {
            for module in modules {
                let Some(name) = module.get("name").and_then(Value::as_str) else {
                    continue;
                };
                if let Some(source) = module.get("source").and_then(Value::as_str) {
                    target.register_module(name, source);
                }
            }
        }
        if let Some(handlers) = snapshot.get("handlers").and_then(Value::as_array) {
            for handler in handlers {
                let Some(name) = handler.get("name").and_then(Value::as_str) else {
                    continue;
                };
                let Some(shape) = handler.get("shape") else {
                    continue;
                };
                let Ok(shape_json) = serde_json::to_string(shape) else {
                    continue;
                };
                let last_value = handler
                    .get("lastValue")
                    .filter(|value| !value.is_null())
                    .and_then(|value| serde_json::to_string(value).ok());
                target.register_runtime_shape(name, &shape_json, last_value.as_deref());
            }
        }
    }

    fn rebuild_service(&mut self) {
        let mut next = LanguageService::new();
        if let Some(manifest) = &self.static_manifest {
            Self::register_manifest(&mut next, &self.root, manifest);
        }
        if let Some(snapshot) = &self.runtime_snapshot {
            Self::register_runtime(&mut next, snapshot);
        }
        for (uri, text) in &self.documents {
            next.open(uri.clone(), text);
        }
        self.service = next;
        let uris: Vec<_> = self.documents.keys().cloned().collect();
        for uri in uris {
            self.publish_diagnostics(&uri);
        }
    }

    fn connect_runtime(&mut self) {
        if let Some(client) = self.runtime_client.as_mut() {
            client.stop();
        }
        let (tx, rx) = mpsc::channel();
        self.runtime_client = Some(RuntimeClient::start(self.root.clone(), tx));
        self.runtime_rx = Some(rx);
    }

    fn handle_runtime(&mut self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::Snapshot(snapshot) => {
                self.runtime_snapshot = snapshot;
                self.rebuild_service();
            }
            RuntimeEvent::Error(error) => {
                let _ = writeln!(io::stderr(), "[napi-vm-lsp/runtime] {error}");
            }
        }
    }

    fn publish_diagnostics(&self, uri: &str) {
        let text = self.documents.get(uri).map(String::as_str).unwrap_or("");
        let diagnostics = self
            .service
            .diagnostics(uri)
            .unwrap_or_default()
            .into_iter()
            .map(|diagnostic| {
                // `crate::lang` reports 1-based character columns; LSP ranges
                // are 0-based and counted in UTF-16 code units.
                let (line, start) = diagnostic_position(text, diagnostic.line, diagnostic.col);
                let (_, end) = diagnostic_position(text, diagnostic.line, diagnostic.col + 1);
                json!({
                    "range": {
                        "start": { "line": line, "character": start },
                        "end": { "line": line, "character": end }
                    },
                    "severity": match diagnostic.severity {
                        DiagnosticSeverity::Error => 1,
                        DiagnosticSeverity::Warning => 2,
                        DiagnosticSeverity::Hint => 4,
                    },
                    "source": "napi-vm",
                    "message": diagnostic.message,
                })
            })
            .collect::<Vec<_>>();
        notify(
            "textDocument/publishDiagnostics",
            json!({ "uri": uri, "diagnostics": diagnostics }),
        );
    }

    fn open_document(&mut self, uri: String, text: String) {
        self.documents.insert(uri.clone(), text.clone());
        self.service.open(uri.clone(), &text);
        self.publish_diagnostics(&uri);
    }

    fn update_document(&mut self, uri: String, text: String) {
        self.documents.insert(uri.clone(), text.clone());
        if !self.service.update(&uri, &text) {
            self.service.open(uri.clone(), &text);
        }
        self.publish_diagnostics(&uri);
    }

    fn handle(&mut self, message: Value) -> bool {
        let id = message.get("id").cloned();
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let params = message.get("params").cloned().unwrap_or(json!({}));

        match method {
            "initialize" => {
                if let Some(workspace) =
                    params.get("rootUri").and_then(Value::as_str).or_else(|| {
                        params
                            .pointer("/workspaceFolders/0/uri")
                            .and_then(Value::as_str)
                    })
                {
                    self.root = uri_path(workspace);
                }
                self.static_manifest = self.load_manifest();
                self.rebuild_service();
                self.connect_runtime();
                if let Some(id) = id {
                    response(
                        id,
                        json!({
                            "capabilities": {
                                // UTF-16 is the LSP default; state it so the
                                // negotiated encoding is never ambiguous.
                                "positionEncoding": "utf-16",
                                "textDocumentSync": 1,
                                "hoverProvider": true,
                                "completionProvider": { "triggerCharacters": [".", "_"] },
                                "documentSymbolProvider": false
                            },
                            "serverInfo": {
                                "name": SERVER_NAME,
                                "version": SERVER_VERSION
                            }
                        }),
                    );
                }
            }
            "initialized" => {}
            "shutdown" => {
                self.shutdown = true;
                if let Some(client) = self.runtime_client.as_mut() {
                    client.stop();
                }
                if let Some(id) = id {
                    response(id, Value::Null);
                }
            }
            // `exit` always stops the event loop. The caller turns the
            // recorded `shutdown` flag into the exit code (0 if `shutdown`
            // was received first, 1 otherwise).
            "exit" => return false,
            "textDocument/didOpen" => {
                if let (Some(uri), Some(text)) = (
                    params.pointer("/textDocument/uri").and_then(Value::as_str),
                    params.pointer("/textDocument/text").and_then(Value::as_str),
                ) {
                    self.open_document(uri.to_string(), text.to_string());
                }
            }
            "textDocument/didChange" => {
                let uri = params
                    .pointer("/textDocument/uri")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let text = params
                    .get("contentChanges")
                    .and_then(Value::as_array)
                    .and_then(|changes| changes.last())
                    .and_then(|change| change.get("text"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if let (Some(uri), Some(text)) = (uri, text) {
                    self.update_document(uri, text);
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = params.pointer("/textDocument/uri").and_then(Value::as_str) {
                    self.documents.remove(uri);
                    self.service.close(uri);
                    notify(
                        "textDocument/publishDiagnostics",
                        json!({ "uri": uri, "diagnostics": [] }),
                    );
                }
            }
            "textDocument/completion" => {
                let uri = params
                    .pointer("/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let text = self.documents.get(uri).cloned().unwrap_or_default();
                let offset = text_offset(&text, params.get("position"));
                let context = self.service.context();
                let items = self
                    .service
                    .complete(uri, offset, &context)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|item| {
                        json!({
                            "label": item.label,
                            "kind": lsp_completion_kind(item.kind),
                            "detail": item.detail,
                        })
                    })
                    .collect::<Vec<_>>();
                if let Some(id) = id {
                    response(id, json!({ "isIncomplete": false, "items": items }));
                }
            }
            "textDocument/hover" => {
                let uri = params
                    .pointer("/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let text = self.documents.get(uri).cloned().unwrap_or_default();
                let offset = text_offset(&text, params.get("position"));
                let result = self.service.hover(uri, offset).map(|info| {
                    let value = match &info.documentation {
                        Some(docs) => format!("{}\n\n{docs}", info.detail),
                        None => info.detail,
                    };
                    json!({ "contents": { "kind": "markdown", "value": value } })
                });
                if let Some(id) = id {
                    response(id, result.unwrap_or(Value::Null));
                }
            }
            "textDocument/documentSymbol" => {
                if let Some(id) = id {
                    response(id, json!([]));
                }
            }
            _ => {
                if let Some(id) = id {
                    response(id, Value::Null);
                }
            }
        }
        true
    }
}

fn register_host(target: &mut LanguageService, host: &Value) {
    let Some(name) = host.get("name").and_then(Value::as_str) else {
        return;
    };
    let params = host
        .get("params")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|param| {
            Some(HostFunctionParameter {
                name: param.get("name")?.as_str()?.to_string(),
                type_name: param
                    .get("typeName")
                    .or_else(|| param.get("type_name"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            })
        })
        .collect();
    target.register_host_function(HostFunctionInfo {
        name: name.to_string(),
        params,
        return_type: host
            .get("returns")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        documentation: host
            .get("documentation")
            .and_then(Value::as_str)
            .map(str::to_string),
        async_fn: host.get("async").and_then(Value::as_bool).unwrap_or(false),
    });
}

fn lsp_completion_kind(kind: CompletionKind) -> u32 {
    match kind {
        CompletionKind::Function => 3,
        CompletionKind::Method => 2,
        CompletionKind::Variable => 6,
        CompletionKind::Property => 10,
        CompletionKind::Class => 7,
        CompletionKind::Module => 9,
        CompletionKind::Keyword => 14,
        CompletionKind::Global => 6,
        CompletionKind::ExposedFn => 3,
    }
}

/// Map an LSP `Position` (0-based line, UTF-16 code-unit character) onto the
/// UTF-8 byte offset that [`LanguageService`] expects.
fn text_offset(text: &str, position: Option<&Value>) -> usize {
    let Some(position) = position else {
        return 0;
    };
    let line = position.get("line").and_then(Value::as_u64).unwrap_or(0) as usize;
    let character = position
        .get("character")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    position_to_offset(text, line, character)
}

fn send(message: Value) {
    let body = message.to_string();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(header.as_bytes());
    let _ = stdout.write_all(body.as_bytes());
    let _ = stdout.flush();
}

fn response(id: Value, result: Value) {
    send(json!({ "jsonrpc": "2.0", "id": id, "result": result }));
}

fn notify(method: &str, params: Value) {
    send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
}

enum Incoming {
    Message(Value),
    Eof,
}

pub fn run() -> i32 {
    let mut server = Server::new();
    let (tx, rx) = mpsc::channel::<Incoming>();
    let reader_tx = tx.clone();
    std::thread::spawn(move || {
        if let Err(error) = read_stdio(reader_tx.clone()) {
            let _ = writeln!(io::stderr(), "[napi-vm-lsp] {error}");
        }
        let _ = reader_tx.send(Incoming::Eof);
    });

    loop {
        // Collect pending runtime events into a temporary Vec so that the
        // immutable borrow on `server.runtime_rx` is released before we call
        // `server.handle_runtime()`, which requires `&mut self`.
        let runtime_events: Vec<RuntimeEvent> = server
            .runtime_rx
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        for event in runtime_events {
            server.handle_runtime(event);
        }

        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Incoming::Message(message)) => {
                if !server.handle(message) {
                    return if server.shutdown { 0 } else { 1 };
                }
            }
            Ok(Incoming::Eof) => return if server.shutdown { 0 } else { 1 },
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return if server.shutdown { 0 } else { 1 },
        }

        let _ = tx;
    }
}

fn read_stdio(tx: mpsc::Sender<Incoming>) -> io::Result<()> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut leftover = Vec::new();
    loop {
        match read_message(&mut reader, &mut leftover)? {
            Some(message) => {
                if tx.send(Incoming::Message(message)).is_err() {
                    return Ok(());
                }
            }
            None => return Ok(()),
        }
    }
}

fn read_message<R: BufRead>(reader: &mut R, leftover: &mut Vec<u8>) -> io::Result<Option<Value>> {
    loop {
        if let Some(separator) = find_separator(leftover) {
            let header = String::from_utf8_lossy(&leftover[..separator]);
            let Some(length) = header.lines().find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|value| value.trim().parse::<usize>().ok())
            }) else {
                leftover.drain(..separator + 4);
                continue;
            };
            let start = separator + 4;
            if leftover.len() >= start + length {
                let body = leftover[start..start + length].to_vec();
                leftover.drain(..start + length);
                return Ok(Some(serde_json::from_slice(&body)?));
            }
        }

        let mut chunk = [0u8; 4096];
        match reader.read(&mut chunk)? {
            0 => return Ok(None),
            n => leftover.extend_from_slice(&chunk[..n]),
        }
    }
}

fn find_separator(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}
