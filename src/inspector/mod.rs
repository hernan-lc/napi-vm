//! Native interactive object inspector — a DevTools-style foldable tree over
//! a live guest `Value`, rendered with the `console.dir` palette. Gated behind
//! the `inspector` Cargo feature (off by default).
//!
//! Reachable two ways:
//! - `console.dir(obj, { inspect: true })` from guest code, and
//! - `vm.inspect("expression")` from the host.
//!
//! In a real terminal this takes over the screen (raw mode + alternate
//! screen) until you quit; in pipes / CI it transparently falls back to a
//! static pretty-printed dump and never blocks.
//!
//! Unlike the TypeScript `examples/inspector.ts`, this walks the guest `Value`
//! directly (no NAPI marshalling), so circular guest structures render as
//! `[Circular *n]` instead of being lost at the boundary.

pub mod config;
mod tree;

use std::io::{IsTerminal, Write};

use crossterm::cursor;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;

use crate::bindings::format::{Painter, colors_enabled};
use crate::value::Value;

use config::Config;
use tree::Tree;

/// A decoded user intent for one keypress.
enum Action {
    None,
    Quit,
    Up,
    Down,
    Expand,
    Collapse,
    ExpandAll,
    CollapseAll,
}

/// Inspect `value` under `label`. Interactive in a TTY, static dump otherwise.
pub fn inspect(value: &Value, label: &str, cfg: &Config) {
    let colors = cfg.colors.unwrap_or_else(colors_enabled);
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        // Non-interactive fallback: the cycle-safe pretty printer, already
        // multi-line and indented. Never blocks.
        let s = crate::bindings::to_string_pretty_colored(value, colors);
        println!("── {} ──\n{}", label, s);
        return;
    }
    if let Err(e) = run_session(value, label, cfg, colors) {
        // Never let the inspector take down the host: restore the terminal
        // best-effort, then report on stderr.
        let _ = terminal::disable_raw_mode();
        let _ = std::io::stdout().execute(DisableMouseCapture);
        let _ = std::io::stdout().execute(cursor::Show);
        let _ = std::io::stdout().execute(LeaveAlternateScreen);
        eprintln!("inspector error: {}", e);
    }
}

