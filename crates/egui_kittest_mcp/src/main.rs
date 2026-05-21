//! `kittest-mcp` — dual-role binary.
//!
//! Default role: **MCP server**. Speaks MCP JSON-RPC over stdio to an agent. Exposes a
//! `launch` tool that spawns a target egui kittest binary with the inspector protocol
//! pointed back at this same executable in shim mode.
//!
//! Shim role: activated when [`HANDSHAKE_ENV_VAR`] is set. The target binary's
//! [`egui_kittest::InspectorPlugin`] thinks it's talking to the regular `kittest_inspector`
//! over stdio; in reality it's talking to us, and we relay the bytes to the MCP server
//! over a unix domain socket.

mod bridge;
mod server;
mod shim;
mod tools;
mod tree;

/// Env var carrying the unix socket path the shim should connect to.
pub const HANDSHAKE_ENV_VAR: &str = "KITTEST_MCP_HANDSHAKE";

fn main() -> anyhow::Result<()> {
    if let Ok(socket_path) = std::env::var(HANDSHAKE_ENV_VAR) {
        // Shim role: relay bytes between harness stdio and the MCP server's socket.
        // No tokio runtime — keep the dependency surface tiny and the relay deterministic.
        shim::run(&socket_path)
    } else {
        // Server role: MCP over stdio.
        init_tracing();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        rt.block_on(server::run())
    }
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_env("KITTEST_MCP_LOG")
        .unwrap_or_else(|_| EnvFilter::new("kittest_mcp=info,warn"));
    // stderr only — stdout is reserved for MCP JSON-RPC traffic.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
