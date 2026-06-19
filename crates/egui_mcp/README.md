# egui_mcp

[![Latest version](https://img.shields.io/crates/v/egui_mcp.svg)](https://crates.io/crates/egui_mcp)
[![Documentation](https://docs.rs/egui_mcp/badge.svg)](https://docs.rs/egui_mcp)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

An [MCP](https://modelcontextprotocol.io) server that lets an AI agent (Claude, Codex, …) drive
a live [egui](https://github.com/emilk/egui) app.

`egui-mcp` connects to a running app over the
[`egui_inspection`](https://crates.io/crates/egui_inspection) protocol and exposes it as MCP
tools: read the **AccessKit widget tree** (`query_tree` / `get_node`), **click / type / scroll /
drag / press keys**, take a **screenshot**, `resize` the window, and `wait_for` async UI to
settle. 

## 1. Enable inspection in your app

Enable eframe's `inspection` feature and launch the app with the `EGUI_INSPECTION` env var set:

```sh
EGUI_INSPECTION=1 cargo run --features eframe/inspection   # binds 127.0.0.1:5719
```

When the variable is unset or falsy (`0` / `false`), inspection is completely off. See [`egui_inspection`](https://crates.io/crates/egui_inspection) for details,
including how to expose it across the network.

## 2. Install the mcp

```sh
cargo install --git https://github.com/emilk/egui egui_mcp
```

This installs the `egui-mcp` binary onto your `PATH`.

## 3. Configure your agent

**Claude Code** — register the server with one command:

```sh
claude mcp add egui egui-mcp
```

or add it to your MCP config (`~/.claude.json` or `.mcp.json`) manually:

```json
{
  "mcpServers": {
    "egui": {
      "command": "egui-mcp"
    }
  }
}
```

**Codex** — add it to `~/.codex/config.toml`:

```toml
[mcp_servers.egui]
command = "egui-mcp"
args = []
```

## Using it

With the app running and the agent configured, ask the agent to `attach`, then
drive the app. You could ask it to e.g. reproduce a bug, verify a new feature or just randomly use, 
test and explore your app.

> **Screenshots need a visible window.** Reading the tree and injecting input work even while the
> app is in the background, but capturing a screenshot requires a rendered frame — which the OS
> won't produce for a fully-occluded or minimized window (notably on macOS). Bring the window to
> the foreground to capture it.
