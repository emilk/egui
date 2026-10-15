# egui_inspection

[![Latest version](https://img.shields.io/crates/v/egui_inspection.svg)](https://crates.io/crates/egui_inspection)
[![Documentation](https://docs.rs/egui_inspection/badge.svg)](https://docs.rs/egui_inspection)

Inspection for [egui](https://github.com/emilk/egui) apps.

`egui_inspection` defines a wire protocol and an [`egui::Plugin`] (`InspectionPlugin`) that
serves it. An external inspector — such as the
[`egui_mcp`](https://crates.io/crates/egui_mcp) MCP server — connects and can:

- read the app's **AccessKit tree** (`GetTree`),
- inject **input events** (`HandleEvents` — clicks, typing, scrolling, …),
- capture a **screenshot** on request (`Screenshot`),
- resize the window (`Resize`).

The protocol is strictly request → response, which maps cleanly onto both a TCP socket and a
unary RPC (so the same machinery can be tunnelled over another transport).

> **Screenshots need a visible window.** Reading the tree and injecting input work even while
> the app is in the background, but capturing a screenshot requires a rendered frame — which
> the OS won't produce for a fully-occluded or minimized window (notably on macOS, where the
> GPU surface isn't available). Bring the window to the foreground to capture it; the
> `Screenshot` request times out otherwise.

## What it's for

`egui_inspection` is the shared foundation for tools that observe or drive an egui app from
the outside. Anything that speaks the protocol (over TCP, or another transport) can be a
consumer:

- **[`egui_mcp`](https://crates.io/crates/egui_mcp)** — an MCP server that exposes the app to
  AI agents and other tooling: query the widget tree, click / type / scroll, take screenshots.
- **An egui inspector GUI** *(planned)* — a visual debugger that connects to a running app to
  browse its widget tree and drive it interactively.
- **Test inspection & frame streaming** *(planned)* — attach to `egui_kittest` tests, and
  stream frames for live mirroring of an app's window.

## Enabling it in an eframe app

Enable eframe's `inspection` feature, then set the `EGUI_INSPECTION` env var at runtime. It's
either truthy, falsy, or a bind address:

```sh
EGUI_INSPECTION=1 cargo run --features inspection            # binds 127.0.0.1:5719
EGUI_INSPECTION=0.0.0.0:5719 cargo run --features inspection # reachable across devices
```

When the variable is unset or falsy (`0` / `false`), inspection is completely off
(production-safe).

> ⚠️ Binding a non-loopback address exposes full control of the app — and its screenshots —
> to anyone who can reach the port, with **no authentication**. A warning is logged when you
> do so. Prefer loopback + an SSH tunnel for remote debugging.

## Using the plugin directly

```rust,no_run
# let ctx = egui::Context::default();
ctx.add_plugin(egui_inspection::InspectionPlugin::new(Some("my app".to_owned())));
egui_inspection::serve(&ctx, "127.0.0.1:5719").unwrap();
```
