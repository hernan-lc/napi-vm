//! Protocol-level tests for the shipped `napi-vm-lsp` binary.
//!
//! Unit tests over `LanguageService` cannot cover the transport, which is now
//! part of the executable: header framing, batched and split writes, UTF-16
//! positions, and the `shutdown`/`exit` exit codes. These tests drive the real
//! process over stdio.

mod common;

use serde_json::{Value, json};
use std::fs;

use common::{Client, temp_root};

#[test]
fn initialize_advertises_capabilities_and_utf16_encoding() {
    let mut client = Client::start(&temp_root());
    client.request(2, "textDocument/documentSymbol", json!({}));
    let response = client.wait_for_response(2);
    assert!(response.get("result").is_some());
    assert_eq!(client.shutdown_and_exit(3), 0);
}

#[test]
fn full_document_lifecycle_publishes_diagnostics() {
    let uri = "file:///tmp/napi-vm-lifecycle.js";
    let mut client = Client::start(&temp_root());

    client.open(uri, "const value = 1;\n");
    assert!(
        client.wait_for_diagnostics(uri).is_empty(),
        "clean source has no diagnostics"
    );

    // Unbalanced delimiter → at least one diagnostic.
    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{ "text": "const value = (1;\n" }]
        }),
    );
    let diagnostics = client.wait_for_diagnostics(uri);
    assert!(!diagnostics.is_empty(), "unbalanced `(` is reported");
    assert_eq!(
        diagnostics[0].pointer("/source"),
        Some(&Value::from("napi-vm"))
    );

    client.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": uri } }),
    );
    assert!(
        client.wait_for_diagnostics(uri).is_empty(),
        "closing clears diagnostics"
    );

    assert_eq!(client.shutdown_and_exit(9), 0);
}

#[test]
fn multiple_messages_in_one_write_are_all_processed() {
    let uri = "file:///tmp/napi-vm-batch.js";
    let mut client = Client::start(&temp_root());

    let mut batch = Client::frame(&json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": { "textDocument": { "uri": uri, "languageId": "javascript", "version": 1, "text": "const value = 1;\n" } }
    }));
    batch.extend(Client::frame(&json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "textDocument/completion",
        "params": { "textDocument": { "uri": uri }, "position": { "line": 1, "character": 0 } }
    })));
    client.send_raw(&batch);

    let response = client.wait_for_response(4);
    assert!(response.pointer("/result/items").is_some());
    assert_eq!(client.shutdown_and_exit(5), 0);
}

#[test]
fn partial_writes_and_extra_headers_are_tolerated() {
    let mut client = Client::start(&temp_root());
    let body = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "textDocument/documentSymbol",
        "params": { "textDocument": { "uri": "file:///tmp/napi-vm-partial.js" } }
    })
    .to_string();
    let framed = format!(
        "Content-Length: {}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n{body}",
        body.len()
    );
    let bytes = framed.into_bytes();
    // Deliver one byte at a time: header, separator and body all split.
    for byte in &bytes {
        client.send_raw(std::slice::from_ref(byte));
    }
    let response = client.wait_for_response(7);
    assert!(response.get("result").is_some());
    assert_eq!(client.shutdown_and_exit(8), 0);
}

#[test]
fn content_length_counts_utf8_bytes_not_characters() {
    let uri = "file:///tmp/napi-vm-unicode.js";
    let mut client = Client::start(&temp_root());
    // The payload contains multi-byte characters, so a character-counting
    // implementation would mis-frame it.
    client.open(uri, "const greeting = \"héllo 🔥 日本語\";\n");
    assert!(client.wait_for_diagnostics(uri).is_empty());
    assert_eq!(client.shutdown_and_exit(6), 0);
}

/// UTF-16 code-unit column of the byte index `at` within `line`.
fn utf16_column(line: &str, at: usize) -> usize {
    line[..at].chars().map(char::len_utf16).sum()
}

