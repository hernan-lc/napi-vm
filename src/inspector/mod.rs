//! Native interactive object inspector — a DevTools-style foldable tree over
//! a live guest `Value`, rendered with the `console.dir` palette. Gated behind
//! the `inspector` Cargo feature (off by default).
//!
//! Reachable two ways:
//! - `console.dir(obj, { inspect: true })` from guest code, and
//! - `vm.inspect("expression")` from the host.
//!
//! Sessions render **inline** at the current cursor position — never on an
//! alternate screen — and block the host until closed: the event loop pauses
//! in `event::read()` (a blocking input wait, no sleeps or timers) while the
//! inspector is open. The live frame is exactly `term_rows - 1` lines tall
//! (header, padded tree viewport, footer hint), so the first draw settles it
//! at the top of the terminal and every redraw rewinds precisely the lines
//! it drew — the geometry can never drift onto neighboring output. On close
//! the live frame is replaced in place by a compact listing (header + the
//! tree window last shown) that **stays in the scrollback**, so repeated
//! inspections accumulate as a list in the console and every `console.log`
//! around them remains visible after the app exits. The tree starts
//! collapsed; open what you need. Controls: click a `▶`/`▼` row to
//! expand/collapse, wheel or arrow keys to scroll, `q`/Esc/ctrl-c or a click
//! outside the tree to close. In pipes / CI the inspector transparently
//! falls back to a static, depth-limited tree dump and never blocks.
//!
//! Unlike the TypeScript `examples/inspector.ts`, this walks the guest `Value`
//! directly (no NAPI marshalling), so circular guest structures render as
//! `[Circular *n]` instead of being lost at the boundary.

pub mod config;
mod tree;

use std::io::{IsTerminal, Write};

use crossterm::ExecutableCommand;
use crossterm::cursor;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::terminal::{self, Clear, ClearType};

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

/// The inline interactive session: render a fixed-height frame at the
/// current cursor position, block on input until the user closes it, and
/// leave a compact listing of the final view in the scrollback.
///
/// The frame is always exactly `viewport + 2` lines tall (header + padded
/// tree viewport + footer) and `viewport` is `term_rows - 3`, so the frame
/// is `term_rows - 1` lines: drawing it from *any* cursor row settles it at
/// terminal rows `0..term_rows-2` with the cursor on the last row, and it
/// stays there. That gives the redraw loop an exact invariant — rewind
/// `last_lines`, repaint `last_lines` lines — with no absolute anchors, no
/// cursor-position queries, and no chance of overrunning neighboring
/// output. The one blank row under the frame also means the frame's
/// trailing newline never scrolls the terminal mid-session.
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
    terminal::enable_raw_mode()?;
    stdout.execute(cursor::Hide)?;
    // Click to expand/collapse or close, wheel to scroll. Scoped to this
    // session and released on the way out (the `mouse` feature, implied by
    // `inspector`).
    stdout.execute(EnableMouseCapture)?;

    let mut start: usize = 0; // first visible tree row (scroll offset)
    let mut last_lines: usize = 0; // height of the frame currently on screen
    // The tree viewport of the frame currently on screen (for the final
    // listing). Uninitialized: the loop always draws a frame before it can
    // break, so it is assigned before any read.
    let mut last_viewport: usize;

    let result = loop {
        let (term_cols, term_rows) = terminal::size()?;
        let rows = tree.visible_rows();
        // Header + footer = 2 chrome lines, plus one blank row of margin
        // under the frame so its trailing newline never scrolls.
        let viewport = (term_rows as usize).saturating_sub(3).max(1);
        let max_start = rows.len().saturating_sub(viewport);
        if start > max_start {
            start = max_start;
        }

        // Rewind to the top of the previous frame, then repaint in place.
        if last_lines > 0 {
            write!(stdout, "{}", cursor::MoveUp(last_lines as u16))?;
        }
        let frame = render_inline_frame(
            &tree,
            &p,
            label,
            quit,
            FrameGeom { start, viewport, cols: term_cols },
            &rows,
        );
        stdout.write_all(frame.as_bytes())?;
        stdout.flush()?;
        // The tree region is padded with blank lines, so the frame is
        // exactly `viewport + 2` lines no matter how much is expanded —
        // this is what makes the rewind above exact.
        last_lines = viewport + 2;
        last_viewport = viewport;

        // Blocks until the next input event: this is what makes the session
        // modal — the host's event loop is parked here, no sleep required.
        match event::read()? {
            Event::Key(k) => {
                // Esc / ctrl-c always close; the quit letter is configurable.
                if k.code == KeyCode::Esc
                    || (k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c'))
                {
                    break Ok(());
                }
                match k.code {
                    KeyCode::Char(c) if c == quit => break Ok(()),
                    KeyCode::Up => start = start.saturating_sub(1),
                    KeyCode::Down => start = (start + 1).min(max_start),
                    KeyCode::PageUp => start = start.saturating_sub(viewport),
                    KeyCode::PageDown => start = (start + viewport).min(max_start),
                    KeyCode::Home => start = 0,
                    KeyCode::End => start = max_start,
                    _ => {}
                }
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => start = start.saturating_sub(3),
                MouseEventKind::ScrollDown => start = (start + 3).min(max_start),
                MouseEventKind::Down(MouseButton::Left) => {
                    // The frame occupies rows 0..term_rows-2 (see the
                    // function docs), so mouse rows map directly: row 0 is
                    // the header, rows 1..=viewport the tree, row
                    // viewport+1 the footer. A click on a tree row toggles
                    // it; a click on the chrome — or anywhere else — closes.
                    let r = m.row as usize;
                    if r >= 1 && r <= viewport {
                        let pos = start + (r - 1);
                        let end = (start + viewport).min(rows.len());
                        if pos < end {
                            let idx = rows[pos];
                            if tree.is_expandable(idx) {
                                let want = !tree.is_expanded(idx);
                                tree.toggle(idx, want);
                            }
                        }
                    } else {
                        break Ok(());
                    }
                }
                _ => {}
            },
            Event::Resize(..) => {
                // Best effort: rewind into the old frame, clear it, and let
                // the next iteration repaint (and re-settle) from there.
                if last_lines > 0 {
                    write!(stdout, "{}", cursor::MoveUp(last_lines as u16))?;
                }
                write!(stdout, "{}", Clear(ClearType::FromCursorDown))?;
                stdout.flush()?;
                last_lines = 0;
            }
            _ => {}
        }
    };

    // Restore the terminal first (raw mode off means `println!` newlines
    // get their carriage returns back), then replace the live frame with
    // the compact listing that stays in the scrollback: rewind to the
    // frame's top, erase it, and print header + the tree window that was
    // on screen at close. The cursor ends below the listing, so the
    // host's next print continues the console flow right there.
    if last_lines > 0 {
        write!(stdout, "{}", cursor::MoveUp(last_lines as u16))?;
    }
    write!(stdout, "{}", Clear(ClearType::FromCursorDown))?;
    stdout.flush()?;
    terminal::disable_raw_mode()?;
    stdout.execute(DisableMouseCapture)?;
    stdout.execute(cursor::Show)?;

    println!(
        "{} {} {}",
        p.dim("──".to_string()),
        p.bold(label.to_string()),
        p.dim("──".to_string())
    );
    let rows = tree.visible_rows();
    let end = (start + last_viewport).min(rows.len());
    for &row in &rows[start..end] {
        println!("{}", tree.render_row(row, &p));
    }
    // A blank line between sessions keeps the console list readable.
    println!();
    result
}

