//! Runtime Protocol v1 lifecycle against the real `napi-vm-lsp` process.
//!
//! Covers the failure modes that only show up over a live socket: an idle
//! connection must stay connected (a read timeout is not a disconnect), a
//! replaced session must take over cleanly, and a vanished runtime must clear
//! the metadata again.

#![cfg(unix)]

mod common;

use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use common::Client;

const URI: &str = "file:///tmp/napi-vm-runtime-test.js";
/// Long enough to cross several socket read timeouts (250 ms each).
const IDLE: Duration = Duration::from_millis(1_500);

struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn create(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("napi-vm-lsp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".napi-vm")).expect("create workspace");
        Self { root }
    }

    fn workspace_id(&self) -> String {
        // Must match `crate::lsp::workspace_id`: sha256 of the canonical path,
        // first 20 hex characters.
        let resolved = std::fs::canonicalize(&self.root).expect("canonicalize");
        let digest = Sha256::digest(resolved.to_string_lossy().as_bytes());
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            .chars()
            .take(20)
            .collect()
    }

    /// Bind a session socket and publish its locator.
    fn publish(&self, session_id: &str) -> UnixListener {
        let socket = self.root.join(format!("{session_id}.sock"));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind unix socket");
        let locator = json!({
            "protocolVersion": 1,
            "workspaceId": self.workspace_id(),
            "sessionId": session_id,
            "pid": std::process::id(),
            "authToken": format!("{session_id}-token"),
            "transport": { "kind": "unix", "address": socket.to_string_lossy() }
        });
        write_atomic(&self.root.join(".napi-vm").join("runtime.json"), &locator);
        listener
    }

    fn unpublish(&self) {
        let _ = std::fs::remove_file(self.root.join(".napi-vm").join("runtime.json"));
    }

    fn uri(&self) -> String {
        format!("file://{}", self.root.to_string_lossy())
    }

    /// The same root as a percent-encoded URI, the way an editor sends it for
    /// a path containing spaces or non-ASCII characters.
    fn encoded_uri(&self) -> String {
        let mut uri = String::from("file://");
        for byte in self.root.to_string_lossy().bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                    uri.push(byte as char)
                }
                _ => uri.push_str(&format!("%{byte:02X}")),
            }
        }
        uri
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn write_atomic(path: &Path, value: &Value) {
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, value.to_string()).expect("write locator");
    std::fs::rename(&temp, path).expect("publish locator");
}

/// Accept the language server's connection, failing rather than hanging.
fn accept(listener: &UnixListener) -> UnixStream {
    listener
        .set_nonblocking(true)
        .expect("set listener nonblocking");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).expect("blocking stream");
                return stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "language server never connected");
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(error) => panic!("accept failed: {error}"),
        }
    }
}

fn send_snapshot(stream: &mut UnixStream, functions: &[&str]) {
    let payload = json!({
        "functions": functions
            .iter()
            .map(|name| json!({
                "name": name,
                "params": [],
                "returns": "void",
                "documentation": format!("runtime host function {name}")
            }))
            .collect::<Vec<_>>()
    });
    let message = json!({ "type": "snapshot", "payload": payload });
    writeln!(stream, "{message}").expect("write snapshot");
    stream.flush().expect("flush snapshot");
}

fn send_global_snapshot(stream: &mut UnixStream) {
    let payload = json!({
        "globals": [{
            "name": "custom",
            "shape": {
                "kind": "object",
                "properties": {
                    "ping": {
                        "kind": "function",
                        "params": [],
                        "returns": { "kind": "string" },
                        "documentation": "Pings the host."
                    },
                    "loadAsync": {
                        "kind": "function",
                        "params": [],
                        "returns": { "kind": "unknown" },
                        "async": true
                    }
                }
            }
        }]
    });
    let message = json!({ "type": "snapshot", "payload": payload });
    writeln!(stream, "{message}").expect("write global snapshot");
    stream.flush().expect("flush global snapshot");
}

