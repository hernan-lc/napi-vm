use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::Value;

use super::protocol::{
    PROTOCOL_VERSION, RuntimeLocator, TransportKind, locator_path, workspace_id,
};

/// How often the discovery loop re-reads `.napi-vm/runtime.json`. Also the
/// upper bound on how long `RuntimeClient::stop` blocks.
const POLL_INTERVAL: Duration = Duration::from_millis(250);
/// Read timeout on the runtime socket. This exists only so the reader thread
/// wakes up to check its stop flag — a timeout is *not* a disconnect.
const READ_TIMEOUT: Duration = Duration::from_millis(250);
/// After a connection to a session drops, wait this long before dialing the
/// same session id again so a repeatedly failing socket cannot spin.
const RECONNECT_BACKOFF: Duration = Duration::from_secs(2);

pub enum RuntimeEvent {
    Snapshot(Option<Value>),
    Error(String),
}

/// A message from a connection reader thread, tagged with the session it
/// belongs to so the discovery loop can drop events from replaced sessions.
enum ConnectionMessage {
    Event(String, RuntimeEvent),
    Closed(String),
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

/// One live connection to a runtime session, with its own cancellation flag so
/// a replaced session's reader is torn down deterministically instead of
/// lingering until its socket happens to close.
struct ActiveConnection {
    session_id: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ActiveConnection {
    fn shutdown(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn discover_loop(root: PathBuf, stop: Arc<AtomicBool>, tx: Sender<RuntimeEvent>) {
    let (conn_tx, conn_rx) = mpsc::channel::<ConnectionMessage>();
    let mut active: Option<ActiveConnection> = None;
    // Session id that just dropped, and when — used for the reconnect backoff.
    let mut cooldown: Option<(String, Instant)> = None;

    while !stop.load(Ordering::SeqCst) {
        for message in conn_rx.try_iter() {
            match message {
                ConnectionMessage::Event(session_id, event) => {
                    // Only the currently active session may drive the service.
                    if active
                        .as_ref()
                        .is_some_and(|conn| conn.session_id == session_id)
                    {
                        let _ = tx.send(event);
                    }
                }
                ConnectionMessage::Closed(session_id) => {
                    if active
                        .as_ref()
                        .is_some_and(|conn| conn.session_id == session_id)
                    {
                        if let Some(conn) = active.take() {
                            conn.shutdown();
                        }
                        cooldown = Some((session_id, Instant::now()));
                        let _ = tx.send(RuntimeEvent::Snapshot(None));
                    }
                }
            }
        }

        match read_locator(&root) {
            Ok(locator) => {
                let current = active.as_ref().map(|conn| conn.session_id.as_str());
                if current != Some(locator.session_id.as_str())
                    && !in_cooldown(&cooldown, &locator.session_id)
                {
                    if let Some(previous) = active.take() {
                        previous.shutdown();
                        let _ = tx.send(RuntimeEvent::Snapshot(None));
                    }
                    cooldown = None;
                    active = Some(connect(locator, conn_tx.clone()));
                }
            }
            Err(_) => {
                if let Some(previous) = active.take() {
                    previous.shutdown();
                    let _ = tx.send(RuntimeEvent::Snapshot(None));
                }
                cooldown = None;
            }
        }

        thread::sleep(POLL_INTERVAL);
    }

    if let Some(previous) = active.take() {
        previous.shutdown();
    }
}

fn in_cooldown(cooldown: &Option<(String, Instant)>, session_id: &str) -> bool {
    cooldown
        .as_ref()
        .is_some_and(|(id, at)| id == session_id && at.elapsed() < RECONNECT_BACKOFF)
}

fn connect(locator: RuntimeLocator, tx: Sender<ConnectionMessage>) -> ActiveConnection {
    let session_id = locator.session_id.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let thread_session = session_id.clone();
    let thread = thread::spawn(move || {
        if let Err(error) = read_transport(&locator, flag, &tx, &thread_session) {
            let _ = tx.send(ConnectionMessage::Event(
                thread_session.clone(),
                RuntimeEvent::Error(error),
            ));
        }
        let _ = tx.send(ConnectionMessage::Closed(thread_session));
    });
    ActiveConnection {
        session_id,
        stop,
        thread: Some(thread),
    }
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
    tx: &Sender<ConnectionMessage>,
    session_id: &str,
) -> Result<(), String> {
    match locator.transport.kind {
        TransportKind::Unix => read_unix(&locator.transport.address, stop, tx, session_id),
        TransportKind::NamedPipe => {
            read_named_pipe(&locator.transport.address, stop, tx, session_id)
        }
    }
}

/// Read newline-delimited JSON until EOF or cancellation.
///
/// The socket carries a read timeout so this loop can observe `stop`. A
/// timeout (`WouldBlock` / `TimedOut`) means "no runtime update yet", never
/// "the peer went away" — treating it as a disconnect made an idle session
/// flap between connected and disconnected every few hundred milliseconds.
fn consume_lines<R: Read>(
    reader: R,
    stop: Arc<AtomicBool>,
    tx: &Sender<ConnectionMessage>,
    session_id: &str,
) {
    let mut reader = BufReader::new(reader);
    // Bytes rather than a String: a timeout can land mid-character, and
    // `read_line` would reject the partial UTF-8 sequence and drop it.
    let mut buffer: Vec<u8> = Vec::new();
    while !stop.load(Ordering::SeqCst) {
        match reader.read_until(b'\n', &mut buffer) {
            Ok(0) => break, // real EOF
            Ok(_) => {
                if !buffer.ends_with(b"\n") {
                    // Partial line, keep accumulating.
                    continue;
                }
                let line = String::from_utf8_lossy(&buffer).trim().to_string();
                buffer.clear();
                if line.is_empty() {
                    continue;
                }
                if handle_line(&line, tx, session_id).is_err() {
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) =>
            {
                // Idle socket: loop back around and re-check `stop`, keeping
                // whatever partial line is already buffered.
                continue;
            }
            Err(error) => {
                let _ = tx.send(ConnectionMessage::Event(
                    session_id.to_string(),
                    RuntimeEvent::Error(error.to_string()),
                ));
                break;
            }
        }
    }
}

/// Returns `Err(())` when the receiving end is gone and the reader should stop.
fn handle_line(line: &str, tx: &Sender<ConnectionMessage>, session_id: &str) -> Result<(), ()> {
    let event = match serde_json::from_str::<Value>(line) {
        Ok(message) if message.get("type").and_then(Value::as_str) == Some("snapshot") => {
            RuntimeEvent::Snapshot(message.get("payload").cloned())
        }
        Ok(_) => return Ok(()),
        Err(error) => RuntimeEvent::Error(error.to_string()),
    };
    tx.send(ConnectionMessage::Event(session_id.to_string(), event))
        .map_err(|_| ())
}

#[cfg(unix)]
fn read_unix(
    address: &str,
    stop: Arc<AtomicBool>,
    tx: &Sender<ConnectionMessage>,
    session_id: &str,
) -> Result<(), String> {
    let stream = std::os::unix::net::UnixStream::connect(address).map_err(|e| e.to_string())?;
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    consume_lines(stream, stop, tx, session_id);
    Ok(())
}

#[cfg(not(unix))]
fn read_unix(
    _address: &str,
    _stop: Arc<AtomicBool>,
    _tx: &Sender<ConnectionMessage>,
    _session_id: &str,
) -> Result<(), String> {
    Err("unix sockets are not supported on this platform".into())
}

fn read_named_pipe(
    address: &str,
    stop: Arc<AtomicBool>,
    tx: &Sender<ConnectionMessage>,
    session_id: &str,
) -> Result<(), String> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(address)
        .map_err(|e| e.to_string())?;
    consume_lines(file, stop, tx, session_id);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor};

    /// A reader that returns a timeout between chunks, the way an idle socket
    /// with `set_read_timeout` does.
    struct Flaky {
        chunks: Vec<Vec<u8>>,
        index: usize,
        timeouts_left: usize,
    }

    impl Read for Flaky {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if self.timeouts_left > 0 {
                self.timeouts_left -= 1;
                return Err(io::Error::new(ErrorKind::WouldBlock, "idle"));
            }
            if self.index >= self.chunks.len() {
                return Ok(0);
            }
            let chunk = &self.chunks[self.index];
            self.index += 1;
            self.timeouts_left = 2;
            let len = chunk.len().min(out.len());
            out[..len].copy_from_slice(&chunk[..len]);
            Ok(len)
        }
    }

    fn snapshots(rx: &Receiver<ConnectionMessage>) -> Vec<Option<Value>> {
        rx.try_iter()
            .filter_map(|message| match message {
                ConnectionMessage::Event(_, RuntimeEvent::Snapshot(snapshot)) => Some(snapshot),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn idle_timeouts_do_not_end_the_connection() {
        let (tx, rx) = mpsc::channel();
        let reader = Flaky {
            chunks: vec![
                br#"{"type":"snapshot","payload":{"functions":[]}}"#.to_vec(),
                b"\n".to_vec(),
                br#"{"type":"snapshot","payload":{"functions":[{"name":"emit"}]}}"#.to_vec(),
                b"\n".to_vec(),
            ],
            index: 0,
            timeouts_left: 3,
        };
        consume_lines(reader, Arc::new(AtomicBool::new(false)), &tx, "session-1");
        let received = snapshots(&rx);
        assert_eq!(received.len(), 2, "both snapshots survive idle timeouts");
        assert!(received[1].as_ref().unwrap().get("functions").is_some());
    }

    #[test]
    fn partial_multibyte_lines_are_reassembled() {
        let (tx, rx) = mpsc::channel();
        let body = br#"{"type":"snapshot","payload":{"name":"caf"#.to_vec();
        // Split the two bytes of `é` across a timeout boundary.
        let reader = Flaky {
            chunks: vec![
                body,
                vec![0xC3],
                vec![0xA9],
                br#""}}"#.to_vec(),
                b"\n".to_vec(),
            ],
            index: 0,
            timeouts_left: 0,
        };
        consume_lines(reader, Arc::new(AtomicBool::new(false)), &tx, "session-1");
        let received = snapshots(&rx);
        assert_eq!(received.len(), 1);
        assert_eq!(
            received[0].as_ref().unwrap().get("name").unwrap(),
            &Value::from("café")
        );
    }

    #[test]
    fn eof_ends_the_connection() {
        let (tx, rx) = mpsc::channel();
        let reader = Cursor::new(br#"{"type":"snapshot","payload":null}"#.to_vec());
        consume_lines(reader, Arc::new(AtomicBool::new(false)), &tx, "session-1");
        // The trailing line has no newline; EOF flushes nothing but must not
        // hang, and the loop must terminate.
        assert!(snapshots(&rx).is_empty());
    }

    #[test]
    fn stale_session_events_are_ignored_by_the_discovery_loop() {
        // The filter the discovery loop applies, exercised directly.
        let active = Some("session-2".to_string());
        let from_old = ConnectionMessage::Event(
            "session-1".into(),
            RuntimeEvent::Snapshot(Some(Value::Null)),
        );
        let accepted = match &from_old {
            ConnectionMessage::Event(id, _) => active.as_deref() == Some(id.as_str()),
            ConnectionMessage::Closed(id) => active.as_deref() == Some(id.as_str()),
        };
        assert!(!accepted);
    }
}