/// The geometry of one frame: the scroll window over the tree (`start` ..
/// `start + viewport`) and the terminal width it renders into.
struct FrameGeom {
    start: usize,
    viewport: usize,
    cols: u16,
}

/// Render one live frame as `viewport + 2` self-contained lines: header,
/// the tree window padded with blank lines to a constant height, and the
/// footer hint. Each line is clear + content + `\r\n`, so the frame can be
/// repainted wherever the cursor sits (the caller rewinds to the previous
/// frame's top first) and always leaves the cursor one line below it.
fn render_inline_frame(
    tree: &Tree,
    p: &Painter,
    label: &str,
    quit: char,
    g: FrameGeom,
    rows: &[usize],
) -> String {
    let FrameGeom { start, viewport, cols } = g;
    let cols = (cols as usize).max(10);
    let mut out = String::with_capacity(8192);

    let mut line = |s: String| {
        out.push_str(&format!(
            "{}{}\r\n",
            Clear(ClearType::CurrentLine),
            truncate_visible(&s, cols),
        ));
    };

    line(format!(
        "{} {} {}",
        p.dim("──".to_string()),
        p.bold(label.to_string()),
        p.dim("──".to_string())
    ));
    let end = (start + viewport).min(rows.len());
    for &row in &rows[start..end] {
        line(tree.render_row(row, p));
    }
    // Pad the tree region to a constant height (see `run_inline_session`'s
    // rewind invariant).
    for _ in (end - start)..viewport {
        line(String::new());
    }
    line(p.dim(format!(
        "click ▶/▼ to fold · wheel or ↑/↓ to scroll · {} or click here to close",
        quit
    )));
    out
}

/// Truncate an ANSI-colored string to `cols` *visible* columns, skipping SGR
/// escapes when counting. A cut line gets a style reset + ellipsis so colors
/// never leak past the cut — and, more importantly, so long lines never
/// soft-wrap, which would break the frame geometry.
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