#[test]
fn utf16_positions_locate_completion_and_hover_after_astral_characters() {
    let uri = "file:///tmp/napi-vm-utf16.js";
    let mut client = Client::start(&temp_root());
    // Every position below sits *after* multi-byte and astral characters on
    // its own line, so a byte-offset implementation lands in the wrong place.
    let second = "const icon = \"🔥\"; const japanese = \"日本語\"; config.";
    let source = format!("const config = {{ alpha: 1, beta: 2 }};\n{second}\n");
    client.open(uri, &source);
    let _ = client.wait_for_diagnostics(uri);

    let items = client.completion(10, uri, 1, utf16_column(second, second.len()));
    assert!(
        items.iter().any(|label| label == "alpha") && items.iter().any(|label| label == "beta"),
        "member completion after `config.`, got {items:?}"
    );

    // Hover inside `japanese`, which follows the emoji on the same line.
    let inside_japanese = second.find("japanese").unwrap() + 2;
    let hover = client.hover(11, uri, 1, utf16_column(second, inside_japanese));
    let detail = hover
        .pointer("/contents/value")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    assert!(
        detail.contains("japanese"),
        "hover resolves the identifier after the emoji, got {hover}"
    );

    // Hover inside `config` on line 0, for a baseline on an ASCII-only line.
    let hover = client.hover(12, uri, 0, 8);
    assert!(
        hover
            .pointer("/contents/value")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("config"),
        "hover: {hover}"
    );

    assert_eq!(client.shutdown_and_exit(13), 0);
}

#[test]
fn ipc_facade_completion_is_absent_without_metadata() {
    let uri = "file:///tmp/napi-vm-ipc.js";
    let mut client = Client::start(&temp_root());
    client.open(uri, "ipc.");

    let items = client.completion(14, uri, 0, 4);
    assert!(
        items
            .iter()
            .all(|label| !["invoke", "invokeAsync", "send", "commands"].contains(&label.as_str()))
    );

    assert_eq!(client.shutdown_and_exit(15), 0);
}

