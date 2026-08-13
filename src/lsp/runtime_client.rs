use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::Value;

use super::protocol::{
    PROTOCOL_VERSION, RuntimeLocator, TransportKind, locator_path, workspace_id,
};

pub enum RuntimeEvent {
    Snapshot(Option<Value>),
    Error(String),
}

pub struct RuntimeClient {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl RuntimeClient {
    pub fn start(root: PathBuf, tx: Sender<RuntimeEvent>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = thread::spawn(move || discover_loop(root, flag, tx));
        Self {
            stop,
            thread: Some(thread),
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for RuntimeClient {
    fn drop(&mut self) {
        self.stop();
    }
}

fn discover_loop(root: PathBuf, stop: Arc<AtomicBool>, tx: Sender<RuntimeEvent>) {
    let mut session_id: Option<String> = None;
    let mut connection: Option<JoinHandle<()>> = None;
    let (close_tx, close_rx) = mpsc::channel::<()>();

    while !stop.load(Ordering::SeqCst) {
        if close_rx.try_recv().is_ok() {
            session_id = None;
            let _ = tx.send(RuntimeEvent::Snapshot(None));
        }

        match read_locator(&root) {
            Ok(locator) => {
                if session_id.as_deref() == Some(locator.session_id.as_str()) {
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }
                session_id = Some(locator.session_id.clone());
                let event_tx = tx.clone();
                let done = close_tx.clone();
                let conn_stop = stop.clone();
                connection = Some(thread::spawn(move || {
                    if let Err(error) = read_transport(&locator, conn_stop, &event_tx) {
                        let _ = event_tx.send(RuntimeEvent::Error(error));
                    }
                    let _ = done.send(());
                }));
            }
            Err(_) => {
                if session_id.take().is_some() {
                    let _ = tx.send(RuntimeEvent::Snapshot(None));
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }

    drop(connection);
}

fn read_locator(root: &Path) -> Result<RuntimeLocator, String> {
    let path = locator_path(root);
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let locator: RuntimeLocator = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if locator.protocol_version != PROTOCOL_VERSION {
        return Err("protocol mismatch".into());
    }
    let expected = workspace_id(root).map_err(|e| e.to_string())?;
    if locator.workspace_id != expected {
        return Err("workspace mismatch".into());
    }
    if !process_alive(locator.pid) {
        return Err("stale process".into());
    }
    Ok(locator)
}

fn process_alive(pid: u32) -> bool {
    if cfg!(windows) {
        return true;
    }
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn read_transport(
    locator: &RuntimeLocator,
    stop: Arc<AtomicBool>,
    tx: &Sender<RuntimeEvent>,
) -> Result<(), String> {
    match locator.transport.kind {
        TransportKind::Unix => read_unix(&locator.transport.address, stop, tx),
        TransportKind::NamedPipe => read_named_pipe(&locator.transport.address, stop, tx),
    }
}

fn consume_lines<R: Read>(reader: R, stop: Arc<AtomicBool>, tx: &Sender<RuntimeEvent>) {
    let mut lines = BufReader::new(reader).lines();
    while !stop.load(Ordering::SeqCst) {
        match lines.next() {
            Some(Ok(line)) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(line) {
                    Ok(message) if message.get("type").and_then(Value::as_str) == Some("snapshot") => {
                        let _ = tx.send(RuntimeEvent::Snapshot(message.get("payload").cloned()));
                    }
                    Ok(_) => {}
                    Err(error) => {
                        let _ = tx.send(RuntimeEvent::Error(error.to_string()));
                    }
                }
            }
            Some(Err(_)) | None => break,
        }
    }
}

#[cfg(unix)]
fn read_unix(address: &str, stop: Arc<AtomicBool>, tx: &Sender<RuntimeEvent>) -> Result<(), String> {
    let stream = std::os::unix::net::UnixStream::connect(address).map_err(|e| e.to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    consume_lines(stream, stop, tx);
    Ok(())
}

#[cfg(not(unix))]
fn read_unix(_address: &str, _stop: Arc<AtomicBool>, _tx: &Sender<RuntimeEvent>) -> Result<(), String> {
    Err("unix sockets are not supported on this platform".into())
}

fn read_named_pipe(address: &str, stop: Arc<AtomicBool>, tx: &Sender<RuntimeEvent>) -> Result<(), String> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(address)
        .map_err(|e| e.to_string())?;
    consume_lines(file, stop, tx);
    Ok(())
}

/// Write a single newline-delimited JSON value to a writer. Used by smoke
/// tests to simulate runtime messages without a real socket.
#[allow(dead_code)]
pub fn write_line<W: Write>(mut writer: W, value: &Value) -> std::io::Result<()> {
    writeln!(writer, "{value}")
}

/// Convenience constructor for a test channel pair.
#[allow(dead_code)]
pub fn events() -> (Sender<RuntimeEvent>, Receiver<RuntimeEvent>) {
    mpsc::channel()
}
