use std::{
    io,
    net::SocketAddr,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use axum::Router;
use log::{info, warn};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use schemars::JsonSchema;
use tokio_util::sync::CancellationToken;

#[cfg(all(feature = "jsonl", feature = "ui"))]
use crate::automation::AutomationBridge;
#[cfg(not(all(feature = "jsonl", feature = "ui")))]
type AutomationBridge = ();
#[cfg(feature = "jsonl")]
use crate::jsonl::{JsonlRunner, JsonlRunnerOptions, JsonlScript, KeyModifier, Target};
use crate::{
    agent_bridge::AgentBridge,
    runtime::{AgentCommand, AgentUpdate, MessageRole},
};
#[cfg(all(feature = "jsonl", feature = "ui"))]
use serde_json::json;

/// Error type returned by MCP SSE server helpers.
pub type McpSseServerResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Default address used by the local MCP SSE server.
pub const DEFAULT_MCP_SSE_ADDR: &str = "127.0.0.1:9002";

/// Default route path for the MCP SSE endpoint.
pub const DEFAULT_MCP_SSE_PATH: &str = "/mcp";

const SERVER_READY_TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_REPLY_TIMEOUT: Duration = Duration::from_secs(10);

/// Handle used to manage a background MCP SSE server thread.
pub struct McpSseServerHandle {
    addr: SocketAddr,
    url: String,
    cancel: CancellationToken,
    join: Option<thread::JoinHandle<()>>,
}

impl McpSseServerHandle {
    /// The HTTP URL clients can connect to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The socket address the server is bound to.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Signal the server to stop and wait for the thread to exit.
    pub fn stop(mut self) {
        self.shutdown();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }

    /// Signal the server to stop without waiting for the thread to exit.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }
}

impl Drop for McpSseServerHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// MCP SSE server powered by the rmcp streamable HTTP transport.
pub struct McpSseServer;

impl McpSseServer {
    /// Spawn a background MCP SSE server on the provided socket address.
    pub fn spawn(addr: &str, bridge: Arc<AgentBridge>) -> McpSseServerResult<McpSseServerHandle> {
        Self::spawn_inner(addr, bridge, None)
    }

    /// Spawn a background MCP SSE server on the default address.
    pub fn spawn_default(bridge: Arc<AgentBridge>) -> McpSseServerResult<McpSseServerHandle> {
        Self::spawn(DEFAULT_MCP_SSE_ADDR, bridge)
    }

    #[cfg(all(feature = "jsonl", feature = "ui"))]
    /// Spawn a background MCP SSE server with live UI automation enabled.
    pub fn spawn_with_automation(
        addr: &str,
        bridge: Arc<AgentBridge>,
        automation: AutomationBridge,
    ) -> McpSseServerResult<McpSseServerHandle> {
        Self::spawn_inner(addr, bridge, Some(automation))
    }

    #[cfg(all(feature = "jsonl", feature = "ui"))]
    /// Spawn a background MCP SSE server on the default address with automation.
    pub fn spawn_default_with_automation(
        bridge: Arc<AgentBridge>,
        automation: AutomationBridge,
    ) -> McpSseServerResult<McpSseServerHandle> {
        Self::spawn_with_automation(DEFAULT_MCP_SSE_ADDR, bridge, automation)
    }

    fn spawn_inner(
        addr: &str,
        bridge: Arc<AgentBridge>,
        automation: Option<AutomationBridge>,
    ) -> McpSseServerResult<McpSseServerHandle> {
        let addr = addr.to_string();
        let cancel = CancellationToken::new();
        let cancel_thread = cancel.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<SocketAddr, String>>();

        let join = thread::Builder::new()
            .name("eframe_agent_mcp_sse_server".to_string())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(err) => {
                        let _ = ready_tx.send(Err(err.to_string()));
                        return;
                    }
                };

                runtime.block_on(async move {
                    let listener = match tokio::net::TcpListener::bind(&addr).await {
                        Ok(listener) => listener,
                        Err(err) => {
                            let _ = ready_tx.send(Err(err.to_string()));
                            return;
                        }
                    };
                    let bound = match listener.local_addr() {
                        Ok(bound) => bound,
                        Err(err) => {
                            let _ = ready_tx.send(Err(err.to_string()));
                            return;
                        }
                    };
                    let _ = ready_tx.send(Ok(bound));

                    let service = StreamableHttpService::new(
                        {
                            let bridge = Arc::clone(&bridge);
                            let automation = automation.clone();
                            move || {
                                Ok(AgentMcpService::new(
                                    Arc::clone(&bridge),
                                    automation.clone(),
                                ))
                            }
                        },
                        LocalSessionManager::default().into(),
                        StreamableHttpServerConfig {
                            cancellation_token: cancel_thread.child_token(),
                            ..Default::default()
                        },
                    );

                    let router = Router::new().nest_service(DEFAULT_MCP_SSE_PATH, service);
                    info!("agent_mcp_sse listening on http://{bound}{DEFAULT_MCP_SSE_PATH}");
                    let shutdown = cancel_thread.clone();
                    let result = axum::serve(listener, router)
                        .with_graceful_shutdown(async move {
                            shutdown.cancelled().await;
                        })
                        .await;
                    if let Err(err) = result {
                        warn!("MCP SSE server stopped: {err}");
                    }
                });
            })
            .expect("failed to spawn MCP SSE server thread");

        let addr = match ready_rx.recv_timeout(SERVER_READY_TIMEOUT) {
            Ok(Ok(addr)) => addr,
            Ok(Err(message)) => {
                return Err(Box::new(io::Error::other(message)));
            }
            Err(err) => {
                return Err(Box::new(io::Error::new(
                    io::ErrorKind::TimedOut,
                    err.to_string(),
                )));
            }
        };

        let url = format!("http://{addr}{DEFAULT_MCP_SSE_PATH}");
        Ok(McpSseServerHandle {
            addr,
            url,
            cancel,
            join: Some(join),
        })
    }
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
struct SubmitPromptArgs {
    prompt: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[cfg(feature = "jsonl")]
#[derive(Debug, serde::Deserialize, JsonSchema)]
struct UiTargetArgs {
    target: Target,
}

#[cfg(feature = "jsonl")]
#[derive(Debug, serde::Deserialize, JsonSchema)]
struct UiTypeTextArgs {
    target: Target,
    text: String,
}

#[cfg(feature = "jsonl")]
#[derive(Debug, serde::Deserialize, JsonSchema)]
struct UiPressKeyArgs {
    key: String,
    #[serde(default)]
    modifiers: Vec<KeyModifier>,
}

#[cfg(feature = "jsonl")]
#[derive(Debug, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum UiQueryKind {
    Exists,
    TextContains,
}

