//! Shared harness for the protocol-level `napi-vm-lsp` tests: framing,
//! request/response correlation and process lifecycle.

#![allow(dead_code)]

use std::io::{BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const BINARY: &str = env!("CARGO_BIN_EXE_napi-vm-lsp");

pub struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    buffer: Vec<u8>,
}

impl Client {
    /// Start the server with a workspace *path* as its root.
    pub fn start(root: &str) -> Self {
        Self::start_at_uri(&format!("file://{root}"))
    }

    /// Start the server with a workspace *URI* as its root, for callers that
    /// need percent-encoding or a non-trivial path.
    pub fn start_at_uri(root_uri: &str) -> Self {
        let mut child = Command::new(BINARY)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn napi-vm-lsp");
        let stdin = Some(child.stdin.take().expect("stdin"));
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let mut client = Self {
            child,
            stdin,
            stdout,
            buffer: Vec::new(),
        };
        client.request(
            1,
            "initialize",
            json!({ "rootUri": root_uri, "capabilities": {} }),
        );
        let response = client.wait_for_response(1);
        assert_eq!(
            response.pointer("/result/serverInfo/name"),
            Some(&Value::from("napi-vm-lsp"))
        );
        client.notify("initialized", json!({}));
        client
    }

    pub fn send_raw(&mut self, bytes: &[u8]) {
        let stdin = self.stdin.as_mut().expect("stdin is still open");
        stdin.write_all(bytes).expect("write");
        stdin.flush().expect("flush");
    }

    pub fn frame(message: &Value) -> Vec<u8> {
        let body = message.to_string();
        let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        out.extend_from_slice(body.as_bytes());
        out
    }

    pub fn send(&mut self, message: Value) {
        let frame = Self::frame(&message);
        self.send_raw(&frame);
    }

    pub fn request(&mut self, id: i64, method: &str, params: Value) {
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    /// Read one complete message, blocking until its framed body arrives.
    pub fn read_message(&mut self) -> Value {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(separator) = self
                .buffer
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
            {
                let header = String::from_utf8_lossy(&self.buffer[..separator]).to_string();
                let length = header
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .expect("Content-Length header");
                let start = separator + 4;
                if self.buffer.len() >= start + length {
                    let body = self.buffer[start..start + length].to_vec();
                    self.buffer.drain(..start + length);
                    return serde_json::from_slice(&body).expect("valid JSON body");
                }
            }
            assert!(Instant::now() < deadline, "timed out waiting for a message");
            let mut chunk = [0u8; 4096];
            match self.stdout.read(&mut chunk).expect("read") {
                0 => panic!("server closed stdout while a message was expected"),
                n => self.buffer.extend_from_slice(&chunk[..n]),
            }
        }
    }

    pub fn wait_for_response(&mut self, id: i64) -> Value {
        loop {
            let message = self.read_message();
            if message.get("id").and_then(Value::as_i64) == Some(id) {
                return message;
            }
        }
    }

    pub fn wait_for_diagnostics(&mut self, uri: &str) -> Vec<Value> {
        loop {
            let message = self.read_message();
            if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
                && message.pointer("/params/uri").and_then(Value::as_str) == Some(uri)
            {
                return message
                    .pointer("/params/diagnostics")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
            }
        }
    }

    pub fn open(&mut self, uri: &str, text: &str) {
        self.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri, "languageId": "javascript", "version": 1, "text": text } }),
        );
    }

    pub fn completion(&mut self, id: i64, uri: &str, line: usize, character: usize) -> Vec<String> {
        self.request(
            id,
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        );
        self.wait_for_response(id)
            .pointer("/result/items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| {
                item.get("label")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect()
    }

    pub fn hover(&mut self, id: i64, uri: &str, line: usize, character: usize) -> Value {
        self.request(
            id,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        );
        self.wait_for_response(id)
            .get("result")
            .cloned()
            .unwrap_or(Value::Null)
    }

    /// Send `shutdown` then `exit` and return the process exit code.
    pub fn shutdown_and_exit(&mut self, id: i64) -> i32 {
        self.request(id, "shutdown", json!({}));
        let response = self.wait_for_response(id);
        assert_eq!(response.get("result"), Some(&Value::Null));
        self.notify("exit", json!({}));
        self.wait_for_exit()
    }

    pub fn wait_for_exit(&mut self) -> i32 {
        // Closing stdin releases the reader thread; `exit` (if it was sent)
        // has already told the event loop to stop.
        self.stdin.take();
        let status = self.child.wait().expect("wait for exit");
        status.code().expect("exit code")
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn temp_root() -> String {
    std::env::temp_dir().to_string_lossy().to_string()
}
