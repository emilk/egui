#![cfg_attr(doc, doc = include_str!("../README.md"))]
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]

pub mod protocol;

pub use protocol::{
    MAX_MESSAGE_BYTES, PROTOCOL_VERSION, Request, Response, read_message, write_message,
};

/// Environment variable that *enables* inspection when set to any non-empty value other than
/// `0`/`false`. The peer binds [`DEFAULT_INSPECTION_ADDR`] unless [`INSPECTION_ADDR_ENV_VAR`]
/// overrides it.
pub const INSPECTION_ENABLE_ENV_VAR: &str = "EGUI_INSPECTION";

/// Environment variable holding a `host:port` bind address (e.g. `127.0.0.1:5719` for
/// local-only, `0.0.0.0:5719` to expose across the network). Setting it also enables
/// inspection, so you don't need [`INSPECTION_ENABLE_ENV_VAR`] as well.
pub const INSPECTION_ADDR_ENV_VAR: &str = "EGUI_INSPECTION_ADDR";

/// Default bind address when inspection is enabled without an explicit
/// [`INSPECTION_ADDR_ENV_VAR`]: loopback only, on a fixed well-known port. The `egui_mcp`
/// server defaults its `attach` to this same port.
pub const DEFAULT_INSPECTION_ADDR: &str = "127.0.0.1:5719";

#[cfg(feature = "png")]
mod png;

#[cfg(feature = "png")]
pub use png::encode_png;

#[cfg(feature = "plugin")]
mod plugin;

#[cfg(feature = "plugin")]
pub use plugin::{InspectionPlugin, attach_from_env, serve};

/// Resolve the bind address from the environment, returning `None` when inspection is not
/// enabled. Used by [`attach_from_env`] and eframe's auto-attach.
#[cfg(feature = "plugin")]
pub fn bind_addr_from_env() -> Option<String> {
    if let Ok(addr) = std::env::var(INSPECTION_ADDR_ENV_VAR) {
        let addr = addr.trim();
        if !addr.is_empty() {
            return Some(addr.to_owned());
        }
    }
    match std::env::var(INSPECTION_ENABLE_ENV_VAR) {
        Ok(v) => {
            let v = v.trim();
            let enabled = !(v.is_empty() || v == "0" || v.eq_ignore_ascii_case("false"));
            enabled.then(|| DEFAULT_INSPECTION_ADDR.to_owned())
        }
        Err(_) => None,
    }
}
