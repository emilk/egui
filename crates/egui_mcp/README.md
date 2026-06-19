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
settle. Everything works in one shared logical-point coordinate frame, so a node's `bounds`
center is exactly where to `click`.

## How it fits together

```
agent (Claude / Codex)  ──MCP/stdio──▶  egui-mcp  ──TCP──▶  your egui app
                                                            (EGUI_INSPECTION set)
```

There are two pieces to set up: enable inspection **in your app**, then point your **agent** at
the `egui-mcp` server.

## 1. Enable inspection in your app

Enable eframe's `inspection` feature and launch the app with the `EGUI_INSPECTION` env var set:

```sh
EGUI_INSPECTION=1 cargo run --features inspection   # binds 127.0.0.1:5719
```

When the variable is unset or falsy (`0` / `false`), inspection is completely off
(production-safe). See [`egui_inspection`](https://crates.io/crates/egui_inspection) for details,
including how to expose it across the network.

## 2. Install the server

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

With the app running (step 1) and the agent configured (step 3), ask the agent to `attach`, then
drive the app. A typical loop is **observe → act → verify**: `query_tree` or `screenshot` to see
what's there, `click` / `type_text` / etc. to act, then re-query to confirm.

`attach` defaults to `127.0.0.1:5719` (the `egui_inspection` default port); pass `host` / `port`
to reach an app bound elsewhere.

> **Screenshots need a visible window.** Reading the tree and injecting input work even while the
> app is in the background, but capturing a screenshot requires a rendered frame — which the OS
> won't produce for a fully-occluded or minimized window (notably on macOS). Bring the window to
> the foreground to capture it.
