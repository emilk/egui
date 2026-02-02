# eframe GUI Agent Layer Design

## Background
- `eframe` manages the lifecycle of `egui::App` for native (winit + glow/wgpu) and web (WebRunner + WebGPU/WebGL).
- This project adds a GUI agent layer to run AI/automation logic and render it with egui widgets.
- We want a single abstraction for agent logic, cross-platform input handling, and extensible UI composition.

## Goals
- **Cross-platform consistency**: one agent layer for native and wasm.
- **Composable**: wrap existing `eframe::App` without forcing new app structure.
- **Testable**: decouple agent state from UI and allow headless logic tests.
- **Observable**: expose agent events and performance signals for logging/profiling.

## Non-goals
- No DOM controls or browser-specific widget stack.
- No replacement of eframe rendering/event loops; only extend the `App` lifecycle and input hooks.

## Architecture Overview
```
winit/Web events
      │
      ▼
eframe RawInput ── raw_input_hook ──► AgentInputAdapter
      │                                      │
      ▼                                      ▼
  App::logic ─────► AgentRuntime ─────► AgentState
      │                                      │
      ▼                                      ▼
  App::ui ────────────────► AgentViews / UI components
      │
      ▼
egui output + Native/Web integration
```

### Key Components
1. **AgentRuntime**
   - Translates `AgentCommand` into `AgentUpdate`.
   - Emits control actions (`AgentUpdate::Control`) for window-level requests such as close.
   - Must be thread-safe so background work can call `Context::request_repaint`.
2. **AgentState**
   - Serializable UI state (messages, tasks) plus runtime snapshot data.
   - Stored via `Frame::storage` when persistence is enabled.
3. **AgentInputAdapter**
   - Hooks `raw_input_hook` to inject/filter events or handle shortcuts.
4. **AgentViewRegistry**
   - Maps `AgentState` to egui panels and windows; views remain pluggable.
5. **Integration Layer**
   - Native: use `ViewportCommand` for close/minimize and access GPU handles if needed.
   - Web: connect to local AgentEnvelope WebSocket bridge.

### Frame Flow
1. **Input phase**
   - eframe collects events, then calls `App::raw_input_hook`.
   - AgentInputAdapter handles shortcuts (e.g. command palette).
2. **Logic phase** (`App::logic`)
   - Drain runtime updates and update state.
   - Apply control actions (e.g. `ViewportCommand::Close`).
3. **Render phase** (`App::ui`)
   - Views read `AgentState` and render panels/widgets.
4. **Output phase**
   - Persist state and route viewport commands.

## Input and Event Strategy
- **Keyboard/mouse/touch**: handled in `AgentInputAdapter`.
- **Runtime updates**: emitted as `AgentUpdate` and applied in `logic`.
- **Control actions**: `AgentUpdate::Control` triggers `egui::ViewportCommand`.
- **Automation injection**: optional automation bridge can enqueue AccessKit action requests and text/key events via `raw_input_hook`.
- **External connectivity**: UI WebSocket bridge using `AgentEnvelope`, plus optional MCP SSE server.

## State and Persistence
- Separate UI state from long-lived agent memory.
- Use `App::save` and `Frame::storage` when persistence is enabled.
- Avoid storing large context blobs; store references instead.

## Platform Notes
- **Native**: UI WebSocket endpoint defaults to `ws://127.0.0.1:9001`, override with `AGENT_WS_URL`.
- **Native (SSE)**: MCP endpoint defaults to `http://127.0.0.1:9002/mcp` when the SSE server is enabled.
- **Web**: UI WebSocket endpoint supplied via query param `?agent_ws=ws://127.0.0.1:9001`.
- **Automation**: AccessKit-based automation is native-first; web/wasm support is limited.
- Both platforms use the same wire format and runtime abstraction.

## Risks and Mitigations
- **Long-running tasks blocking UI**: run in background and send `AgentUpdate` events.
- **State growth**: restrict serialized content and cap history.
- **Web restrictions**: only local WebSocket is supported without a gateway.
- **Selector brittleness**: prefer stable ids/labels to reduce false negatives.

## Current Implementation Checklist
- `crates/eframe_agent`: runtime, input adapter, state model, view registry, persistence helpers, and AgentEnvelope WebSocket runtime (`agent_ws` feature).
- `examples/agent_demo`: demo app that wires `eframe_agent::agent_ws` on native + web.
- `crates/eframe_agent/src/agent_ws_server.rs`: local UI WebSocket bridge for apps, demos, and tests.
- `crates/eframe_agent/src/mcp_sse_server.rs`: local MCP SSE server using rmcp streamable HTTP transport.
- `tests/agent_state.rs`: state and input adapter unit tests.
- Docs: `ARCHITECTURE.md` + `README.md` include pointers.

## MVP Plan (Closed-loop UI Automation)
Scope: basic UI automation for an MVP agent demo. UI WebSocket bridge stays single-client and local.

1. **Closed-loop verification**
   - Primary path: in-app automation bridge (AccessKit queries + injected `RawInput` events).
   - Optional: use `egui_kittest` for offline regression or snapshot testing.
