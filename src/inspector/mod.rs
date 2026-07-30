//! Native object inspector — a DevTools-style foldable tree over a live
//! guest `Value`, rendered with the `console.dir` palette. Gated behind the
//! `inspector` Cargo feature (off by default).
//!
//! Reachable two ways:
//! - `console.dir(obj, { inspect: true })` from guest code, and
//! - `vm.inspect("expression")` from the host.
//!
//! The inspector **never blocks**: it prints a compact tree inline — at the
//! current position in the console flow, never on an alternate screen and
//! never full-page — and returns immediately, so the host's event loop and
//! output continue uninterrupted. The tree is printed **closed by default**
//! (every container shows a `▶` fold hint); `depth` in the config — or the
//! `INSPECTOR_DEPTH` env var — expands it level by level. Each dump stays in
//! the scrollback like any other log line, so repeated inspections
//! accumulate as a list in the console alongside `console.log` output, all
//! of it visible after the app exits. There is no interactive session, no
//! raw mode, no mouse capture, and no key to press — nothing to close.
//!
//! Unlike the TypeScript `examples/inspector.ts`, this walks the guest `Value`
//! directly (no NAPI marshalling), so circular guest structures render as
//! `[Circular *n]` instead of being lost at the boundary.

pub mod config;
mod tree;

use crate::bindings::format::{Painter, colors_enabled};
use crate::value::Value;

use config::Config;
use tree::Tree;

/// Print `value` under `label` as an inline tree dump and return
/// immediately. Never blocks, in a TTY or a pipe alike: containers start
/// closed and the dump expands them down to `cfg.depth` levels.
pub fn inspect(value: &Value, label: &str, cfg: &Config) {
    let colors = cfg.colors.unwrap_or_else(colors_enabled);
    let mut tree = Tree::new(value.clone());
    tree.expand_to_depth(cfg.depth);
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
