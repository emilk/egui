//! `egui_mcp` — an MCP server that drives a live egui app over the request/response
//! [`egui_inspection`] protocol, plus the building blocks to embed or extend it.
//!
//! The `egui-mcp` binary (`src/main.rs`) is a thin wrapper around [`server::run`], which serves
//! [`Server`] — the TCP `attach` / `disconnect` lifecycle plus the egui UI tools.
//!
//! The library surface exists so another MCP server can keep *its own* connection logic while
//! reusing the egui UI/inspection tool set:
//!
//! - [`Bridge::with_transport`] builds a bridge over any [`Transport`] (e.g. a host's own
//!   request/response channel); [`FramedTransport`] is the built-in byte-stream implementation.
//! - [`UiServer::new`] wraps that bridge as a UI server. Pair it with the router from
//!   [`UiServer::router`]: merge `router.list_all()` into your own `list_tools`, and delegate
//!   non-connection calls to [`UiServer::dispatch`].

pub mod bridge;
pub mod server;
pub mod tools;
pub mod tree;

pub use bridge::{BoxFuture, Bridge, FramedTransport, PeerInfo, Transport, TreeSnapshot};
pub use tools::{Server, UiServer};