#[cfg(feature = "jsonl")]
#[derive(Debug, serde::Deserialize, JsonSchema)]
struct UiQueryArgs {
    kind: UiQueryKind,
    #[serde(default)]
    target: Option<Target>,
    #[serde(default)]
    value: Option<String>,
}

#[cfg(feature = "jsonl")]
#[derive(Debug, serde::Deserialize, JsonSchema)]
struct UiScreenshotArgs {
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[cfg(feature = "jsonl")]
#[derive(Debug, serde::Deserialize, JsonSchema)]
struct RunJsonlArgs {
    script: String,
    #[serde(default)]
    default_within_ms: Option<u64>,
    #[serde(default)]
    poll_interval_ms: Option<u64>,
}

#[cfg(feature = "jsonl")]
fn format_target(target: &Target) -> String {
    format!("{:?}={}", target.by, target.value)
}

#[cfg(feature = "jsonl")]
fn format_modifiers(modifiers: &[KeyModifier]) -> String {
    if modifiers.is_empty() {
        return "none".to_string();
    }
    modifiers
        .iter()
        .map(|modifier| format!("{modifier:?}"))
        .collect::<Vec<_>>()
        .join("+")
}

#[derive(Clone)]
struct AgentMcpService {
    bridge: Arc<AgentBridge>,
    tool_router: ToolRouter<Self>,
    automation: Option<AutomationBridge>,
}

#[tool_router]
impl AgentMcpService {
    fn new(bridge: Arc<AgentBridge>, automation: Option<AutomationBridge>) -> Self {
        Self {
            bridge,
            tool_router: Self::tool_router(),
            automation,
        }
    }

    fn log_tool(&self, message: impl Into<String>) {
        self.bridge.broadcast_update(AgentUpdate::ui_log(message));
    }

