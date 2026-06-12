//! `egui-mcp` — an MCP server that drives a live egui app.
//!
//! Speaks MCP JSON-RPC over stdio to an agent. Exposes an `attach` tool that connects to a
//! running egui app's inspection port (an app built with `egui_inspection` / eframe's
//! `inspection` feature, launched with `EGUI_INSPECTION` set), then drives it via the
//! request/response inspection protocol.

use egui_mcp::server;

fn main() -> anyhow::Result<()> {
    init_tracing();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(server::run())
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_env("EGUI_MCP_LOG")
        .unwrap_or_else(|_| EnvFilter::new("egui_mcp=info,warn"));
    // stderr only — stdout is reserved for MCP JSON-RPC traffic.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
