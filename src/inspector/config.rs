//! Inspector configuration: display options and the close key, with sensible
//! defaults, environment-variable overrides, and a process-global config that
//! the NAPI layer can mutate via `setInspectorConfig`.
//!
//! Precedence (lowest to highest): built-in defaults → `INSPECTOR_*` env vars
//! → `setInspectorConfig()` calls. A session snapshots the global config when
//! it starts, so changes take effect on the next `inspect`/`console.dir`.

use std::sync::{LazyLock, Mutex};

/// Display + input configuration for an inspector session. Sessions are
/// mouse-driven (click to expand/collapse, wheel to scroll, click outside to
/// close); the only keyboard input is the close key, Esc, and ctrl-c.
#[derive(Debug, Clone)]
pub struct Config {
    /// Force colors on (`Some(true)`) or off (`Some(false)`); `None` means
    /// auto-detect (TTY + `NO_COLOR`/`FORCE_COLOR`, via `colors_enabled`).
    pub colors: Option<bool>,
    /// How deep the static (non-TTY) tree dump expands containers before
    /// collapsing them into `▶` rows.
    pub max_static_depth: usize,
    /// Letter that closes an interactive session (Esc and ctrl-c always
    /// close). Values that are not exactly one character are ignored so a
    /// malformed environment can never break the inspector.
    pub key_quit: char,
}

impl Default for Config {
    fn default() -> Self {
        let max_static_depth = std::env::var("INSPECTOR_DEPTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4);
        let key_quit = std::env::var("INSPECTOR_KEY_QUIT")
            .ok()
            .and_then(|v| {
                let mut chars = v.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => Some(c),
                    _ => None,
                }
            })
            .unwrap_or('q');
        Config {
            colors: None,
            max_static_depth,
            key_quit,
        }
    }
}

/// Process-global config. `setInspectorConfig` mutates it; each session reads
/// a snapshot via [`current`] when it starts.
pub static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::default()));

/// Clone the current global config (a snapshot for one session).
pub fn current() -> Config {
    CONFIG.lock().unwrap().clone()
}

/// Mutate the global config under the lock.
pub fn update(f: impl FnOnce(&mut Config)) {
    let mut guard = CONFIG.lock().unwrap();
    f(&mut guard);
}
