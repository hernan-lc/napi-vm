//! N-API bindings for the VM: everything that crosses the Rust ↔ Node.js
//! boundary.
//!
//! - [`format`]  — string rendering of values (`to_string`, the pretty
//!   printer, and its ANSI color support)
//! - [`marshal`] — structured value marshalling built on the raw `napi_sys`
//!   ABI (`to_napi` / `from_napi`)
//! - [`bridge`]  — host function bridge: persisted `napi_ref`s, synchronous
//!   calls, and TSFN-based async dispatch for `exposeAsyncFunction`
//! - [`vm`]      — the `#[napi]`-exported `VM` class and free functions
//!
//! The re-exports below keep the module's public API unchanged, so
//! `crate::bindings::{VM, create_vm, to_string, ..}` resolve exactly as
//! they did before the split.
mod bridge;
mod format;
mod marshal;
mod vm;

pub use format::{colors_enabled, to_string, to_string_pretty, to_string_pretty_colored};
pub use vm::{create_vm, debug_parse, run_code, run_source, VM};
