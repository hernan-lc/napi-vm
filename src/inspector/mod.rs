//! Native interactive object inspector — a DevTools-style foldable tree over
//! a live guest `Value`, rendered with the `console.dir` palette. Gated behind
//! the `inspector` Cargo feature (off by default).
//!
//! Reachable two ways:
//! - `console.dir(obj, { inspect: true })` from guest code, and
//! - `vm.inspect("expression")` from the host.
//!
//! Sessions render **inline**: the tree is printed at the current cursor
//! position — never on an alternate screen — and is driven with the mouse
//! (click a `▶`/`▼` row to expand/collapse, wheel to scroll, click outside
//! the tree to close). The final frame stays in the scrollback, so repeated
//! inspections accumulate as a list in the console. In pipes / CI it
//! transparently falls back to a static, depth-limited tree dump and never
//! blocks.
//!
//! Unlike the TypeScript `examples/inspector.ts`, this walks the guest `Value`
//! directly (no NAPI marshalling), so circular guest structures render as
//! `[Circular *n]` instead of being lost at the boundary.

pub mod config;
mod tree;

use std::io::{IsTerminal, Write};

use crossterm::cursor;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::ExecutableCommand;

use crate::bindings::format::{Painter, colors_enabled};
use crate::value::Value;

use config::Config;
use tree::Tree;

/// Inspect `value` under `label`. Interactive (mouse-driven) in a TTY, static
/// tree dump otherwise.
pub fn inspect(value: &Value, label: &str, cfg: &Config) {
    let colors = cfg.colors.unwrap_or_else(colors_enabled);
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        // Non-interactive fallback: the same tree rows as a live session,
        // expanded to `max_static_depth` and frozen. Never blocks.
        dump_static(value, label, cfg, colors);
        return;
    }
    if let Err(e) = run_inline_session(value, label, cfg, colors) {
        // Never let the inspector take down the host: restore the terminal
        // best-effort, then report on stderr.
        let _ = terminal::disable_raw_mode();
        let _ = std::io::stdout().execute(DisableMouseCapture);
        let _ = std::io::stdout().execute(cursor::Show);
        eprintln!("inspector error: {}", e);
    }
}

/// The non-TTY fallback: a depth-limited tree list (fold arrows + the
/// `console.dir` palette) printed straight into the console flow.
fn dump_static(value: &Value, label: &str, cfg: &Config, colors: bool) {
    let mut tree = Tree::new(value.clone());
    tree.expand_to_depth(cfg.max_static_depth);
    let p = Painter::new(colors);
    println!(
        "{} {} {}",
        p.dim("──".to_string()),
        p.bold(label.to_string()),
        p.dim("──".to_string())
    );
    for row in tree.visible_rows() {
        println!("{}", tree.render_row(row, &p));
    }
}

