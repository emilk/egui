#![cfg_attr(doc, doc = include_str!("../README.md"))]
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]

pub mod protocol;

pub use protocol::{
    EncodedPng, MAX_MESSAGE_BYTES, PROTOCOL_MAGIC, PROTOCOL_VERSION, Request, Response,
    read_message, write_message,
};

/// The single environment variable that controls inspection.
///
/// Interpreted by [`bind_addr_from_env`]: falsy (unset / empty / `0` / `false`) disables it;
/// truthy (`1` / `true`) enables it on [`DEFAULT_INSPECTION_ADDR`]; anything else is taken as
/// a `host:port` bind address (e.g. `0.0.0.0:5719` to expose it across the network).
pub const INSPECTION_ENV_VAR: &str = "EGUI_INSPECTION";

/// Default bind address used when [`INSPECTION_ENV_VAR`] is just truthy.
///
/// Loopback only, on a fixed well-known port. The `egui_mcp` server defaults its `attach` to
/// this same port.
pub const DEFAULT_INSPECTION_ADDR: &str = "127.0.0.1:5719";

#[cfg(feature = "png")]
mod png;

#[cfg(feature = "plugin")]
mod plugin;

#[cfg(feature = "plugin")]
pub use plugin::InspectionPlugin;

#[cfg(all(feature = "plugin", not(target_arch = "wasm32")))]
pub use plugin::{attach_from_env, serve};

/// Resolve the bind address from [`INSPECTION_ENV_VAR`], returning `None` when inspection is
/// disabled. Used by [`attach_from_env`] and eframe's auto-attach.
#[cfg(feature = "plugin")]
pub fn bind_addr_from_env() -> Option<String> {
    let value = std::env::var(INSPECTION_ENV_VAR).ok()?;
    match value.trim() {
        "" | "0" => None,
        v if v.eq_ignore_ascii_case("false") => None,
        "1" => Some(DEFAULT_INSPECTION_ADDR.to_owned()),
        v if v.eq_ignore_ascii_case("true") => Some(DEFAULT_INSPECTION_ADDR.to_owned()),
        addr => Some(addr.to_owned()),
    }
}
