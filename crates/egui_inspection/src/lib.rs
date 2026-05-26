#![cfg_attr(doc, doc = include_str!("../README.md"))]
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]

pub mod protocol;

pub use protocol::{
    Capabilities, Frame, FrameScreenshot, HarnessMessage, InspectorCommand, MAX_MESSAGE_BYTES,
    PROTOCOL_VERSION, PeerHello, PeerKind, SourceView, read_message, write_message,
};

/// Environment variable: when set to a local-socket name, [`InspectionPlugin::from_env`]
/// (and similar inspector-side code) connects to it. Parse it with
/// [`transport::socket_name`].
///
/// Exposed unconditionally so both ends of the connection — the plugin (on `plugin`) and
/// the inspector / MCP server — can reference the same name without pulling in the full
/// plugin impl.
pub const INSPECTION_SOCKET_ENV_VAR: &str = "EGUI_INSPECTION_SOCKET";

#[cfg(feature = "transport")]
pub mod transport;

#[cfg(feature = "png")]
mod png;

#[cfg(feature = "png")]
pub use png::encode_png;

#[cfg(feature = "plugin")]
mod plugin;

#[cfg(feature = "plugin")]
pub use plugin::{InspectionError, InspectionPlugin};
