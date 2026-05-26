//! `kittest-mcp` — an MCP server.
//!
//! Speaks MCP JSON-RPC over stdio to an agent. Exposes a `launch` tool that binds a local
//! socket, spawns a target egui kittest binary with [`egui_inspection::INSPECTION_SOCKET_ENV_VAR`]
//! pointed at it, and accepts the harness's inbound connection — the same mechanism as
//! `attach` for live apps. The harness's [`egui_kittest::InspectorPlugin`] dials the socket
//! directly, so there's no relaying middleman.

mod bridge;
mod server;
mod tools;
mod tree;

fn main() -> anyhow::Result<()> {
    init_tracing();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(server::run())
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
