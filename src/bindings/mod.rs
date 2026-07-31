//! N-API bindings for the VM: everything that crosses the Rust ↔ Node.js
//! boundary.
//!
//! - [`marshal`] — structured value marshalling built on the raw `napi_sys`
//!   ABI (`to_napi` / `from_napi`)
//! - [`bridge`]  — host function bridge: persisted `napi_ref`s, synchronous
//!   calls, and TSFN-based async dispatch for `exposeAsyncFunction`
//! - [`vm`]      — the `#[napi]`-exported `VM` class and free functions
//!
//! Value string rendering (`to_string`, the pretty printer) lives in the
//! NAPI-free [`crate::format`] module; it is re-exported here so
//! `crate::bindings::{VM, create_vm, to_string, ..}` resolve exactly as
//! they did before the split.
mod bridge;
mod marshal;
mod vm;

pub use crate::format::{colors_enabled, to_string, to_string_pretty, to_string_pretty_colored};
pub use vm::{VM, create_vm, debug_parse, run_code, run_source};
