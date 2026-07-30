//! Inspector configuration: the keymap and display options, with sensible
//! defaults, environment-variable overrides, and a process-global config that
//! the NAPI layer can mutate via `setInspectorConfig`.
//!
//! Precedence (lowest to highest): built-in defaults → `INSPECTOR_*` env vars
//! → `setInspectorConfig()` calls. A session snapshots the global config when
//! it starts, so changes take effect on the next `inspect`/`console.dir`.

use std::sync::{LazyLock, Mutex};

/// Letter shortcuts for the inspector. The structural keys — arrow keys,
/// space, enter, esc, ctrl-c — are always active and not configurable; these
/// configure the mnemonic *letter* keys (vi-style by default).
#[derive(Debug, Clone)]
pub struct Keymap {
    pub up: char,
    pub down: char,
    pub expand: char,
    pub collapse: char,
    pub expand_all: char,
    pub collapse_all: char,
    pub quit: char,
}

impl Default for Keymap {
    fn default() -> Self {
        Keymap {
            up: 'k',
            down: 'j',
            expand: 'l',
            collapse: 'h',
            expand_all: 'e',
            collapse_all: 'c',
            quit: 'q',
        }
    }
}

impl Keymap {
    /// Override individual letters from `INSPECTOR_KEY_*` env vars, e.g.
    /// `INSPECTOR_KEY_QUIT=x`. Values that are not exactly one character are
    /// ignored so a malformed environment can never break the inspector.
    fn apply_env(&mut self) {
        fn read(slot: &mut char, var: &str) {
            if let Ok(v) = std::env::var(var) {
                let mut chars = v.chars();
                if let (Some(c), None) = (chars.next(), chars.next()) {
                    *slot = c;
                }
            }
        }
        read(&mut self.up, "INSPECTOR_KEY_UP");
        read(&mut self.down, "INSPECTOR_KEY_DOWN");
        read(&mut self.expand, "INSPECTOR_KEY_EXPAND");
        read(&mut self.collapse, "INSPECTOR_KEY_COLLAPSE");
        read(&mut self.expand_all, "INSPECTOR_KEY_EXPAND_ALL");
        read(&mut self.collapse_all, "INSPECTOR_KEY_COLLAPSE_ALL");
        read(&mut self.quit, "INSPECTOR_KEY_QUIT");
    }
}

/// Display + input configuration for an inspector session.
#[derive(Debug, Clone)]
pub struct Config {
    /// Force colors on (`Some(true)`) or off (`Some(false)`); `None` means
    /// auto-detect (TTY + `NO_COLOR`/`FORCE_COLOR`, via `colors_enabled`).
    pub colors: Option<bool>,
    /// Reserved for future static-fallback tuning; the non-TTY dump already
    /// delegates to the cycle-safe pretty printer.
    pub max_static_depth: usize,
    pub keys: Keymap,
}

impl Default for Config {
    fn default() -> Self {
        let mut keys = Keymap::default();
        keys.apply_env();
        let max_static_depth = std::env::var("INSPECTOR_DEPTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4);
        Config {
            colors: None,
            max_static_depth,
            keys,
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
