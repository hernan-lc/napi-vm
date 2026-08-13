use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportKind {
    Unix,
    #[serde(rename = "named-pipe")]
    NamedPipe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transport {
    pub kind: TransportKind,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLocator {
    pub protocol_version: u32,
    pub workspace_id: String,
    pub session_id: String,
    pub pid: u32,
    pub transport: Transport,
}

pub fn locator_path(root: &Path) -> PathBuf {
    root.join(".napi-vm").join("runtime.json")
}

/// Match `crypto.createHash("sha256").update(realpath).digest("hex").slice(0, 20)`.
pub fn workspace_id(root: &Path) -> std::io::Result<String> {
    let resolved = std::fs::canonicalize(root)?;
    let digest = Sha256::digest(resolved.to_string_lossy().as_bytes());
    Ok(hex_prefix(&digest, 20))
}

fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    let mut out = String::with_capacity(chars);
    for byte in bytes {
        if out.len() >= chars {
            break;
        }
        let next = format!("{byte:02x}");
        for ch in next.chars() {
            if out.len() >= chars {
                break;
            }
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_id_is_20_hex_chars() {
        let id = workspace_id(Path::new(".")).unwrap();
        assert_eq!(id.len(), 20);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