/// Poll completion until `predicate` holds, or fail after `timeout`.
fn wait_until(
    client: &mut Client,
    id: &mut i64,
    timeout: Duration,
    label: &str,
    predicate: impl Fn(&[String]) -> bool,
) -> Vec<String> {
    let deadline = Instant::now() + timeout;
    let mut last;
    loop {
        *id += 1;
        last = client.completion(*id, URI, 0, 8);
        if predicate(&last) {
            return last;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {label}; last completion was {last:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn runtime_metadata_survives_idle_and_session_changes() {
    let workspace = Workspace::create("runtime");
    let listener = workspace.publish("session-one");

    let mut client = Client::start_at_uri(&workspace.uri());
    client.open(URI, "napiTest");
    let _ = client.wait_for_diagnostics(URI);

    let mut stream = accept(&listener);
    let mut id = 100;

    // 1. The first snapshot reaches completion.
    send_snapshot(&mut stream, &["napiTestAlpha"]);
    wait_until(
        &mut client,
        &mut id,
        Duration::from_secs(10),
        "the first runtime snapshot",
        |items| items.iter().any(|label| label == "napiTestAlpha"),
    );

    // 2. Leaving the socket idle must not drop the connection…
    std::thread::sleep(IDLE);
    let items = wait_until(
        &mut client,
        &mut id,
        Duration::from_secs(2),
        "runtime metadata to survive an idle socket",
        |items| items.iter().any(|label| label == "napiTestAlpha"),
    );
    assert!(items.iter().any(|label| label == "napiTestAlpha"));

    // 3. …and an update on that same, still-open connection must arrive.
    send_snapshot(&mut stream, &["napiTestAlpha", "napiTestBeta"]);
    wait_until(
        &mut client,
        &mut id,
        Duration::from_secs(10),
        "an update after an idle period",
        |items| items.iter().any(|label| label == "napiTestBeta"),
    );

    // 4. A replaced session takes over: the old socket closes, a new locator
    //    is published, and only the new session's data is served.
    drop(stream);
    drop(listener);
    let listener = workspace.publish("session-two");
    let mut stream = accept(&listener);
    send_snapshot(&mut stream, &["napiTestGamma"]);
    wait_until(
        &mut client,
        &mut id,
        Duration::from_secs(15),
        "the replacement session's snapshot",
        |items| {
            items.iter().any(|label| label == "napiTestGamma")
                && !items.iter().any(|label| label == "napiTestBeta")
        },
    );

    // 5. Removing the locator clears runtime metadata again.
    workspace.unpublish();
    drop(stream);
    drop(listener);
    wait_until(
        &mut client,
        &mut id,
        Duration::from_secs(10),
        "runtime metadata to be cleared",
        |items| !items.iter().any(|label| label.starts_with("napiTest")),
    );

    assert_eq!(client.shutdown_and_exit(id + 1), 0);
}

#[test]
fn runtime_global_completion_and_hover_use_declarative_shapes() {
    let workspace = Workspace::create("runtime-global");
    let listener = workspace.publish("session-global");
    let mut client = Client::start_at_uri(&workspace.uri());
    let source = "custom.ping; custom.loadAsync;";
    client.open(URI, source);
    let _ = client.wait_for_diagnostics(URI);
    let mut stream = accept(&listener);

    send_global_snapshot(&mut stream);
    let deadline = Instant::now() + Duration::from_secs(10);
    let items = loop {
        let items = client.completion(300, URI, 0, 7);
        if items.iter().any(|label| label == "ping") {
            break items;
        }
        assert!(
            Instant::now() < deadline,
            "global completion never arrived: {items:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(items.iter().any(|label| label == "loadAsync"));

    let hover = client.hover(301, URI, 0, 9);
    let value = hover
        .pointer("/contents/value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(value.contains("() => string"), "hover: {value}");
    assert!(
        value.contains("Pings the host."),
        "hover documentation: {value}"
    );

    let async_hover = client.hover(303, URI, 0, source.find("loadAsync").unwrap() + 2);
    let async_value = async_hover
        .pointer("/contents/value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        async_value.contains("() => Promise<unknown>"),
        "async hover: {async_value}"
    );

    assert_eq!(client.shutdown_and_exit(302), 0);
}

#[test]
fn percent_encoded_workspace_root_still_finds_the_runtime() {
    // The root contains a space and an accent, so the server only discovers
    // `.napi-vm/runtime.json` if it decodes the `file://` URI properly — the
    // workspace id is a hash of the *canonical* path.
    let workspace = Workspace::create("My Próject");
    let listener = workspace.publish("session-encoded");

    let mut client = Client::start_at_uri(&workspace.encoded_uri());
    client.open(URI, "napiTest");
    let _ = client.wait_for_diagnostics(URI);

    let mut stream = accept(&listener);
    let mut id = 200;
    send_snapshot(&mut stream, &["napiTestEncoded"]);
    wait_until(
        &mut client,
        &mut id,
        Duration::from_secs(10),
        "a snapshot from a percent-encoded workspace root",
        |items| items.iter().any(|label| label == "napiTestEncoded"),
    );

    assert_eq!(client.shutdown_and_exit(id + 1), 0);
}
