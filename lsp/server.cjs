#!/usr/bin/env node

// Minimal stdio LSP adapter for Zed. The language intelligence lives in the
// Rust core and is exposed to this process through the napi-vm addon.
const fs = require("node:fs");
const path = require("node:path");
const { fileURLToPath } = require("node:url");
const { LanguageService } = require("../index.js");

const service = new LanguageService();
const documents = new Map();
let root = process.cwd();
let shutdown = false;

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

function loadManifest() {
  const configuredPath = argument("--config");
  const manifestPath = configuredPath
    ? path.resolve(root, configuredPath)
    : path.join(root, ".napi-vm.json");
  if (!fs.existsSync(manifestPath)) return;
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));

  for (const host of manifest.hostFunctions || []) {
    service.registerHostFunction(
      host.name,
      host.params || [],
      host.returns || "unknown",
      host.documentation,
      host.async || false,
    );
  }

  for (const module of manifest.modules || []) {
    const modulePath = path.resolve(root, module.path);
    if (fs.existsSync(modulePath)) {
      service.registerModule(
        module.name,
        fs.readFileSync(modulePath, "utf8"),
      );
    }
  }
}

function uriPath(uri) {
  if (uri.startsWith("file://")) return fileURLToPath(uri);
  return uri;
}

function textOffset(text, position) {
  const lines = text.split(/\r?\n/);
  const line = Math.max(0, Math.min(position.line, lines.length - 1));
  const character = Math.max(0, Math.min(position.character, lines[line].length));
  let codeUnits = character;
  for (let index = 0; index < line; index++) codeUnits += lines[index].length + 1;
  return Buffer.byteLength(text.slice(0, codeUnits), "utf8");
}

function lspCompletionKind(kind) {
  return {
    function: 3,
    method: 2,
    variable: 6,
    property: 10,
    class: 7,
    module: 9,
    keyword: 14,
    global: 6,
    exposed: 3,
  }[kind] || 1;
}

function send(message) {
  const body = JSON.stringify(message);
  process.stdout.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

function response(id, result) {
  send({ jsonrpc: "2.0", id, result });
}

function notify(method, params) {
  send({ jsonrpc: "2.0", method, params });
}

function publishDiagnostics(uri) {
  const diagnostics = service.diagnostics(uri).map((diagnostic) => ({
    range: {
      start: { line: diagnostic.line - 1, character: Math.max(0, diagnostic.col - 1) },
      end: { line: diagnostic.line - 1, character: diagnostic.col },
    },
    severity: diagnostic.severity === "error" ? 1 : diagnostic.severity === "warning" ? 2 : 4,
    source: "napi-vm",
    message: diagnostic.message,
  }));
  notify("textDocument/publishDiagnostics", { uri, diagnostics });
}

function openDocument(uri, text) {
  documents.set(uri, text);
  service.open(uri, text);
  publishDiagnostics(uri);
}

function updateDocument(uri, text) {
  documents.set(uri, text);
  if (!service.update(uri, text)) service.open(uri, text);
  publishDiagnostics(uri);
}

function handle(message) {
  const { id, method, params = {} } = message;
  if (method === "initialize") {
    const workspace = params.rootUri || params.workspaceFolders?.[0]?.uri;
    if (workspace) root = uriPath(workspace);
    loadManifest();
    return response(id, {
      capabilities: {
        textDocumentSync: 1,
        hoverProvider: true,
        completionProvider: { triggerCharacters: [".", "_"] },
        documentSymbolProvider: false,
      },
      serverInfo: { name: "napi-vm-lsp", version: "0.1.0" },
    });
  }
  if (method === "initialized") return;
  if (method === "shutdown") {
    shutdown = true;
    return response(id, null);
  }
  if (method === "exit") process.exit(shutdown ? 0 : 1);

  if (method === "textDocument/didOpen") {
    openDocument(params.textDocument.uri, params.textDocument.text);
    return;
  }
  if (method === "textDocument/didChange") {
    const change = params.contentChanges?.at(-1);
    if (change && typeof change.text === "string") {
      updateDocument(params.textDocument.uri, change.text);
    }
    return;
  }
  if (method === "textDocument/didClose") {
    documents.delete(params.textDocument.uri);
    service.close(params.textDocument.uri);
    notify("textDocument/publishDiagnostics", {
      uri: params.textDocument.uri,
      diagnostics: [],
    });
    return;
  }

  if (method === "textDocument/completion") {
    const uri = params.textDocument.uri;
    const text = documents.get(uri) || "";
    const offset = textOffset(text, params.position || { line: 0, character: 0 });
    const items = service.complete(uri, offset).map((item) => ({
      label: item.label,
      kind: lspCompletionKind(item.kind),
      detail: item.detail || undefined,
    }));
    return response(id, { isIncomplete: false, items });
  }
  if (method === "textDocument/hover") {
    const uri = params.textDocument.uri;
    const text = documents.get(uri) || "";
    const offset = textOffset(text, params.position || { line: 0, character: 0 });
    const info = service.hover(uri, offset);
    if (!info) return response(id, null);
    const value = info.documentation
      ? `${info.detail}\n\n${info.documentation}`
      : info.detail;
    return response(id, { contents: { kind: "markdown", value } });
  }
  if (method === "textDocument/documentSymbol") return response(id, []);
  if (id !== undefined) return response(id, null);
}

let buffer = Buffer.alloc(0);
process.stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const separator = buffer.indexOf("\r\n\r\n");
    if (separator < 0) break;
    const header = buffer.subarray(0, separator).toString("ascii");
    const match = header.match(/Content-Length:\s*(\d+)/i);
    if (!match) {
      buffer = buffer.subarray(separator + 4);
      continue;
    }
    const length = Number(match[1]);
    const start = separator + 4;
    if (buffer.length < start + length) break;
    const body = buffer.subarray(start, start + length).toString("utf8");
    buffer = buffer.subarray(start + length);
    try {
      handle(JSON.parse(body));
    } catch (error) {
      process.stderr.write(`[napi-vm-lsp] ${error.stack || error}\n`);
    }
  }
});
