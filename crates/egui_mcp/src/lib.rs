//! `egui_mcp` — an MCP server that drives a live egui app over the request/response
//! [`egui_inspection`] protocol, plus the building blocks to embed or extend it.
//!
//! The `egui-mcp` binary (`src/main.rs`) is a thin wrapper around [`server::run`], which serves
//! [`Server`] — the TCP `attach` / `disconnect` lifecycle plus the egui UI tools.
//!
//! The library surface exists so another MCP server can keep *its own* connection logic while
//! reusing the egui UI/inspection tool set:
//!
//! - [`Bridge::from_transport`] builds a bridge over any async byte transport.
//! - [`AppState::install_bridge`] injects that bridge so the UI tools drive it.
//! - [`UiServer`] exposes those tools: embed it and merge [`UiServer::tools`] into your own
//!   `list_tools`, delegating non-connection calls to [`UiServer::dispatch`].

pub mod bridge;
pub mod server;
pub mod tools;
pub mod tree;

pub use bridge::{Bridge, PeerInfo, TreeSnapshot};
pub use tools::{AppState, Server, UiServer};
