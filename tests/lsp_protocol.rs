//! Protocol-level tests for the shipped `napi-vm-lsp` binary.
//!
//! Unit tests over `LanguageService` cannot cover the transport, which is now
//! part of the executable: header framing, batched and split writes, UTF-16
//! positions, and the `shutdown`/`exit` exit codes. These tests drive the real
//! process over stdio.

mod common;

use serde_json::{Value, json};

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
fn ipc_facade_completion_is_available_without_a_runtime_snapshot() {
    let uri = "file:///tmp/napi-vm-ipc.js";
    let mut client = Client::start(&temp_root());
    client.open(uri, "ipc.");

    let items = client.completion(14, uri, 0, 4);
    for expected in ["invoke", "invokeAsync", "send", "commands"] {
        assert!(
            items.iter().any(|label| label == expected),
            "missing {expected} in IPC completion: {items:?}"
        );
    }

    assert_eq!(client.shutdown_and_exit(15), 0);
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
