#![cfg_attr(doc, doc = include_str!("../README.md"))]
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]

#[cfg(feature = "protocol")]
pub mod protocol;

#[cfg(feature = "protocol")]
pub use protocol::{
    Capabilities, Frame, FrameScreenshot, HarnessMessage, InspectorCommand, MAX_MESSAGE_BYTES,
    PROTOCOL_VERSION, PeerHello, PeerKind, SourceView, read_message, write_message,
};

/// Environment variable: when set to a unix socket path, [`InspectionPlugin::from_env`]
/// (and similar inspector-side code) connects to it.
///
/// Exposed unconditionally so both ends of the connection — the plugin (on `plugin`,
/// unix) and the inspector / MCP server — can reference the same name without pulling in
/// the full plugin impl.
pub const INSPECTION_SOCKET_ENV_VAR: &str = "EGUI_INSPECTION_SOCKET";

// The plugin uses `std::os::unix::net::UnixStream` for transport, so the impl is
// unix-only. Non-unix builds with `plugin` enabled still get the protocol types.
#[cfg(all(feature = "plugin", unix))]
mod plugin;

#[cfg(all(feature = "plugin", unix))]
pub use plugin::{InspectionError, InspectionPlugin};