    #[tool(description = "Submit a prompt to the GUI agent and return the next agent reply.")]
    async fn submit_prompt(
        &self,
        Parameters(args): Parameters<SubmitPromptArgs>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_millis(
            args.timeout_ms
                .unwrap_or_else(|| DEFAULT_REPLY_TIMEOUT.as_millis() as u64),
        );
        let mut updates = self.bridge.subscribe();
        let prompt = args.prompt;
        self.log_tool(format!("submit_prompt: {prompt}"));
        self.bridge
            .submit_command(AgentCommand::SubmitPrompt(prompt));

        let reply =
            tokio::task::spawn_blocking(move || wait_for_agent_reply(&mut updates, timeout))
                .await
                .map_err(|err| {
                    McpError::internal_error(format!("agent wait task failed: {err}"), None)
                })?
                .map_err(|err| McpError::internal_error(err, None))?;

        Ok(CallToolResult::success(vec![Content::text(reply)]))
    }

    #[tool(description = "Request that the active agent task be cancelled.")]
    async fn cancel_active_task(&self) -> Result<CallToolResult, McpError> {
        self.log_tool("cancel_active_task");
        self.bridge.submit_command(AgentCommand::CancelActiveTask);
        Ok(CallToolResult::success(vec![Content::text(
            "cancel requested",
        )]))
    }

    #[tool(description = "Clear the agent conversation history.")]
    async fn clear_history(&self) -> Result<CallToolResult, McpError> {
        self.log_tool("clear_history");
        self.bridge.submit_command(AgentCommand::ClearHistory);
        Ok(CallToolResult::success(vec![Content::text(
            "history cleared",
        )]))
    }

    #[cfg(all(feature = "jsonl", feature = "ui"))]
    fn automation_ref(&self) -> Result<&AutomationBridge, McpError> {
        self.automation
            .as_ref()
            .ok_or_else(|| McpError::internal_error("automation bridge not configured", None))
    }

    #[cfg(all(feature = "jsonl", feature = "ui"))]
    #[tool(description = "Click a UI target using the live automation bridge.")]
    async fn ui_click(
        &self,
        Parameters(args): Parameters<UiTargetArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.automation_ref().and_then(|automation| {
            automation
                .click(&args.target)
                .map_err(|err| McpError::internal_error(err.to_string(), None))
        })?;
        self.log_tool(format!("ui_click: {}", format_target(&args.target)));
        Ok(CallToolResult::success(vec![Content::text("queued")]))
    }

    #[cfg(all(feature = "jsonl", feature = "ui"))]
    #[tool(description = "Focus a UI target using the live automation bridge.")]
    async fn ui_focus(
        &self,
        Parameters(args): Parameters<UiTargetArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.automation_ref().and_then(|automation| {
            automation
                .focus(&args.target)
                .map_err(|err| McpError::internal_error(err.to_string(), None))
        })?;
        self.log_tool(format!("ui_focus: {}", format_target(&args.target)));
        Ok(CallToolResult::success(vec![Content::text("queued")]))
    }

    #[cfg(all(feature = "jsonl", feature = "ui"))]
    #[tool(description = "Type text into a UI target using the live automation bridge.")]
    async fn ui_type_text(
        &self,
        Parameters(args): Parameters<UiTypeTextArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.automation_ref().and_then(|automation| {
            automation
                .type_text(&args.target, &args.text)
                .map_err(|err| McpError::internal_error(err.to_string(), None))
        })?;
        self.log_tool(format!(
            "ui_type_text: {} text={}",
            format_target(&args.target),
            args.text
        ));
        Ok(CallToolResult::success(vec![Content::text("queued")]))
    }

    #[cfg(all(feature = "jsonl", feature = "ui"))]
    #[tool(description = "Press a key with modifiers using the live automation bridge.")]
    async fn ui_press_key(
        &self,
        Parameters(args): Parameters<UiPressKeyArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.automation_ref().and_then(|automation| {
            automation
                .press_key(&args.key, &args.modifiers)
                .map_err(|err| McpError::internal_error(err.to_string(), None))
        })?;
        self.log_tool(format!(
            "ui_press_key: key={} modifiers={}",
            args.key,
            format_modifiers(&args.modifiers)
        ));
        Ok(CallToolResult::success(vec![Content::text("queued")]))
    }