/// The inline interactive session: render the tree at the current cursor
/// position (no alternate screen), loop on mouse input until the user closes
/// it, and leave the final frame in the scrollback.
fn run_inline_session(
    value: &Value,
    label: &str,
    cfg: &Config,
    colors: bool,
) -> Result<(), std::io::Error> {
    let mut tree = Tree::new(value.clone());
    let p = Painter::new(colors);
    let quit = cfg.key_quit;

    let mut stdout = std::io::stdout();
    // Anchor the frame at column 0 so row arithmetic stays simple.
    if cursor::position().map(|(c, _)| c).unwrap_or(0) != 0 {
        stdout.write_all(b"\r\n")?;
        stdout.flush()?;
    }
    // Make sure there is room below for a minimal frame; otherwise the
    // terminal would scroll the anchor row out from under us.
    loop {
        let (_, term_rows) = terminal::size()?;
        let (_, row) = cursor::position()?;
        if term_rows.saturating_sub(row) >= 8 || term_rows < 10 {
            break;
        }
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }
    let (_, mut origin_row) = cursor::position().unwrap_or((0, 0));

    terminal::enable_raw_mode()?;
    stdout.execute(cursor::Hide)?;
    // Click to expand/collapse or close, wheel to scroll. Scoped to this
    // session and released on the way out (the `mouse` feature, implied by
    // `inspector`).
    stdout.execute(EnableMouseCapture)?;

    let mut start: usize = 0; // first visible tree row (scroll offset)
    let mut last_lines: usize = 0; // height of the frame currently on screen

    let result = loop {
        let (term_cols, term_rows) = terminal::size()?;
        let rows = tree.visible_rows();
        // header + hint + footer occupy 3 lines below the anchor.
        let viewport = (term_rows as usize)
            .saturating_sub(origin_row as usize + 3)
            .max(3);
        let max_start = rows.len().saturating_sub(viewport);
        if start > max_start {
            start = max_start;
        }

        let (frame, lines) =
            render_inline_frame(&tree, &p, label, quit, start, viewport, origin_row, term_cols, &rows);
        stdout.write_all(frame.as_bytes())?;
        // Erase leftovers when the new frame is shorter than the previous one.
        if lines < last_lines {
            write!(
                stdout,
                "{}{}",
                cursor::MoveTo(0, origin_row + lines as u16),
                Clear(ClearType::FromCursorDown)
            )?;
        }
        stdout.flush()?;
        last_lines = lines;

        match event::read()? {
            Event::Key(k) => {
                // Esc / ctrl-c always close; the quit letter is configurable.
                if k.code == KeyCode::Esc
                    || (k.modifiers.contains(KeyModifiers::CONTROL)
                        && k.code == KeyCode::Char('c'))
                    || k.code == KeyCode::Char(quit)
                {
                    break Ok(());
                }
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => start = start.saturating_sub(3),
                MouseEventKind::ScrollDown => start = (start + 3).min(max_start),
                MouseEventKind::Down(MouseButton::Left) => {
                    // The tree starts two lines under the anchor (header +
                    // hint) and ends one line above the frame bottom
                    // (footer). A click on a tree row toggles it; a click
                    // anywhere else — header, hint, footer, or outside the
                    // frame — closes the session.
                    let tree_top = origin_row + 2;
                    let tree_rows = last_lines.saturating_sub(3);
                    let r = m.row;
                    if r >= tree_top && r < tree_top + tree_rows as u16 {
                        let idx = rows[start + (r - tree_top) as usize];
                        if tree.is_expandable(idx) {
                            let want = !tree.is_expanded(idx);
                            tree.toggle(idx, want);
                        }
                    } else {
                        break Ok(());
                    }
                }
                _ => {}
            },
            Event::Resize(_, new_rows) => {
                // Discard the old frame and re-anchor so a shrunken terminal
                // cannot leave the anchor row off-screen.
                write!(
                    stdout,
                    "{}{}",
                    cursor::MoveTo(0, origin_row),
                    Clear(ClearType::FromCursorDown)
                )?;
                stdout.flush()?;
                last_lines = 0;
                if origin_row + 8 > new_rows {
                    // Only reachable when the terminal shrank drastically;
                    // moving up may repaint a line of prior output, which is
                    // preferable to an off-screen frame.
                    origin_row = new_rows.saturating_sub(8);
                }
            }
            _ => {}
        }
    };

    // Park the cursor just below the final frame, then restore the terminal.
    write!(stdout, "{}", cursor::MoveTo(0, origin_row + last_lines as u16))?;
    stdout.flush()?;
    terminal::disable_raw_mode()?;
    stdout.execute(DisableMouseCapture)?;
    stdout.execute(cursor::Show)?;
    // A blank line between sessions keeps the console list readable.
    println!();
    result
}

/// Render one inline frame using absolute cursor positioning: every line is
/// `MoveTo` + clear + content, so re-renders overwrite the previous frame in
/// place no matter how its height changed. Returns the frame bytes and the
/// total line count (tree rows are `lines - 3`: header, hint, footer).
#[allow(clippy::too_many_arguments)]
fn render_inline_frame(
    tree: &Tree,
    p: &Painter,
    label: &str,
    quit: char,
    start: usize,
    viewport: usize,
    origin_row: u16,
    term_cols: u16,
    rows: &[usize],
) -> (String, usize) {
    let cols = (term_cols as usize).max(10);
    let end = (start + viewport).min(rows.len());
    let mut out = String::with_capacity(4096);

    let mut line = |i: usize, s: String| {
        out.push_str(&format!(
            "{}{}{}",
            cursor::MoveTo(0, origin_row + i as u16),
            Clear(ClearType::CurrentLine),
            truncate_visible(&s, cols),
        ));
    };

    line(
        0,
        format!(
            "{} {} {}",
            p.dim("── inspector:".to_string()),
            p.bold(label.to_string()),
            p.dim("──".to_string()),
        ),
    );
    line(
        1,
        p.dim(format!(
            "  click ▶/▼ expand/collapse · wheel scroll · click outside or '{}' close",
            quit
        )),
    );
    for (offset, &row) in rows[start..end].iter().enumerate() {
        line(2 + offset, tree.render_row(row, p));
    }
    let footer = 2 + (end - start);
    line(
        footer,
        p.dim(format!("── {}–{}/{} ──", start + 1, end, rows.len())),
    );
    (out, footer + 1)
}

/// Truncate an ANSI-colored string to `cols` *visible* columns, skipping SGR
/// escapes when counting. A cut line gets a style reset + ellipsis so colors
/// never leak past the cut — and, more importantly, so long lines never
/// soft-wrap, which would break the in-place re-render geometry.
fn truncate_visible(s: &str, cols: usize) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len().min(cols + 32));
    let mut visible = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == 0x1b && bytes[i..].starts_with(b"\x1b[") {
            // Copy the SGR sequence verbatim; it has no visible width.
            match s[i..].find('m') {
                Some(rel) => {
                    out.push_str(&s[i..i + rel + 1]);
                    i += rel + 1;
                }
                None => break, // malformed escape; stop rather than guess
            }
            continue;
        }
        if visible >= cols {
            out.push_str("\x1b[0m…");
            return out;
        }
        let c = s[i..].chars().next().unwrap();
        out.push(c);
        visible += 1;
        i += c.len_utf8();
    }
    out
}
