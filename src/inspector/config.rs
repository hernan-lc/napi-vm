//! Inspector configuration: display options with sensible defaults, an
//! environment-variable override, and a process-global config that the NAPI
//! layer can mutate via `setInspectorConfig`.
//!
//! Precedence (lowest to highest): built-in defaults → the `INSPECTOR_DEPTH`
//! env var → `setInspectorConfig()` calls. A dump snapshots the global
//! config when it starts, so changes take effect on the next
//! `inspect`/`console.dir`.

use std::sync::{LazyLock, Mutex};

/// Display configuration for the inspector. The inspector is a non-blocking
/// inline tree dump — there is no session and no keymap, just how much of
/// the tree to open and whether to colorize it.
#[derive(Debug, Clone)]
pub struct Config {
    /// Force colors on (`Some(true)`) or off (`Some(false)`); `None` means
    /// auto-detect (TTY + `NO_COLOR`/`FORCE_COLOR`, via `colors_enabled`).
    pub colors: Option<bool>,
    /// How many levels deep the dump opens containers before leaving them
    /// closed (`▶` rows). `0` — the default — prints the tree fully closed.
    pub depth: usize,
}

impl Default for Config {
    fn default() -> Self {
        let depth = std::env::var("INSPECTOR_DEPTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        Config {
            colors: None,
            depth,
        }
    }
}

/// Process-global config. `setInspectorConfig` mutates it; each dump reads a
/// snapshot via [`current`] when it starts.
pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::default()));

/// Clone the current global config (a snapshot for one dump).
pub fn current() -> Config {
    CONFIG.lock().unwrap().clone()
}

/// Mutate the global config under the lock.
pub fn update(f: impl FnOnce(&mut Config)) {
    let mut guard = CONFIG.lock().unwrap();
    f(&mut guard);
}