    #[cfg(all(feature = "jsonl", feature = "ui"))]
    #[tool(description = "Query the live UI tree for existence or text.")]
    async fn ui_query(
        &self,
        Parameters(args): Parameters<UiQueryArgs>,
    ) -> Result<CallToolResult, McpError> {
        let automation = self.automation_ref()?;
        let matched = match args.kind {
            UiQueryKind::Exists => {
                let target = args.target.ok_or_else(|| {
                    McpError::internal_error("missing target for exists query", None)
                })?;
                automation
                    .ui_exists(&target)
                    .map_err(|err| McpError::internal_error(err.to_string(), None))?
            }
            UiQueryKind::TextContains => {
                let value = args.value.ok_or_else(|| {
                    McpError::internal_error("missing value for text_contains query", None)
                })?;
                automation
                    .ui_text_contains(&value)
                    .map_err(|err| McpError::internal_error(err.to_string(), None))?
            }
        };

        Ok(CallToolResult::success(vec![Content::text(
            matched.to_string(),
        )]))
    }

    #[cfg(all(feature = "jsonl", feature = "ui"))]
    #[tool(description = "Capture a screenshot of the current UI and return metadata.")]
    async fn ui_screenshot(
        &self,
        Parameters(args): Parameters<UiScreenshotArgs>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_millis(args.timeout_ms.unwrap_or(2_000));
        let automation = self.automation_ref()?.clone();
        let screenshot =
            tokio::task::spawn_blocking(move || automation.request_screenshot(timeout))
                .await
                .map_err(|err| {
                    McpError::internal_error(format!("screenshot task failed: {err}"), None)
                })?
                .map_err(|err| McpError::internal_error(err.to_string(), None))?;
        self.log_tool(format!(
            "ui_screenshot: {}x{} hash={}",
            screenshot.width, screenshot.height, screenshot.hash
        ));
        let payload = json!({
            "width": screenshot.width,
            "height": screenshot.height,
            "hash": screenshot.hash,
        })
        .to_string();
        Ok(CallToolResult::success(vec![Content::text(payload)]))
    }

    #[cfg(all(feature = "jsonl", feature = "ui"))]
    #[tool(description = "Return a JSON state snapshot if supported.")]
    async fn ui_state_snapshot(&self) -> Result<CallToolResult, McpError> {
        let snapshot = self
            .automation_ref()?
            .state_snapshot()
            .map_err(|err| McpError::internal_error(err.to_string(), None))?;
        let payload = snapshot
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string());
        Ok(CallToolResult::success(vec![Content::text(payload)]))
    }

    #[cfg(all(feature = "jsonl", feature = "ui"))]
    #[tool(description = "Run a JSONL script against the live UI.")]
    async fn run_jsonl(
        &self,
        Parameters(args): Parameters<RunJsonlArgs>,
    ) -> Result<CallToolResult, McpError> {
        let script = JsonlScript::parse(&args.script)
            .map_err(|err| McpError::internal_error(err.to_string(), None))?;
        let options = JsonlRunnerOptions {
            default_within_ms: args.default_within_ms.unwrap_or(3_000),
            poll_interval_ms: args.poll_interval_ms.unwrap_or(50),
        };
        let runner = JsonlRunner::new(options);
        let automation = self.automation_ref()?.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut driver = automation.driver();
            runner.run_script(&script, &mut driver)
        })
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?;

        match result {
            Ok(()) => {
                let lines = args.script.lines().count();
                self.log_tool(format!("run_jsonl: lines={lines}"));
                Ok(CallToolResult::success(vec![Content::text("ok")]))
            }
            Err(err) => Err(McpError::internal_error(err.to_string(), None)),
        }
    }
}

#[tool_handler]
impl ServerHandler for AgentMcpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Expose a submit_prompt tool that forwards prompts into the GUI agent runtime."
                    .into(),
            ),
        }
    }
}

fn wait_for_agent_reply(
    updates: &mut std::sync::mpsc::Receiver<AgentUpdate>,
    timeout: Duration,
) -> Result<String, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("agent response timed out".into());
        }

        match updates.recv_timeout(remaining) {
            Ok(AgentUpdate::Message {
                role: MessageRole::Agent,
                text,
            }) => return Ok(text),
            Ok(_) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                return Err("agent response timed out".into());
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err("agent update channel closed".into());
            }
        }
    }
}

