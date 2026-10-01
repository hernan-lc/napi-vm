//! Explicitly opted-in native service supervision. Connections are capabilities,
//! not a bundled MCP client. No dependency installation or operation retry.
use napi_vm_plugin_protocol::{PluginResult, RpcError};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    path::PathBuf,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    process::{Child, Command},
    task::JoinHandle,
    time::{Instant, sleep, timeout},
};

#[derive(Clone, Debug)]
pub struct NativeTarget {
    pub os: String,
    pub arch: String,
    pub libc: Option<String>,
    pub abi: String,
}
/// Environment may contain secrets. Intentionally does not implement Debug.
pub struct ManagedServiceSpec {
    pub name: String,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub target: NativeTarget,
    pub sha256: String,
    pub endpoint: String,
    pub readiness: HttpReadiness,
    pub startup_timeout: Duration,
    pub shutdown_timeout: Duration,
}
#[derive(Clone, Debug)]
pub struct HttpReadiness {
    pub address: SocketAddr,
    pub path: String,
    pub expected_status: u16,
}
pub struct ManagedService {
    child: Child,
    drainers: Vec<JoinHandle<()>>,
    log_bytes: Arc<AtomicU64>,
    endpoint: String,
    shutdown_timeout: Duration,
    stopped: bool,
}
fn invalid(message: &str) -> RpcError {
    RpcError::new("INVALID_ARGUMENT", message)
}
impl NativeTarget {
    pub fn validate_current(&self) -> PluginResult<()> {
        let os = match std::env::consts::OS {
            "macos" => "darwin",
            "windows" => "win32",
            x => x,
        };
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            x => x,
        };
        if self.os != os || self.arch != arch {
            return Err(invalid(
                "service target does not match current OS/architecture",
            ));
        }
        if self.abi != "native-executable" {
            return Err(invalid("managed services require native-executable ABI"));
        }
        if cfg!(target_os = "linux") {
            let libc = if cfg!(target_env = "musl") {
                "musl"
            } else {
                "gnu"
            };
            if self.libc.as_deref() != Some(libc) {
                return Err(invalid("service Linux libc must match explicitly"));
            }
        } else if self.libc.is_some() {
            return Err(invalid("libc is only valid for Linux services"));
        }
        Ok(())
    }
}
impl ManagedServiceSpec {
    /// Static preflight does not execute code. A checksum is integrity, not trust.
    pub async fn preflight(&self) -> PluginResult<()> {
        if self.name.is_empty() || self.name.len() > 200 || self.arguments.len() > 256 {
            return Err(invalid("invalid service name or argument count"));
        }
        if !self.executable.is_absolute() {
            return Err(invalid(
                "service executable must be an explicit absolute path",
            ));
        }
        self.target.validate_current()?;
        if self.startup_timeout.is_zero()
            || self.startup_timeout > Duration::from_secs(300)
            || self.shutdown_timeout.is_zero()
            || self.shutdown_timeout > Duration::from_secs(60)
        {
            return Err(invalid("service startup/shutdown deadline out of range"));
        }
        if !self.readiness.address.ip().is_loopback()
            || !self.readiness.path.starts_with('/')
            || self.readiness.path.bytes().any(|b| b < 32 || b == 127)
            || !(200..=599).contains(&self.readiness.expected_status)
        {
            return Err(invalid("readiness requires safe loopback HTTP path/status"));
        }
        let endpoint = format!("http://{}", self.readiness.address);
        if self.endpoint != endpoint && !self.endpoint.starts_with(&(endpoint + "/")) {
            return Err(invalid(
                "service endpoint must use readiness loopback authority",
            ));
        }
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(invalid("service SHA256 must be lowercase hex"));
        }
        let bytes = tokio::fs::read(&self.executable).await?;
        let actual = format!("{:x}", Sha256::digest(bytes));
        if actual != self.sha256 {
            return Err(invalid("service executable checksum mismatch"));
        }
        Ok(())
    }
}
async fn healthy(spec: &HttpReadiness) -> bool {
    let check = async {
        let mut stream = TcpStream::connect(spec.address).await?;
        let request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            spec.path, spec.address
        );
        stream.write_all(request.as_bytes()).await?;
        let mut bytes = Vec::with_capacity(256);
        let mut chunk = [0u8; 256];
        loop {
            let n = stream.read(&mut chunk).await?;
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..n]);
            if bytes.windows(2).any(|s| s == b"\r\n") {
                break;
            }
            if bytes.len() > 4096 {
                return Ok::<bool, std::io::Error>(false);
            }
        }
        let line = std::str::from_utf8(&bytes)
            .unwrap_or("")
            .split("\r\n")
            .next()
            .unwrap_or("");
        let mut words = line.split(' ');
        let version = words.next().unwrap_or("");
        let code = words.next().and_then(|s| s.parse::<u16>().ok());
        Ok::<bool, std::io::Error>(
            (version == "HTTP/1.1" || version == "HTTP/1.0") && code == Some(spec.expected_status),
        )
    };
    matches!(
        timeout(Duration::from_millis(500), check).await,
        Ok(Ok(true))
    )
}
impl ManagedService {
    pub async fn start(spec: ManagedServiceSpec) -> PluginResult<Self> {
        spec.preflight().await?;
        // Fail if the endpoint was already occupied; never attach to a different service.
        match timeout(
            spec.startup_timeout.min(Duration::from_millis(500)),
            TcpStream::connect(spec.readiness.address),
        )
        .await
        {
            Ok(Ok(_)) => {
                return Err(invalid(
                    "managed service readiness address is already occupied",
                ));
            }
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::ConnectionRefused => {}
            Ok(Err(_)) => {
                return Err(invalid(
                    "managed service readiness address could not be checked",
                ));
            }
            Err(_) => {
                return Err(RpcError::new(
                    "DEADLINE_EXCEEDED",
                    "managed service address preflight timed out",
                ));
            }
        }
        let mut command = Command::new(&spec.executable);
        command
            .args(&spec.arguments)
            .envs(&spec.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let counter = Arc::new(AtomicU64::new(0));
        let mut drainers = Vec::new();
        if let Some(mut out) = child.stdout.take() {
            let c = counter.clone();
            drainers.push(tokio::spawn(async move {
                let mut b = [0u8; 8192];
                while let Ok(n) = out.read(&mut b).await {
                    if n == 0 {
                        break;
                    }
                    c.fetch_add(n as u64, Ordering::Relaxed);
                }
            }));
        }
        if let Some(mut err) = child.stderr.take() {
            let c = counter.clone();
            drainers.push(tokio::spawn(async move {
                let mut b = [0u8; 8192];
                while let Ok(n) = err.read(&mut b).await {
                    if n == 0 {
                        break;
                    }
                    c.fetch_add(n as u64, Ordering::Relaxed);
                }
            }));
        }
        let mut owned = Self {
            child,
            drainers,
            log_bytes: counter,
            endpoint: spec.endpoint,
            shutdown_timeout: spec.shutdown_timeout,
            stopped: false,
        };
        let deadline = Instant::now() + spec.startup_timeout;
        loop {
            if owned.child.try_wait()?.is_some() {
                owned.shutdown().await?;
                return Err(RpcError::new(
                    "PLUGIN_EXITED",
                    "managed service exited before readiness",
                ));
            }
            if Instant::now() >= deadline {
                owned.shutdown().await?;
                return Err(RpcError::new(
                    "DEADLINE_EXCEEDED",
                    "managed service readiness deadline exceeded",
                ));
            }
            if healthy(&spec.readiness).await {
                return Ok(owned);
            }
            sleep(Duration::from_millis(25)).await;
        }
    }
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
    /// Sidecar output is continuously discarded, not retained, to avoid leaking credentials.
    pub fn drained_log_bytes(&self) -> u64 {
        self.log_bytes.load(Ordering::Relaxed)
    }
    /// Deterministic direct-child termination. No claim to arbitrary descendants.
    pub async fn shutdown(&mut self) -> PluginResult<()> {
        if self.stopped {
            return Ok(());
        }
        self.stopped = true;
        if self.child.try_wait()?.is_none() {
            self.child.start_kill()?;
            timeout(self.shutdown_timeout, self.child.wait())
                .await
                .map_err(|_| {
                    RpcError::new(
                        "DEADLINE_EXCEEDED",
                        "managed service did not reap before deadline",
                    )
                })??;
        }
        for task in self.drainers.drain(..) {
            task.abort();
            let _ = task.await;
        }
        Ok(())
    }
}
impl Drop for ManagedService {
    fn drop(&mut self) {
        if !self.stopped {
            let _ = self.child.start_kill();
        }
        for task in &self.drainers {
            task.abort();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_rejects_mismatch() {
        let target = NativeTarget {
            os: "wrong".into(),
            arch: "x64".into(),
            libc: None,
            abi: "native-executable".into(),
        };
        assert!(target.validate_current().is_err());
    }
    #[tokio::test]
    async fn readiness_checks_http_status() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut b = [0; 1024];
            let _ = s.read(&mut b).await;
            s.write_all(b"HTTP/1.1 503 Not Ready\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
        });
        assert!(
            !healthy(&HttpReadiness {
                address,
                path: "/health".into(),
                expected_status: 200
            })
            .await
        );
        task.await.unwrap();
    }
    #[test]
    #[ignore = "controlled sidecar child invoked only by lifecycle test"]
    fn service_child() {
        let address =
            std::env::var("NAPI_VM_TEST_HEALTH_ADDRESS").expect("managed fixture address");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = tokio::net::TcpListener::bind(address).await.unwrap();
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut b = [0u8; 1024];
                let _ = stream.read(&mut b).await;
                let _ = stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK",
                    )
                    .await;
            }
        });
    }
    #[tokio::test]
    async fn starts_only_after_http_readiness_and_reaps_direct_child() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let exe = std::env::current_exe().unwrap();
        let hash = format!("{:x}", Sha256::digest(tokio::fs::read(&exe).await.unwrap()));
        let os = match std::env::consts::OS {
            "macos" => "darwin",
            "windows" => "win32",
            x => x,
        };
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            x => x,
        };
        let spec = ManagedServiceSpec {
            name: "fixture".into(),
            executable: exe,
            arguments: vec![
                "--ignored".into(),
                "--exact".into(),
                "services::tests::service_child".into(),
            ],
            environment: BTreeMap::from([(
                "NAPI_VM_TEST_HEALTH_ADDRESS".into(),
                address.to_string(),
            )]),
            target: NativeTarget {
                os: os.into(),
                arch: arch.into(),
                libc: if cfg!(target_os = "linux") {
                    Some(
                        if cfg!(target_env = "musl") {
                            "musl"
                        } else {
                            "gnu"
                        }
                        .into(),
                    )
                } else {
                    None
                },
                abi: "native-executable".into(),
            },
            sha256: hash,
            endpoint: format!("http://{address}"),
            readiness: HttpReadiness {
                address,
                path: "/health".into(),
                expected_status: 200,
            },
            startup_timeout: Duration::from_secs(5),
            shutdown_timeout: Duration::from_secs(2),
        };
        let mut service = ManagedService::start(spec).await.unwrap();
        assert!(service.pid().is_some());
        assert_eq!(service.endpoint(), format!("http://{address}"));
        service.shutdown().await.unwrap();
        service.shutdown().await.unwrap();
        assert!(service.child.try_wait().unwrap().is_some());
    }
}
