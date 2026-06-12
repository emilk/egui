//! MCP server entry point, built on the official `rmcp` SDK over stdio.
//!
//! [`run`] constructs a [`crate::tools::Server`] (which derives its tool router via
//! `#[tool_router]`) and serves it on `(stdin, stdout)`. Returns once the client closes
//! the connection (EOF on stdin) or the runtime is shut down.

use rmcp::{ServiceExt as _, transport};

use crate::tools::Server;

/// Serve the MCP tool set on stdio until the client disconnects.
///
/// # Errors
/// If the stdio transport fails to start or the connection ends in error.
pub async fn run() -> anyhow::Result<()> {
    let server = Server::new();
    let running = server.serve(transport::stdio()).await?;
    let _reason = running.waiting().await?;
    Ok(())
}