#[cfg(all(test, feature = "mcp_sse", feature = "ui", feature = "jsonl"))]
mod tests {
    use std::borrow::Cow;
    use std::{
        collections::HashSet,
        future::Future,
        time::{Duration, Instant},
    };

    use rmcp::{
        RoleClient, ServiceExt,
        model::{CallToolRequestParams, CallToolResult, ClientInfo},
        service::Peer,
        transport::StreamableHttpClientTransport,
    };
    use serde_json::{Value, json};
    use tokio::time::sleep;

    fn target_id(value: &str) -> Value {
        json!({
            "by": "id",
            "value": value,
        })
    }

    fn content_text(result: &CallToolResult) -> String {
        result
            .content
            .first()
            .and_then(|content| content.as_text())
            .map(|text| text.text.clone())
            .unwrap_or_default()
    }

    fn bool_from_result(result: &CallToolResult) -> bool {
        content_text(result)
            .parse::<bool>()
            .expect("expected boolean content")
    }

    fn format_ui_log(snapshot: &Value) -> String {
        if !snapshot.is_object() {
            return format!("snapshot={snapshot}");
        }
        let mut lines = Vec::new();
        let draft = snapshot
            .get("draft_prompt")
            .and_then(|value| value.as_str());
        lines.push(format!("draft_prompt={draft:?}"));
        let ui_log = snapshot.get("ui_log").and_then(|value| value.as_array());
        if let Some(ui_log) = ui_log {
            for entry in ui_log {
                let text = entry.as_str().unwrap_or("");
                lines.push(format!("ui_log {text}"));
            }
        } else {
            lines.push("ui_log=<missing>".to_string());
        }
        let messages = snapshot.get("messages").and_then(|value| value.as_array());
        if let Some(messages) = messages {
            for message in messages {
                let role = message
                    .get("role")
                    .and_then(|value| value.as_str())
                    .unwrap_or("<unknown>");
                let text = message
                    .get("text")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                lines.push(format!("message[{role}] {text}"));
            }
        } else {
            lines.push("messages=<missing>".to_string());
        }
        let tasks = snapshot.get("tasks").and_then(|value| value.as_array());
        if let Some(tasks) = tasks {
            for task in tasks {
                let id = task
                    .get("id")
                    .and_then(|value| value.as_u64())
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let label = task
                    .get("label")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                let status = task
                    .get("status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("<unknown>");
                lines.push(format!("task[{id}] {status} {label}"));
            }
        } else {
            lines.push("tasks=<missing>".to_string());
        }
        lines.join("\n")
    }

    async fn log_ui_text(peer: &Peer<RoleClient>, label: &str) {
        let snapshot = snapshot_value(peer).await;
        let log = format_ui_log(&snapshot);
        eprintln!("[ui_log:{label}]\n{log}");
    }

    async fn call_tool(
        peer: &Peer<RoleClient>,
        name: &str,
        arguments: Option<Value>,
    ) -> CallToolResult {
        let arguments = arguments.map(|value| {
            value
                .as_object()
                .cloned()
                .expect("tool arguments must be an object")
        });
        peer.call_tool(CallToolRequestParams {
            meta: None,
            name: Cow::Owned(name.to_string()),
            arguments,
            task: None,
        })
        .await
        .unwrap_or_else(|err| panic!("call_tool {name} failed: {err}"))
    }

    struct WaitForStatus {
        done: bool,
        detail: Option<String>,
    }

    async fn wait_for<F, Fut>(label: &str, timeout: Duration, mut check: F)
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = WaitForStatus>,
    {
        let start = Instant::now();
        let deadline = start + timeout;
        let mut last_log = start;
        let mut last_detail: Option<String> = None;
        loop {
            let status = check().await;
            if status.done {
                return;
            }
            if let Some(detail) = status.detail {
                last_detail = Some(detail);
            }
            let now = Instant::now();
            if now.duration_since(last_log) >= Duration::from_secs(1) {
                let detail = last_detail.as_deref().unwrap_or("<no detail>");
                eprintln!(
                    "[wait_for] {label} elapsed={:?} detail={detail}",
                    now - start
                );
                last_log = now;
            }
            if now >= deadline {
                let detail = last_detail.as_deref().unwrap_or("<no detail>");
                panic!("condition not met within {timeout:?} ({label}). last_detail={detail}");
            }
            sleep(Duration::from_millis(10)).await;
        }
    }

    async fn snapshot_value(peer: &Peer<RoleClient>) -> Value {
        let snapshot = call_tool(peer, "ui_state_snapshot", None).await;
        serde_json::from_str(&content_text(&snapshot)).expect("snapshot json")
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "requires a running agent_demo MCP SSE server"]
    async fn mcp_tools_closed_loop_agent_demo() {
        let url = std::env::var("MCP_AGENT_DEMO_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:9002/mcp".to_string());

        let connect_deadline = Instant::now() + Duration::from_secs(10);
        let mut last_error;
        let mut client = loop {
            let transport = StreamableHttpClientTransport::from_uri(url.as_str());
            let client_info = ClientInfo::default();
            match client_info.serve(transport).await {
                Ok(client) => break client,
                Err(err) => {
                    last_error = Some(err.to_string());
                    if Instant::now() >= connect_deadline {
                        panic!(
                            "failed to connect to {url}: {}. Start agent_demo with `cargo run -p agent_demo --features mcp_sse`",
                            last_error.unwrap_or_else(|| "unknown error".into())
                        );
                    }
                    sleep(Duration::from_millis(200)).await;
                }
            }
        };
        let peer = client.peer().clone();

        let tools = peer.list_all_tools().await.expect("list tools");
        let tool_names: HashSet<String> = tools
            .iter()
            .map(|tool| tool.name.as_ref().to_string())
            .collect();
        for name in [
            "submit_prompt",
            "cancel_active_task",
            "clear_history",
            "ui_click",
            "ui_focus",
            "ui_type_text",
            "ui_press_key",
            "ui_query",
            "ui_screenshot",
            "ui_state_snapshot",
            "run_jsonl",
        ] {
            assert!(tool_names.contains(name), "tool {name} missing");
        }

        wait_for("prompt_input visible", Duration::from_secs(10), || async {
            let palette_open = call_tool(
                &peer,
                "ui_query",
                Some(json!({
                    "kind": "exists",
                    "target": target_id("command_palette_input"),
                })),
            )
            .await;
            let palette_open = bool_from_result(&palette_open);
            if palette_open {
                let _ = call_tool(
                    &peer,
                    "ui_press_key",
                    Some(json!({
                        "key": "K",
                        "modifiers": ["command"],
                    })),
                )
                .await;
            }
            let exists = call_tool(
                &peer,
                "ui_query",
                Some(json!({
                    "kind": "exists",
                    "target": target_id("prompt_input"),
                })),
            )
            .await;
            let prompt_exists = bool_from_result(&exists);
            WaitForStatus {
                done: !palette_open && prompt_exists,
                detail: Some(format!(
                    "palette_open={palette_open}, prompt_exists={prompt_exists}"
                )),
            }
        })
        .await;
        log_ui_text(&peer, "prompt_input ready").await;

        let clicked = call_tool(
            &peer,
            "ui_click",
            Some(json!({
                "target": target_id("prompt_input"),
            })),
        )
        .await;
        assert_eq!(content_text(&clicked), "queued");

        let focused = call_tool(
            &peer,
            "ui_focus",
            Some(json!({
                "target": target_id("prompt_input"),
            })),
        )
        .await;
        assert_eq!(content_text(&focused), "queued");

        let typed = call_tool(
            &peer,
            "ui_type_text",
            Some(json!({
                "target": target_id("prompt_input"),
                "text": "hello",
            })),
        )
        .await;
        assert_eq!(content_text(&typed), "queued");
        log_ui_text(&peer, "after type_text").await;

        wait_for("draft_prompt == hello", Duration::from_secs(2), || async {
            let snapshot = snapshot_value(&peer).await;
            let draft = snapshot
                .get("draft_prompt")
                .and_then(|value| value.as_str());
            WaitForStatus {
                done: draft == Some("hello"),
                detail: Some(format!("draft_prompt={draft:?}")),
            }
        })
        .await;
        log_ui_text(&peer, "after draft_prompt hello").await;

        let press_backspace = call_tool(
            &peer,
            "ui_press_key",
            Some(json!({
                "key": "Backspace",
                "modifiers": [],
            })),
        )
        .await;
        assert_eq!(content_text(&press_backspace), "queued");

        wait_for("draft_prompt == hell", Duration::from_secs(2), || async {
            let snapshot = snapshot_value(&peer).await;
            let draft = snapshot
                .get("draft_prompt")
                .and_then(|value| value.as_str());
            WaitForStatus {
                done: draft == Some("hell"),
                detail: Some(format!("draft_prompt={draft:?}")),
            }
        })
        .await;
        log_ui_text(&peer, "after backspace").await;

        let submit = call_tool(
            &peer,
            "submit_prompt",
            Some(json!({
                "prompt": "mcp hello",
                "timeout_ms": 2_000,
            })),
        )
        .await;
        assert_eq!(content_text(&submit), "Echo: mcp hello");
        log_ui_text(&peer, "after submit_prompt").await;

        wait_for(
            "submit_prompt echo visible",
            Duration::from_secs(2),
            || async {
                let result = call_tool(
                    &peer,
                    "ui_query",
                    Some(json!({
                        "kind": "text_contains",
                        "value": "Echo: mcp hello",
                    })),
                )
                .await;
                let present = bool_from_result(&result);
                WaitForStatus {
                    done: present,
                    detail: Some(format!("echo_present={present}")),
                }
            },
        )
        .await;
        log_ui_text(&peer, "after echo visible").await;

        let cancel = call_tool(&peer, "cancel_active_task", None).await;
        assert_eq!(content_text(&cancel), "cancel requested");
        log_ui_text(&peer, "after cancel request").await;

        wait_for(
            "cancel requested visible",
            Duration::from_secs(2),
            || async {
                let result = call_tool(
                    &peer,
                    "ui_query",
                    Some(json!({
                        "kind": "text_contains",
                        "value": "Cancel requested",
                    })),
                )
                .await;
                let present = bool_from_result(&result);
                WaitForStatus {
                    done: present,
                    detail: Some(format!("cancel_present={present}")),
                }
            },
        )
        .await;
        log_ui_text(&peer, "after cancel visible").await;

        let script = [
            r#"{"kind":"meta","schema_version":1,"app":"agent_demo"}"#,
            r#"{"kind":"action","action":"focus","target":{"by":"id","value":"prompt_input"}}"#,
            r#"{"kind":"action","action":"press_key","key":"A","modifiers":["ctrl"]}"#,
            r#"{"kind":"action","action":"press_key","key":"Backspace"}"#,
            r#"{"kind":"action","action":"type_text","target":{"by":"id","value":"prompt_input"},"text":"jsonl ping"}"#,
            r#"{"kind":"action","action":"sleep_ms","ms":300}"#,
            r#"{"kind":"expect","checks":[{"kind":"state_path_equals","path":"/draft_prompt","value":"jsonl ping"}],"within_ms":1000}"#,
            r#"{"kind":"action","action":"click","target":{"by":"id","value":"prompt_send"}}"#,
            r#"{"kind":"expect","checks":[{"kind":"ui_text_contains","value":"Echo: jsonl ping"}],"within_ms":2000}"#,
        ]
        .join("\n");
        let run_jsonl = call_tool(
            &peer,
            "run_jsonl",
            Some(json!({
                "script": script,
                "default_within_ms": 1_000,
                "poll_interval_ms": 10,
            })),
        )
        .await;
        assert_eq!(content_text(&run_jsonl), "ok");
        log_ui_text(&peer, "after run_jsonl").await;

        let screenshot = call_tool(
            &peer,
            "ui_screenshot",
            Some(json!({
                "timeout_ms": 2_000,
            })),
        )
        .await;
        let screenshot_meta: Value =
            serde_json::from_str(&content_text(&screenshot)).expect("screenshot json");
        let width = screenshot_meta
            .get("width")
            .and_then(|value| value.as_u64())
            .unwrap_or_default();
        let height = screenshot_meta
            .get("height")
            .and_then(|value| value.as_u64())
            .unwrap_or_default();
        assert!(width > 0 && height > 0, "invalid screenshot size");
        log_ui_text(&peer, "after screenshot").await;

        let echo_text = call_tool(
            &peer,
            "ui_query",
            Some(json!({
                "kind": "text_contains",
                "value": "Echo: jsonl ping",
            })),
        )
        .await;
        assert!(bool_from_result(&echo_text));
        log_ui_text(&peer, "after jsonl echo").await;

        client.close().await.ok();
    }
}
