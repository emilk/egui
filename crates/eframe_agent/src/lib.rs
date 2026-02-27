//! Helper crate for building GUI-driven agents on top of [`eframe`](https://github.com/emilk/egui/tree/main/crates/eframe).
//!
//! The main entry point is [`AgentApp`], which wires a runtime, shared agent state, and a registry
//! of egui views into a regular `eframe::App`.
#![warn(missing_docs, clippy::all, rust_2018_idioms)]

/// Shared bridge for the AgentEnvelope WebSocket bridge and MCP SSE server.
#[cfg(all(
    any(feature = "agent_ws", feature = "mcp_sse"),
    not(target_arch = "wasm32")
))]
pub mod agent_bridge;

/// AgentEnvelope WebSocket runtime for native/web agent backends.
#[cfg(feature = "agent_ws")]
pub mod agent_ws;

/// Local UI WebSocket bridge for demos/tests (AgentEnvelope, not MCP spec).
#[cfg(all(feature = "agent_ws", not(target_arch = "wasm32")))]
pub mod agent_ws_server;

/// eframe `App` wrapper and builder.
#[cfg(feature = "ui")]
pub mod app;

/// Live UI automation bridge for in-app verification.
#[cfg(all(feature = "ui", feature = "jsonl"))]
pub mod automation;

/// Persistence helpers for storing agent state.
#[cfg(feature = "ui")]
pub mod bridge;

/// Input adapter for agent shortcuts and synthetic events.
#[cfg(feature = "ui")]
pub mod input;

/// JSONL protocol and runner for closed-loop automation.
#[cfg(feature = "jsonl")]
pub mod jsonl;

/// Local MCP SSE server (rmcp streamable HTTP transport).
#[cfg(all(feature = "mcp_sse", not(target_arch = "wasm32")))]
pub mod mcp_sse_server;

/// Runtime traits and default runtime implementations.
pub mod runtime;

/// Serializable agent state models.
pub mod state;

/// Reusable egui views and view registry.
#[cfg(feature = "ui")]
pub mod views;

#[cfg(feature = "ui")]
pub use crate::{
    app::{AgentApp, AgentAppBuilder},
    bridge::{STORAGE_KEY, load_state_from_storage, save_state_to_storage},
    input::{AgentInputAdapter, InputAction},
    views::{AgentView, AgentViewRegistry, ToolLogView},
};
pub use crate::{
    runtime::{
        AgentCommand, AgentEnvelope, AgentRuntime, AgentUpdate, ControlAction, MessageRole,
        SimpleAgentRuntime,
    },
    state::{AgentState, AgentTaskState, TaskStatus},
};

#[cfg(feature = "jsonl")]
pub use crate::jsonl::{
    ActionKind, ActionRecord, Check, ExpectRecord, JsonlDriver, JsonlEntry, JsonlParseError,
    JsonlRunError, JsonlRunner, JsonlRunnerOptions, JsonlScript, KeyModifier, MetaRecord,
    RecordMeta, ScriptRecord, Target, TargetBy, parse_jsonl_script,
};

#[cfg(all(feature = "ui", feature = "jsonl"))]
pub use crate::automation::{AutomationBridge, AutomationDriver, AutomationError};

#[cfg(all(
    any(feature = "agent_ws", feature = "mcp_sse"),
    not(target_arch = "wasm32")
))]
pub use crate::agent_bridge::AgentBridge;
#[cfg(all(feature = "agent_ws", not(target_arch = "wasm32")))]
pub use crate::agent_ws::build_runtime_with_local_server as build_agent_ws_runtime_with_local_server;
#[cfg(feature = "agent_ws")]
pub use crate::agent_ws::{AgentWsRuntime, build_runtime as build_agent_ws_runtime};
#[cfg(all(feature = "agent_ws", not(target_arch = "wasm32")))]
pub use crate::agent_ws_server::{AgentWsServer, AgentWsServerHandle, AgentWsServerResult};
#[cfg(all(feature = "mcp_sse", not(target_arch = "wasm32")))]
pub use crate::mcp_sse_server::{
    DEFAULT_MCP_SSE_ADDR, DEFAULT_MCP_SSE_PATH, McpSseServer, McpSseServerHandle,
    McpSseServerResult,
};