/// Map a key event to an [`Action`] using the configured keymap. Esc and
/// ctrl-c always quit, regardless of the keymap.
fn classify(keys: &config::Keymap, k: &KeyEvent) -> Action {
    use Action::*;
    if k.code == KeyCode::Esc {
        return Quit;
    }
    if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
        return Quit;
    }
    match k.code {
        KeyCode::Up => Up,
        KeyCode::Down => Down,
        KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => Expand,
        KeyCode::Left => Collapse,
        KeyCode::Char(c) => {
            if c == keys.quit {
                Quit
            } else if c == keys.up {
                Up
            } else if c == keys.down {
                Down
            } else if c == keys.expand {
                Expand
            } else if c == keys.collapse {
                Collapse
            } else if c == keys.expand_all {
                ExpandAll
            } else if c == keys.collapse_all {
                CollapseAll
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Apply an action to the tree/cursor. Returns `true` when the session should
/// quit.
fn apply(tree: &mut Tree, action: Action, cursor: &mut usize, start: &mut usize) -> bool {
    use Action::*;
    match action {
        Quit => return true,
        None => {}
        Up => *cursor = cursor.saturating_sub(1),
        Down => {
            let n = tree.visible_rows().len();
            *cursor = (*cursor + 1).min(n.saturating_sub(1));
        }
        Expand => {
            let rows = tree.visible_rows();
            if let Some(&idx) = rows.get(*cursor) {
                let was = tree.is_expanded(idx);
                tree.toggle(idx, true);
                // Step into the first child only if we actually revealed them.
                if !was && tree.visible_rows().len() > rows.len() {
                    *cursor += 1;
                }
            }
        }
        Collapse => {
            let rows = tree.visible_rows();
            if let Some(&idx) = rows.get(*cursor) {
                if tree.is_expanded(idx) {
                    tree.toggle(idx, false);
                } else if let Some(parent) = tree.parent_of(idx)
                    && let Some(pos) = tree.visible_rows().iter().position(|&r| r == parent)
                {
                    *cursor = pos;
                }
            }
        }
        ExpandAll => tree.set_all(true),
        CollapseAll => {
            tree.set_all(false);
            tree.expand_root();
            *cursor = 0;
            *start = 0;
        }
    }
    false
}

/// Breadcrumb path from the root to the focused node, e.g. `Object › address`.
fn breadcrumb(tree: &Tree, idx: Option<usize>) -> String {
    let Some(mut idx) = idx else {
        return String::new();
    };
    let mut parts = Vec::new();
    loop {
        parts.push(tree.label_of(idx));
        match tree.parent_of(idx) {
            Some(p) => idx = p,
            None => break,
        }
    }
    parts.reverse();
    parts.join(" › ")
}

/// Render one full frame into a string.
fn render_frame(
    tree: &Tree,
    p: &Painter,
    label: &str,
    keys: &config::Keymap,
    cursor: usize,
    start: usize,
    height: usize,
) -> String {
    let rows = tree.visible_rows();
    let focused = rows.get(cursor).copied();

    let mut out = String::with_capacity(4096);
    out.push_str("\x1b[H\x1b[2J"); // home + clear
    out.push_str(&format!(
        "{} {} {} {}\n",
        p.dim("── inspector:".to_string()),
        p.bold(label.to_string()),
        p.dim("──".to_string()),
        p.dim(breadcrumb(tree, focused)),
    ));
    out.push_str(&p.dim(format!(
        "  ↑/↓ move · →/{} expand · ←/{} collapse · {}/{} all · {}/esc quit",
        keys.expand, keys.collapse, keys.expand_all, keys.collapse_all, keys.quit,
    )));
    out.push('\n');

    let end = (start + height).min(rows.len());
    for (offset, &row) in rows[start..end].iter().enumerate() {
        let i = start + offset;
        let line = tree.render_row(row, p);
        if i == cursor {
            out.push_str(&p.inverse("❯".to_string()));
            out.push_str(&p.bold(line));
        } else {
            out.push(' ');
            out.push_str(&line);
        }
        out.push('\n');
    }
    out.push_str(&p.dim(format!(
        "── {}/{} ──",
        cursor.min(rows.len().saturating_sub(1)) + 1,
        rows.len()
    )));
    out
}

/// The interactive session: take over the terminal, loop on input, restore on
/// the way out.
fn run_session(
    value: &Value,
    label: &str,
    cfg: &Config,
    colors: bool,
) -> Result<(), std::io::Error> {
    let mut tree = Tree::new(value.clone());
    let painter = Painter::new(colors);
    let keys = cfg.keys.clone();

    let mut stdout = std::io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(cursor::Hide)?;

    let mut cursor: usize = 0;
    let mut start: usize = 0;

    let result = loop {
        let rows_len = tree.visible_rows().len();
        if cursor >= rows_len {
            cursor = rows_len.saturating_sub(1);
        }
        let (_, term_rows) = terminal::size()?;
        let height = (term_rows as usize).saturating_sub(3).max(3);
        if cursor < start {
            start = cursor;
        }
        if cursor >= start + height {
            start = cursor - height + 1;
        }

        let frame = render_frame(&tree, &painter, label, &keys, cursor, start, height);
        stdout.write_all(frame.as_bytes())?;
        stdout.flush()?;

        // Only key presses drive the session; resize and other events simply
        // fall through to a re-render.
        if let Event::Key(k) = event::read()? {
            let action = classify(&keys, &k);
            if apply(&mut tree, action, &mut cursor, &mut start) {
                break Ok(());
            }
        }
    };

    terminal::disable_raw_mode()?;
    stdout.execute(cursor::Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    result
}