2. **UI WebSocket bridge constraints (MVP)**
   - Document that the UI WebSocket bridge is single-client and local; no auth, no multi-client semantics.
   - Prefer "last connection wins" or "reject extra clients" behavior and log it clearly.
3. **Docs and examples**
   - Add a short usage snippet for embedding the MVP views/runtime in an `eframe::App`.

## Requirement-driven Closed-loop Verification (In-app Automation)
Background:
- Goal: user provides requirements, Codex implements UI + tests, and the GUI agent verifies behavior without user-written test code.
- Automation runs against the live `eframe` app, not a headless harness.
- The agent can read code and UI text, so selectors may use stable ids or visible labels/text.
- UI queries/actions are based on AccessKit tree updates from the running app.
- MCP is the control surface for running actions and reporting verdicts.

TODO:
- Add an automation bridge owned by the app to hold action queues, query requests, and the latest AccessKit tree snapshot.
- Enable AccessKit when automation is active and capture `PlatformOutput::accesskit_update` each frame to maintain a persistent UI tree.
- Resolve selectors by `id/label/text/role`; `id` maps to AccessKit `author_id` (set by the app), while `label/text` use AccessKit label/value fields.
- Inject actions into `RawInput` from the queued automation actions (AccessKit action requests for click/focus; text/key events for typing).
- Implement `JsonlDriver` backed by the automation bridge to reuse the JSONL runner for `action`/`expect` records.
- Extend the MCP SSE server with `ui_click/ui_focus/ui_type_text/ui_press_key/ui_query/run_jsonl/ui_state_snapshot` tools and cancellation support.
- Provide state snapshots by serializing `AgentState` for `state_path_*` checks.
- Report evidence on failures (last UI tree, last action, optional screenshot in the future).
- Document constraints: AccessKit automation is native-first; web/wasm support is limited and may require alternate backends.

## UI WebSocket Protocol (agent_demo)
- Transport: WebSocket text frames (JSON).
- Envelope: `AgentEnvelope` (`kind=command|update`).

Examples:
```json
{"kind":"command","command":{"SubmitPrompt":"hello"}}
```
```json
{"kind":"update","update":{"Message":{"role":"Agent","text":"hi"}}}
```

## MCP Usage
There are two distinct transports:
- UI WebSocket bridge: `AgentEnvelope` over WebSocket for egui UI (not MCP spec).
- MCP SSE server: streamable HTTP/SSE using `rmcp` (spec-compliant MCP server).
  - Optional UI automation tools (e.g. `ui_click`, `ui_query`, `run_jsonl`) are exposed via MCP when in-app automation is enabled.

Enable features:
- UI WebSocket bridge: `agent_ws` feature.
- MCP SSE server: `mcp_sse` feature (native only).

Minimal native wiring (shared agent runtime + WS + SSE):
```rust
# #[cfg(not(target_arch = "wasm32"))]
# fn run() -> Result<(), Box<dyn std::error::Error>> {
use std::sync::Arc;
use eframe_agent::{
    AgentApp, AgentRuntime, SimpleAgentRuntime,
    agent_ws::AgentWsRuntime,
    agent_bridge::AgentBridge,
    agent_ws_server::AgentWsServer,
    mcp_sse_server::McpSseServer,
};

let agent_runtime: Arc<dyn AgentRuntime> = Arc::new(SimpleAgentRuntime::new());
let bridge = Arc::new(AgentBridge::new(Arc::clone(&agent_runtime)));
let ws = AgentWsServer::spawn_with_bridge("127.0.0.1:9001", Arc::clone(&bridge))?;
let _sse = McpSseServer::spawn_default(Arc::clone(&bridge))?;

let ui_runtime: Arc<dyn AgentRuntime> = Arc::new(AgentWsRuntime::connect(ws.url()));
let app = AgentApp::builder(ui_runtime).build();
// run app with eframe::run_native(...)
# Ok(())
# }
```

Client config example (external MCP client):
```json
{
  "mcpServers": {
    "eframe-agent-sse": {
      "url": "http://127.0.0.1:9002/mcp"
    }
  }
}
```

Run:
1. Start UI: `cargo run -p agent_demo --release` (native auto-spawns a local AgentEnvelope WebSocket bridge unless `AGENT_WS_URL` is set).
2. App integration: call `eframe_agent::agent_ws::build_runtime_with_local_server()` to get a runtime plus WebSocket bridge handle.
3. Optional: embed a standalone UI WebSocket bridge via `eframe_agent::agent_ws_server::AgentWsServer::serve`.
4. Optional (MCP SSE): start `eframe_agent::mcp_sse_server::McpSseServer` on `http://127.0.0.1:9002/mcp` (feature `mcp_sse`).
5. Web: open `http://localhost:XXXX/?agent_ws=ws://127.0.0.1:9001`

References: `crates/eframe_agent/src/agent_ws_server.rs`, `crates/eframe_agent/src/mcp_sse_server.rs`, `crates/eframe_agent/src/agent_ws.rs`.