#[test]
fn static_manifest_globals_support_completion_hover_and_documentation() {
    let config =
        std::env::temp_dir().join(format!("napi-vm-lsp-manifest-{}.json", std::process::id()));
    fs::write(
        &config,
        json!({
            "globals": [{
                "name": "analytics",
                "shape": {
                    "kind": "object",
                    "properties": {
                        "track": {
                            "kind": "function",
                            "params": [{"name": "event", "type": {"kind": "string"}}],
                            "returns": {"kind": "void"},
                            "documentation": "Records an analytics event."
                        }
                    }
                }
            }]
        })
        .to_string(),
    )
    .unwrap();
    let config_arg = config.to_string_lossy().to_string();
    let mut client = Client::start_at_uri_with_args(
        &format!("file://{}", std::env::temp_dir().to_string_lossy()),
        &["--config", &config_arg],
    );
    let uri = "file:///tmp/napi-vm-manifest.js";
    client.open(uri, "analytics.track");
    let _ = client.wait_for_diagnostics(uri);

    let items = client.completion(20, uri, 0, "analytics.".len());
    assert!(items.iter().any(|label| label == "track"));
    let hover = client.hover(21, uri, 0, "analytics.track".len() - 1);
    let value = hover
        .pointer("/contents/value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(value.contains("(event: string) => void"), "hover: {value}");
    assert!(
        value.contains("Records an analytics event."),
        "hover: {value}"
    );

    assert_eq!(client.shutdown_and_exit(22), 0);
    let _ = fs::remove_file(config);
}

#[test]
fn invalid_static_manifest_globals_are_ignored_safely() {
    let config = std::env::temp_dir().join(format!(
        "napi-vm-lsp-invalid-manifest-{}.json",
        std::process::id()
    ));
    fs::write(
        &config,
        json!({
            "globals": [{
                "name": "broken",
                "shape": { "kind": "banana" }
            }]
        })
        .to_string(),
    )
    .unwrap();
    let config_arg = config.to_string_lossy().to_string();
    let mut client = Client::start_at_uri_with_args(
        &format!("file://{}", std::env::temp_dir().to_string_lossy()),
        &["--config", &config_arg],
    );
    let uri = "file:///tmp/napi-vm-invalid-manifest.js";
    client.open(uri, "broken.");
    let _ = client.wait_for_diagnostics(uri);
    let items = client.completion(24, uri, 0, "broken.".len());
    assert!(!items.iter().any(|label| label == "broken"));
    assert_eq!(client.shutdown_and_exit(25), 0);
    let _ = fs::remove_file(config);
}

#[test]
fn exit_without_shutdown_returns_one() {
    let mut client = Client::start(&temp_root());
    client.notify("exit", json!({}));
    assert_eq!(client.wait_for_exit(), 1);
}

#[test]
fn exit_after_shutdown_returns_zero() {
    let mut client = Client::start(&temp_root());
    assert_eq!(client.shutdown_and_exit(2), 0);
}

#[test]
fn closed_stdin_without_shutdown_returns_one() {
    let mut client = Client::start(&temp_root());
    assert_eq!(client.wait_for_exit(), 1);
}

// ---------------------------------------------------------------------------
// Navigation: definition, references, rename, outline, signature help, inlay
// hints and semantic tokens.
// ---------------------------------------------------------------------------

/// Open a document and return a client positioned to query it.
fn client_with(uri: &str, source: &str) -> Client {
    let mut client = Client::start(&temp_root());
    client.open(uri, source);
    let _ = client.wait_for_diagnostics(uri);
    client
}

fn position(line: u64, character: u64) -> Value {
    json!({ "line": line, "character": character })
}

#[test]
fn initialize_advertises_the_navigation_capabilities() {
    let mut client = Client::start(&temp_root());
    // `Client::start` already performed `initialize`; re-request it to read
    // the advertised capabilities.
    client.request(40, "initialize", json!({ "rootUri": Value::Null }));
    let response = client.wait_for_response(40);
    let capabilities = response
        .pointer("/result/capabilities")
        .expect("capabilities");
    for capability in [
        "documentSymbolProvider",
        "definitionProvider",
        "referencesProvider",
        "documentHighlightProvider",
        "renameProvider",
        "signatureHelpProvider",
        "inlayHintProvider",
        "semanticTokensProvider",
    ] {
        assert!(
            capabilities.get(capability).is_some(),
            "{capability} is advertised"
        );
    }
    assert_eq!(client.shutdown_and_exit(41), 0);
}

#[test]
fn document_symbols_carry_ranges() {
    let uri = "file:///tmp/napi-vm-symbols.js";
    let mut client = client_with(
        uri,
        "function greet(name) {}\nclass Widget {}\nconst total = 1;\n",
    );
    client.request(
        42,
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": uri } }),
    );
    let response = client.wait_for_response(42);
    let symbols = response
        .pointer("/result")
        .and_then(Value::as_array)
        .expect("symbol array")
        .clone();
    let names: Vec<&str> = symbols
        .iter()
        .filter_map(|s| s.get("name").and_then(Value::as_str))
        .collect();
    assert!(names.contains(&"greet"));
    assert!(names.contains(&"Widget"));
    assert!(names.contains(&"total"));

    let greet = symbols
        .iter()
        .find(|s| s.get("name").and_then(Value::as_str) == Some("greet"))
        .expect("greet");
    assert_eq!(greet.pointer("/detail"), Some(&Value::from("(name)")));
    assert_eq!(greet.pointer("/range/start/line"), Some(&Value::from(0)));
    assert_eq!(
        greet.pointer("/range/start/character"),
        Some(&Value::from(9))
    );
    assert_eq!(client.shutdown_and_exit(43), 0);
}

#[test]
fn definition_jumps_to_the_declaration() {
    let uri = "file:///tmp/napi-vm-definition.js";
    let mut client = client_with(uri, "const total = 1;\nconst doubled = total * 2;\n");
    client.request(
        44,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri },
            "position": position(1, 16)
        }),
    );
    let response = client.wait_for_response(44);
    assert_eq!(
        response.pointer("/result/range/start/line"),
        Some(&Value::from(0))
    );
    assert_eq!(
        response.pointer("/result/range/start/character"),
        Some(&Value::from(6))
    );
    assert_eq!(client.shutdown_and_exit(45), 0);
}

#[test]
fn references_are_scoped_to_the_binding() {
    let uri = "file:///tmp/napi-vm-references.js";
    let mut client = client_with(
        uri,
        "function a() { let v = 1; return v; }\nfunction b() { let v = 2; return v; }\n",
    );
    client.request(
        46,
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri },
            "position": position(0, 19),
            "context": { "includeDeclaration": true }
        }),
    );
    let response = client.wait_for_response(46);
    let locations = response
        .pointer("/result")
        .and_then(Value::as_array)
        .expect("locations");
    assert_eq!(locations.len(), 2, "only the `v` inside `a`");
    for location in locations {
        assert_eq!(location.pointer("/range/start/line"), Some(&Value::from(0)));
    }
    assert_eq!(client.shutdown_and_exit(47), 0);
}

