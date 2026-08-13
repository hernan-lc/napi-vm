//! Transport-only LSP adapter and runtime protocol client.
//!
//! Language intelligence stays in [`crate::lang`]. This module only maps
//! JSON-RPC / Runtime Protocol v1 onto [`crate::lang::LanguageService`].

mod protocol;
mod runtime_client;
mod server;
mod text;

pub use protocol::{PROTOCOL_VERSION, RuntimeLocator, TransportKind, locator_path, workspace_id};
pub use runtime_client::RuntimeClient;
pub use server::run;