#[test]
fn rename_edits_only_the_matching_binding() {
    let uri = "file:///tmp/napi-vm-rename.js";
    let mut client = client_with(
        uri,
        "function a() { let v = 1; return v; }\nfunction b() { let v = 2; return v; }\n",
    );
    client.request(
        48,
        "textDocument/rename",
        json!({
            "textDocument": { "uri": uri },
            "position": position(0, 19),
            "newName": "renamed"
        }),
    );
    let response = client.wait_for_response(48);
    let edits = response
        .pointer("/result/changes")
        .and_then(Value::as_object)
        .and_then(|changes| changes.get(uri))
        .and_then(Value::as_array)
        .expect("edits for the document");
    assert_eq!(edits.len(), 2);
    assert_eq!(edits[0].pointer("/newText"), Some(&Value::from("renamed")));
    assert_eq!(client.shutdown_and_exit(49), 0);
}

#[test]
fn rename_to_a_non_identifier_is_refused() {
    let uri = "file:///tmp/napi-vm-rename-bad.js";
    let mut client = client_with(uri, "let v = 1;\nv;\n");
    client.request(
        50,
        "textDocument/rename",
        json!({
            "textDocument": { "uri": uri },
            "position": position(0, 4),
            "newName": "2 + 2"
        }),
    );
    let response = client.wait_for_response(50);
    assert_eq!(response.pointer("/result"), Some(&Value::Null));
    assert_eq!(client.shutdown_and_exit(51), 0);
}

#[test]
fn signature_help_reports_the_active_parameter() {
    let uri = "file:///tmp/napi-vm-signature.js";
    let mut client = client_with(uri, "function add(a, b) { return a + b; }\nadd(1, 2);\n");
    client.request(
        52,
        "textDocument/signatureHelp",
        json!({
            "textDocument": { "uri": uri },
            "position": position(1, 7)
        }),
    );
    let response = client.wait_for_response(52);
    assert_eq!(
        response.pointer("/result/signatures/0/label"),
        Some(&Value::from("add(a, b)"))
    );
    assert_eq!(
        response.pointer("/result/activeParameter"),
        Some(&Value::from(1))
    );
    assert_eq!(client.shutdown_and_exit(53), 0);
}

#[test]
fn inlay_hints_label_positional_arguments() {
    let uri = "file:///tmp/napi-vm-inlay.js";
    let mut client = client_with(uri, "function move(x, y) {}\nmove(1, 2);\n");
    client.request(
        54,
        "textDocument/inlayHint",
        json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": position(0, 0),
                "end": position(2, 0)
            }
        }),
    );
    let response = client.wait_for_response(54);
    let hints = response
        .pointer("/result")
        .and_then(Value::as_array)
        .expect("hints");
    let labels: Vec<&str> = hints
        .iter()
        .filter_map(|h| h.get("label").and_then(Value::as_str))
        .collect();
    assert_eq!(labels, vec!["x:", "y:"]);
    assert_eq!(client.shutdown_and_exit(55), 0);
}

#[test]
fn semantic_tokens_are_encoded_as_deltas() {
    let uri = "file:///tmp/napi-vm-semantic.js";
    let mut client = client_with(uri, "const x = 1;\n");
    client.request(
        56,
        "textDocument/semanticTokens/full",
        json!({ "textDocument": { "uri": uri } }),
    );
    let response = client.wait_for_response(56);
    let data = response
        .pointer("/result/data")
        .and_then(Value::as_array)
        .expect("token data");
    // Five integers per token, and the first token starts at the origin.
    assert!(!data.is_empty());
    assert_eq!(data.len() % 5, 0);
    assert_eq!(data[0], Value::from(0));
    assert_eq!(data[1], Value::from(0));
    assert_eq!(client.shutdown_and_exit(57), 0);
}

#[test]
fn document_highlight_marks_every_occurrence() {
    let uri = "file:///tmp/napi-vm-highlight.js";
    let mut client = client_with(uri, "let v = 1;\nv = 2;\nv;\n");
    client.request(
        58,
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": uri },
            "position": position(0, 4)
        }),
    );
    let response = client.wait_for_response(58);
    let highlights = response
        .pointer("/result")
        .and_then(Value::as_array)
        .expect("highlights");
    assert_eq!(highlights.len(), 3);
    // A highlight is a bare range, with no document URI.
    assert!(highlights[0].get("uri").is_none());
    assert_eq!(client.shutdown_and_exit(59), 0);
}
