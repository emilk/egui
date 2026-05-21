//! MCP tool definitions + dispatch, built on the official `rmcp` SDK.
//!
//! Each tool is an async method on [`Server`] annotated with `#[tool]`. The macro derives
//! the input schema from the typed parameter struct (via `schemars::JsonSchema`) and wires
//! the method into a [`ToolRouter`] returned by [`Server::tool_router`].
//!
//! Tools that need a running app go through [`Server::run_inner`], which holds the
//! `AppState` lock for the duration of one call. Lifecycle tools (`launch`, `attach`,
//! `kill`, `status`) manage the bridge themselves.
//!
//! Recoverable failures (no app running, node not found, etc.) are returned as a tool
//! result with `isError: true`, not as a JSON-RPC error — per MCP spec, recoverable tool
//! failures belong in `result`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, anyhow, bail};
use base64::Engine as _;
use egui::Event;
use egui_inspection::protocol::InspectorCommand;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::bridge::{Bridge, StateSnapshot};
use crate::tree::{self, Locator, NodeView, QueryFilter};

// ---------------------------------------------------------------------------------------
// App state + Server wrapper
// ---------------------------------------------------------------------------------------

/// Holds the single in-flight bridge to a kittest harness / live egui app. Shared between
/// all `#[tool]` handlers on [`Server`].
#[derive(Default)]
pub struct AppState {
    bridge: Mutex<Option<Bridge>>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

#[derive(Clone)]
pub struct Server {
    state: Arc<AppState>,
    #[allow(dead_code, reason = "read by the `#[tool_router]` macro expansion")]
    tool_router: ToolRouter<Self>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
            tool_router: Self::tool_router(),
        }
    }

    /// Acquire the bridge lock, run `f(bridge)`, and shape the result into a
    /// `CallToolResult`. Returns `is_error: true` if no app is running, matching MCP
    /// spec for recoverable failures. The future returned by `f` is boxed so the closure
    /// can borrow the bridge across an `await` point.
    async fn run_inner<R, F>(&self, f: F) -> Result<CallToolResult, McpError>
    where
        R: Serialize,
        F: for<'a> FnOnce(
            &'a Bridge,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = anyhow::Result<R>> + Send + 'a>,
        >,
    {
        let guard = self.state.bridge.lock().await;
        let Some(bridge) = guard.as_ref() else {
            return Ok(text_error(
                "no app running — call `launch` or `attach` first",
            ));
        };
        match f(bridge).await {
            Ok(v) => Ok(text_ok(&v)),
            Err(e) => Ok(text_error(format!("{e:#}"))),
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------------------
// Helpers: ToolResult shaping
// ---------------------------------------------------------------------------------------

fn text_ok<T: Serialize>(value: &T) -> CallToolResult {
    match serde_json::to_string(value) {
        Ok(s) => CallToolResult::success(vec![Content::text(s)]),
        Err(e) => text_error(format!("serialize result: {e}")),
    }
}

fn text_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(msg.into())])
}

// ---------------------------------------------------------------------------------------
// Target — locator OR raw position, shared by click / hover / scroll / drag.
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct Target {
    /// Decimal AccessKit node id from `query_tree`.
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub label_contains: Option<String>,
    /// Raw position in logical points (use instead of locator fields).
    #[serde(default)]
    pub pos: Option<Pos2Lit>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
pub struct Pos2Lit {
    pub x: f32,
    pub y: f32,
}

impl Target {
    fn has_any(&self) -> bool {
        self.id.is_some()
            || self.role.is_some()
            || self.label_contains.is_some()
            || self.pos.is_some()
    }

    fn as_locator(&self) -> Option<Locator> {
        if self.id.is_none() && self.role.is_none() && self.label_contains.is_none() {
            return None;
        }
        let locator_json = json!({
            "id": self.id,
            "role": self.role,
            "label_contains": self.label_contains,
        });
        serde_json::from_value(locator_json).ok()
    }
}

async fn resolve_target(
    bridge: &Bridge,
    target: &Target,
) -> anyhow::Result<(Option<String>, egui::Pos2)> {
    if !target.has_any() {
        bail!("target requires `id`, `role`, `label_contains`, or `pos`");
    }
    if let Some(p) = target.pos {
        return Ok((None, egui::Pos2::new(p.x, p.y)));
    }
    let locator = target
        .as_locator()
        .ok_or_else(|| anyhow!("target requires `id`, `role`, `label_contains`, or `pos`"))?;
    let snap = bridge.state.snapshot().await;
    let pixels_per_point = snap.frame.as_ref().map(|f| f.pixels_per_point).unwrap_or(1.0);
    bridge
        .state
        .with_tree(|t| {
            let tree = t.ok_or_else(|| anyhow!("no accesskit tree yet"))?;
            let node = tree::resolve_node(tree, &locator)
                .ok_or_else(|| anyhow!("node not found"))?;
            let view = tree::node_view(&node);
            let bounds = view
                .bounds
                .ok_or_else(|| anyhow!("node has no bounds — can't target"))?;
            let (cx, cy) = bounds.center();
            let center = egui::Pos2::new(
                (cx as f32) / pixels_per_point,
                (cy as f32) / pixels_per_point,
            );
            Ok::<_, anyhow::Error>((Some(view.id), center))
        })
        .await
}

// ---------------------------------------------------------------------------------------
// Args structs
// ---------------------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LaunchArgs {
    /// Path to the binary to spawn.
    pub bin: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AttachArgs {
    /// Optional binary to spawn with the inspection socket pre-wired.
    #[serde(default)]
    pub bin: Option<PathBuf>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default = "default_attach_timeout")]
    pub timeout_secs: u64,
}

fn default_attach_timeout() -> u64 {
    60
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct EmptyArgs {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StepArgs {
    #[serde(default = "default_one")]
    pub count: u32,
}

fn default_one() -> u32 {
    1
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetNodeArgs {
    pub id: String,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct PressKeyModifiers {
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub mac_cmd: bool,
    /// = Cmd on Mac / Ctrl on Win+Linux.
    #[serde(default)]
    pub command: bool,
}

impl PressKeyModifiers {
    fn to_egui(&self) -> egui::Modifiers {
        egui::Modifiers {
            alt: self.alt,
            ctrl: self.ctrl,
            shift: self.shift,
            mac_cmd: self.mac_cmd,
            command: self.command,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClickArgs {
    #[serde(flatten)]
    pub target: Target,
    /// `primary`/`secondary`/`middle`/`extra1`/`extra2` (or aliases `left`/`right`).
    #[serde(default = "default_click_button")]
    pub button: String,
    /// `2` → double-click; `3` → triple-click (multi-click detected via egui's timing).
    #[serde(default = "default_one")]
    pub count: u32,
    #[serde(default)]
    pub modifiers: PressKeyModifiers,
}

fn default_click_button() -> String {
    "primary".into()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HoverArgs {
    #[serde(flatten)]
    pub target: Target,
    #[serde(default = "default_settle_frames")]
    pub settle_frames: u32,
}

fn default_settle_frames() -> u32 {
    2
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScrollArgs {
    #[serde(flatten)]
    pub target: Target,
    /// Logical points. Positive Y scrolls content down (revealing content below).
    pub delta: Pos2Lit,
    #[serde(default)]
    pub modifiers: PressKeyModifiers,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DragArgs {
    pub start: Target,
    pub end: Target,
    #[serde(default = "default_drag_steps")]
    pub steps: u32,
    #[serde(default)]
    pub modifiers: PressKeyModifiers,
}

fn default_drag_steps() -> u32 {
    8
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResizeArgs {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WaitForArgs {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub label_contains: Option<String>,
    #[serde(default = "default_wait_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_min_matches")]
    pub min_matches: u32,
}

fn default_wait_timeout() -> u64 {
    5
}

fn default_min_matches() -> u32 {
    1
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TypeTextArgs {
    pub text: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub label_contains: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PressKeyArgs {
    pub key: String,
    #[serde(default)]
    pub modifiers: PressKeyModifiers,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchArgs {
    pub actions: Vec<BatchAction>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchAction {
    pub name: String,
    #[serde(default)]
    #[schemars(with = "serde_json::Map<String, serde_json::Value>")]
    pub args: Value,
}

// ---------------------------------------------------------------------------------------
// Tool router — each tool is a thin wrapper around an inner async fn.
// ---------------------------------------------------------------------------------------

#[tool_router]
impl Server {
    #[tool(
        description = "Spawn a kittest harness binary as a child process. The binary must \
                        link `egui_kittest` and call `Harness::run()` — `InspectorPlugin` \
                        auto-attaches via the `KITTEST_INSPECTOR` env var this tool sets."
    )]
    async fn launch(
        &self,
        Parameters(args): Parameters<LaunchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.bridge.lock().await;
        if guard.is_some() {
            return Ok(text_error(
                "an app is already running — call `kill` first to start a new one",
            ));
        }
        let env: Vec<(String, String)> = args.env.into_iter().collect();
        let bridge = match Bridge::launch(args.bin, args.args, env, args.cwd).await {
            Ok(b) => b,
            Err(e) => return Ok(text_error(format!("launch failed: {e:#}"))),
        };
        let _ = bridge
            .wait_for_frame_after(0, Duration::from_secs(5))
            .await;
        let snap = bridge.state.snapshot().await;
        let info = bridge.peer_info.clone();
        *guard = Some(bridge);
        Ok(text_ok(&json!({
            "ok": true,
            "launched": info,
            "step": snap.frame.as_ref().map(|f| f.step).unwrap_or(0),
            "blocked": snap.blocked,
        })))
    }

    #[tool(
        description = "Bind a unix socket and wait for a live egui app (built with the \
                        `egui_inspection` plugin, e.g. eframe + the `inspection` feature) \
                        to dial in. If `bin` is provided, also spawns it with \
                        `EGUI_INSPECTION_SOCKET` pre-set. Otherwise prints the path and \
                        waits for an externally-launched app."
    )]
    async fn attach(
        &self,
        Parameters(args): Parameters<AttachArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.bridge.lock().await;
        if guard.is_some() {
            return Ok(text_error(
                "an app is already running — call `kill` first before attaching",
            ));
        }
        let (socket_dir, listener, socket_path) = match Bridge::prepare_attach().await {
            Ok(t) => t,
            Err(e) => return Ok(text_error(format!("attach prepare failed: {e:#}"))),
        };

        let mut spawned: Option<tokio::process::Child> = None;
        if let Some(bin) = args.bin.clone() {
            let mut cmd = tokio::process::Command::new(&bin);
            cmd.args(&args.args)
                .env(egui_inspection::INSPECTION_SOCKET_ENV_VAR, &socket_path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true);
            for (k, v) in &args.env {
                cmd.env(k, v);
            }
            if let Some(d) = &args.cwd {
                cmd.current_dir(d);
            }
            match cmd.spawn() {
                Ok(c) => spawned = Some(c),
                Err(e) => return Ok(text_error(format!("spawn {}: {e}", bin.display()))),
            }
        }

        let timeout = Duration::from_secs(args.timeout_secs);
        let bridge = match Bridge::accept_pending(
            socket_dir,
            listener,
            socket_path.clone(),
            spawned,
            timeout,
        )
        .await
        {
            Ok(b) => b,
            Err(e) => return Ok(text_error(format!("attach failed: {e:#}"))),
        };

        // Live plugins push a frame on each `output_hook`; we wait briefly for the first
        // one so the AccessKit tree is populated before any inspector tool call.
        let _ = bridge.wait_for_frame_after(0, Duration::from_secs(5)).await;
        let snap = bridge.state.snapshot().await;
        let info = bridge.peer_info.clone();
        *guard = Some(bridge);
        Ok(text_ok(&json!({
            "ok": true,
            "attached": info,
            "step": snap.frame.as_ref().map(|f| f.step).unwrap_or(0),
        })))
    }

    #[tool(
        description = "Terminate the running harness child (or detach from an attached live \
                        app). After kill, other tools return `not_running` until `launch` / \
                        `attach` is called again."
    )]
    async fn kill(&self, _p: Parameters<EmptyArgs>) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.bridge.lock().await;
        match guard.take() {
            Some(bridge) => {
                bridge.kill().await;
                Ok(text_ok(&json!({ "ok": true })))
            }
            None => Ok(text_error("no app running")),
        }
    }

    #[tool(
        description = "Report whether a harness is running and its current step/blocked state."
    )]
    async fn status(&self, _p: Parameters<EmptyArgs>) -> Result<CallToolResult, McpError> {
        let guard = self.state.bridge.lock().await;
        let body = match guard.as_ref() {
            None => json!({ "state": "idle" }),
            Some(bridge) => {
                let snap = bridge.state.snapshot().await;
                if let Some(fin) = &snap.finished {
                    json!({
                        "state": "finished",
                        "ok": fin.ok,
                        "message": fin.message,
                        "step": snap.frame.as_ref().map(|f| f.step),
                    })
                } else {
                    json!({
                        "state": "running",
                        "blocked": snap.blocked,
                        "step": snap.frame.as_ref().map(|f| f.step),
                        "peer": bridge.peer_info,
                    })
                }
            }
        };
        Ok(text_ok(&body))
    }

    #[tool(description = "Return the latest rendered frame as PNG.")]
    async fn screenshot(
        &self,
        _p: Parameters<EmptyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let guard = self.state.bridge.lock().await;
        let Some(bridge) = guard.as_ref() else {
            return Ok(text_error("no app running — call `launch` or `attach` first"));
        };
        match screenshot_inner(bridge).await {
            Ok((meta, png_b64)) => {
                let meta_text = match serde_json::to_string(&meta) {
                    Ok(s) => s,
                    Err(e) => return Ok(text_error(format!("serialize: {e}"))),
                };
                Ok(CallToolResult::success(vec![
                    Content::text(meta_text),
                    Content::image(png_b64, "image/png"),
                ]))
            }
            Err(e) => Ok(text_error(format!("{e:#}"))),
        }
    }

    #[tool(
        description = "Walk the AccessKit tree and return nodes matching the filter. Use \
                        the returned `id` (a decimal string) with `click`, `type_text`, or \
                        `get_node`."
    )]
    async fn query_tree(
        &self,
        Parameters(filter): Parameters<QueryFilter>,
    ) -> Result<CallToolResult, McpError> {
        self.run_inner(|bridge| {
            Box::pin(async move {
                let results = bridge
                    .state
                    .with_tree(|t| match t {
                        Some(tree) => tree::query(tree, &filter),
                        None => Vec::new(),
                    })
                    .await;
                Ok(results)
            })
        })
        .await
    }

    #[tool(description = "Return a single AccessKit node by id (decimal string).")]
    async fn get_node(
        &self,
        Parameters(args): Parameters<GetNodeArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.run_inner(|bridge| {
            Box::pin(async move {
                let locator_json = json!({ "id": args.id });
                let locator: Locator =
                    serde_json::from_value(locator_json).context("invalid id")?;
                let view = bridge
                    .state
                    .with_tree(|t| {
                        let tree = t?;
                        tree::resolve_node(tree, &locator).map(|n| tree::node_view(&n))
                    })
                    .await;
                Ok(view)
            })
        })
        .await
    }

    #[tool(
        description = "Click the center of a node's bounding box, or a raw `pos` in logical \
                        points. Specify either a locator (`id` from `query_tree` or \
                        `role`/`label_contains`) or `pos: { x, y }`. `button` defaults to \
                        `primary` (accepts `primary`/`secondary`/`middle`/`extra1`/`extra2`, \
                        or aliases `left`/`right`). `count` sends repeated press/release \
                        pairs in one batch — egui's multi-click detection turns `count: 2` \
                        into a double-click and `count: 3` into a triple-click."
    )]
    async fn click(
        &self,
        Parameters(args): Parameters<ClickArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.run_inner(|bridge| Box::pin(click_inner(bridge, args)))
            .await
    }

    #[tool(
        description = "Move the pointer over a node (or raw `pos`) without clicking, then \
                        step a few frames so tooltips / hover popups settle."
    )]
    async fn hover(
        &self,
        Parameters(args): Parameters<HoverArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.run_inner(|bridge| Box::pin(hover_inner(bridge, args)))
            .await
    }

    #[tool(
        description = "Send a mouse wheel scroll over a node (or raw `pos`). `delta` is in \
                        logical points: positive Y scrolls content down (revealing content \
                        below); positive X scrolls right."
    )]
    async fn scroll(
        &self,
        Parameters(args): Parameters<ScrollArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.run_inner(|bridge| Box::pin(scroll_inner(bridge, args)))
            .await
    }

    #[tool(
        description = "Primary-button drag from `start` to `end`. Each target accepts the \
                        same fields as `click`: locator (`id`/`role`/`label_contains`) or \
                        `pos: {x, y}`. `steps` controls how many intermediate pointer-move \
                        events are emitted between press and release."
    )]
    async fn drag(
        &self,
        Parameters(args): Parameters<DragArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.run_inner(|bridge| Box::pin(drag_inner(bridge, args)))
            .await
    }

    #[tool(
        description = "Resize the peer's viewport (live app: `ViewportCommand::InnerSize`) \
                        or the kittest harness window to the given logical-point dimensions."
    )]
    async fn resize(
        &self,
        Parameters(args): Parameters<ResizeArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.run_inner(|bridge| Box::pin(resize_inner(bridge, args)))
            .await
    }

    #[tool(
        description = "Poll the AccessKit tree until at least `min_matches` visible nodes \
                        match the filter, or until `timeout_secs` elapses."
    )]
    async fn wait_for(
        &self,
        Parameters(args): Parameters<WaitForArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.run_inner(|bridge| Box::pin(wait_for_inner(bridge, args)))
            .await
    }

    #[tool(description = "Advance the harness by N frames (default 1) and return the new screenshot.")]
    async fn step(
        &self,
        Parameters(args): Parameters<StepArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.run_inner(|bridge| Box::pin(step_inner(bridge, args)))
            .await
    }

    #[tool(
        description = "Type text into the currently focused widget. Sends one `Event::Text` \
                        per character and waits for a frame between characters so each \
                        keystroke is applied independently. Optionally first focuses a node \
                        (by `id` or `role`/`label_contains`) via a click before typing."
    )]
    async fn type_text(
        &self,
        Parameters(args): Parameters<TypeTextArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.run_inner(|bridge| Box::pin(type_text_inner(bridge, args)))
            .await
    }

    #[tool(
        description = "Send a key press (down + up) to the focused widget. `key` is an egui \
                        key name such as `Backspace`, `Delete`, `Enter`, `Tab`, `A`–`Z`, \
                        `ArrowLeft`, `ArrowRight`, `Home`, `End`, `Escape`."
    )]
    async fn press_key(
        &self,
        Parameters(args): Parameters<PressKeyArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.run_inner(|bridge| Box::pin(press_key_inner(bridge, args)))
            .await
    }

    #[tool(
        description = "Execute a sequence of tool calls in one round trip. Stops on the \
                        first error. Results are emitted in execution order, interleaved: \
                        each step contributes one JSON text item followed by any image \
                        items it produced (e.g. screenshots) — so position in the content \
                        stream tells you which step each image belongs to. `batch` cannot \
                        be nested."
    )]
    async fn batch(
        &self,
        Parameters(args): Parameters<BatchArgs>,
    ) -> Result<CallToolResult, McpError> {
        if args.actions.iter().any(|a| a.name == "batch") {
            return Ok(text_error("nested `batch` is not allowed"));
        }
        // Interleaved layout: for each step, emit a Text item carrying the step's JSON
        // metadata, then any Image items the step produced. Matches `browser_batch` in
        // claude-in-chrome — callers can tell which screenshot belongs to which step by
        // position in the content stream.
        let mut content: Vec<Content> = Vec::new();
        let mut any_error = false;
        for action in args.actions {
            let result = Box::pin(self.dispatch_internal(&action.name, action.args)).await;
            let mut step_texts: Vec<String> = Vec::new();
            let mut step_images: Vec<Content> = Vec::new();
            for item in &result.content {
                if let Some(text) = content_as_text(item) {
                    step_texts.push(text.to_string());
                } else if content_is_image(item) {
                    step_images.push(item.clone());
                }
            }
            let entry = json!({
                "name": action.name,
                "isError": result.is_error.unwrap_or(false),
                "content": step_texts,
            });
            let entry_text = match serde_json::to_string(&entry) {
                Ok(s) => s,
                Err(e) => return Ok(text_error(format!("serialize batch step: {e}"))),
            };
            content.push(Content::text(entry_text));
            content.extend(step_images);
            if result.is_error.unwrap_or(false) {
                any_error = true;
                break;
            }
        }
        let mut result = if any_error {
            CallToolResult::error(content)
        } else {
            CallToolResult::success(content)
        };
        result.is_error = Some(any_error);
        Ok(result)
    }
}

#[tool_handler]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "kittest-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
    }
}

// ---------------------------------------------------------------------------------------
// Batch internal dispatch
// ---------------------------------------------------------------------------------------

impl Server {
    /// Route a tool call by name. Used by `batch` to recurse without going back through
    /// the rmcp router (which would require a self-handle).
    async fn dispatch_internal(&self, name: &str, args: Value) -> CallToolResult {
        match name {
            "launch" => unpack_then(args, |p| async move { self.launch(p).await }).await,
            "attach" => unpack_then(args, |p| async move { self.attach(p).await }).await,
            "kill" => unpack_then(args, |p| async move { self.kill(p).await }).await,
            "status" => unpack_then(args, |p| async move { self.status(p).await }).await,
            "screenshot" => {
                unpack_then(args, |p| async move { self.screenshot(p).await }).await
            }
            "query_tree" => unpack_then(args, |p| async move { self.query_tree(p).await }).await,
            "get_node" => unpack_then(args, |p| async move { self.get_node(p).await }).await,
            "click" => unpack_then(args, |p| async move { self.click(p).await }).await,
            "hover" => unpack_then(args, |p| async move { self.hover(p).await }).await,
            "scroll" => unpack_then(args, |p| async move { self.scroll(p).await }).await,
            "drag" => unpack_then(args, |p| async move { self.drag(p).await }).await,
            "resize" => unpack_then(args, |p| async move { self.resize(p).await }).await,
            "wait_for" => unpack_then(args, |p| async move { self.wait_for(p).await }).await,
            "step" => unpack_then(args, |p| async move { self.step(p).await }).await,
            "type_text" => unpack_then(args, |p| async move { self.type_text(p).await }).await,
            "press_key" => unpack_then(args, |p| async move { self.press_key(p).await }).await,
            other => text_error(format!("unknown tool `{other}`")),
        }
    }
}

async fn unpack_then<A, F, Fut>(args: Value, f: F) -> CallToolResult
where
    A: for<'de> serde::Deserialize<'de>,
    F: FnOnce(Parameters<A>) -> Fut,
    Fut: std::future::Future<Output = Result<CallToolResult, McpError>>,
{
    let parsed: A = match serde_json::from_value(args) {
        Ok(p) => p,
        Err(e) => return text_error(format!("invalid arguments: {e}")),
    };
    match f(Parameters(parsed)).await {
        Ok(r) => r,
        Err(e) => text_error(e.message.to_string()),
    }
}

fn content_as_text(c: &Content) -> Option<&str> {
    match &c.raw {
        rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
        _ => None,
    }
}

fn content_is_image(c: &Content) -> bool {
    matches!(&c.raw, rmcp::model::RawContent::Image(_))
}

// ---------------------------------------------------------------------------------------
// Inner action handlers (shared with batch)
// ---------------------------------------------------------------------------------------

#[derive(Serialize)]
struct ScreenshotMeta {
    step: u64,
    width: u32,
    height: u32,
    pixels_per_point: f32,
}

async fn screenshot_inner(bridge: &Bridge) -> anyhow::Result<(ScreenshotMeta, String)> {
    // Fast path: kittest harnesses (and live apps in continuous-screenshot mode) attach a
    // `FrameScreenshot` to every frame. Just use the latest one if it already has pixels.
    let initial = bridge.state.snapshot().await;
    if let Some(frame) = initial.frame.as_ref() {
        if let Some(shot) = frame.screenshot.as_ref() {
            let meta = ScreenshotMeta {
                step: frame.step,
                width: shot.width,
                height: shot.height,
                pixels_per_point: frame.pixels_per_point,
            };
            let png = encode_png(shot).context("encode PNG")?;
            return Ok((meta, base64::engine::general_purpose::STANDARD.encode(png)));
        }
    }

    // Live apps only attach a screenshot on demand. Ask for a fresh capture, then wait
    // until a frame with `screenshot: Some(_)` arrives.
    let prev_step = initial.frame.as_ref().map(|f| f.step).unwrap_or(0);
    bridge.send(InspectorCommand::Screenshot)?;

    // The viewport screenshot needs at least two frames to round-trip on live apps:
    // one to issue the request, one to emit `Event::Screenshot`. Poll until the pixels
    // show up (or we time out).
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut last_step = prev_step;
    let frame = loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(anyhow!("timed out waiting for screenshot frame"));
        }
        let snap = bridge.wait_for_frame_after(last_step, remaining).await?;
        let Some(frame) = snap.frame else { continue };
        last_step = frame.step;
        if frame.screenshot.is_some() {
            break frame;
        }
    };
    let shot = frame.screenshot.as_ref().expect("checked above");
    let png = encode_png(shot).context("encode PNG")?;
    let meta = ScreenshotMeta {
        step: frame.step,
        width: shot.width,
        height: shot.height,
        pixels_per_point: frame.pixels_per_point,
    };
    Ok((
        meta,
        base64::engine::general_purpose::STANDARD.encode(png),
    ))
}

fn encode_png(shot: &egui_inspection::protocol::FrameScreenshot) -> anyhow::Result<Vec<u8>> {
    let img = image::RgbaImage::from_raw(shot.width, shot.height, shot.rgba.clone())
        .ok_or_else(|| anyhow!("frame rgba length mismatch"))?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)?;
    Ok(out.into_inner())
}

fn parse_pointer_button(name: &str) -> anyhow::Result<egui::PointerButton> {
    match name.to_ascii_lowercase().as_str() {
        "primary" | "left" => Ok(egui::PointerButton::Primary),
        "secondary" | "right" => Ok(egui::PointerButton::Secondary),
        "middle" => Ok(egui::PointerButton::Middle),
        "extra1" => Ok(egui::PointerButton::Extra1),
        "extra2" => Ok(egui::PointerButton::Extra2),
        other => bail!(
            "unknown button `{other}` — expected primary/secondary/middle/extra1/extra2 (or left/right)"
        ),
    }
}

async fn click_inner(bridge: &Bridge, args: ClickArgs) -> anyhow::Result<Value> {
    let button = parse_pointer_button(&args.button)?;
    let count = args.count.max(1);
    let modifiers = args.modifiers.to_egui();
    let (node_id, center) = resolve_target(bridge, &args.target).await?;
    let prev_step = bridge
        .state
        .snapshot()
        .await
        .frame
        .as_ref()
        .map(|f| f.step)
        .unwrap_or(0);

    // Send `count` press/release pairs in one batch — they share the frame's input time,
    // which egui treats as consecutive clicks within `multi_click_delay`, so double /
    // triple clicks register naturally.
    let mut events = vec![Event::PointerMoved(center)];
    for _ in 0..count {
        events.push(Event::PointerButton {
            pos: center,
            button,
            pressed: true,
            modifiers,
        });
        events.push(Event::PointerButton {
            pos: center,
            button,
            pressed: false,
            modifiers,
        });
    }
    bridge.send(InspectorCommand::Handle { events })?;
    let snap = bridge
        .wait_for_frame_after(prev_step, Duration::from_secs(5))
        .await?;
    Ok(json!({
        "ok": true,
        "clicked_id": node_id,
        "pos": [center.x, center.y],
        "button": args.button,
        "count": count,
        "step": snap.frame.as_ref().map(|f| f.step),
    }))
}

async fn hover_inner(bridge: &Bridge, args: HoverArgs) -> anyhow::Result<Value> {
    let (node_id, pos) = resolve_target(bridge, &args.target).await?;
    let mut last_step = bridge
        .state
        .snapshot()
        .await
        .frame
        .as_ref()
        .map(|f| f.step)
        .unwrap_or(0);

    bridge.send(InspectorCommand::Handle {
        events: vec![Event::PointerMoved(pos)],
    })?;
    let snap = bridge
        .wait_for_frame_after(last_step, Duration::from_secs(5))
        .await?;
    if let Some(f) = &snap.frame {
        last_step = f.step;
    }
    for _ in 0..args.settle_frames {
        bridge.send(InspectorCommand::Step)?;
        let snap = bridge
            .wait_for_frame_after(last_step, Duration::from_secs(5))
            .await?;
        if let Some(f) = &snap.frame {
            last_step = f.step;
        }
        if snap.finished.is_some() {
            break;
        }
    }
    Ok(json!({
        "ok": true,
        "hovered_id": node_id,
        "pos": [pos.x, pos.y],
        "step": last_step,
    }))
}

async fn scroll_inner(bridge: &Bridge, args: ScrollArgs) -> anyhow::Result<Value> {
    let (node_id, pos) = resolve_target(bridge, &args.target).await?;
    let modifiers = args.modifiers.to_egui();
    let prev_step = bridge
        .state
        .snapshot()
        .await
        .frame
        .as_ref()
        .map(|f| f.step)
        .unwrap_or(0);
    let events = vec![
        Event::PointerMoved(pos),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::Vec2::new(args.delta.x, args.delta.y),
            phase: egui::TouchPhase::Move,
            modifiers,
        },
    ];
    bridge.send(InspectorCommand::Handle { events })?;
    let snap = bridge
        .wait_for_frame_after(prev_step, Duration::from_secs(5))
        .await?;
    Ok(json!({
        "ok": true,
        "scrolled_id": node_id,
        "pos": [pos.x, pos.y],
        "delta": [args.delta.x, args.delta.y],
        "step": snap.frame.as_ref().map(|f| f.step),
    }))
}

async fn drag_inner(bridge: &Bridge, args: DragArgs) -> anyhow::Result<Value> {
    let (start_id, start_pos) = resolve_target(bridge, &args.start).await?;
    let (end_id, end_pos) = resolve_target(bridge, &args.end).await?;
    let modifiers = args.modifiers.to_egui();
    let steps = args.steps.max(1);

    let mut last_step = bridge
        .state
        .snapshot()
        .await
        .frame
        .as_ref()
        .map(|f| f.step)
        .unwrap_or(0);

    bridge.send(InspectorCommand::Handle {
        events: vec![
            Event::PointerMoved(start_pos),
            Event::PointerButton {
                pos: start_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers,
            },
        ],
    })?;
    let snap = bridge
        .wait_for_frame_after(last_step, Duration::from_secs(5))
        .await?;
    if let Some(f) = &snap.frame {
        last_step = f.step;
    }

    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let waypoint = egui::Pos2::new(
            start_pos.x + (end_pos.x - start_pos.x) * t,
            start_pos.y + (end_pos.y - start_pos.y) * t,
        );
        bridge.send(InspectorCommand::Handle {
            events: vec![Event::PointerMoved(waypoint)],
        })?;
        let snap = bridge
            .wait_for_frame_after(last_step, Duration::from_secs(5))
            .await?;
        if let Some(f) = &snap.frame {
            last_step = f.step;
        }
    }

    bridge.send(InspectorCommand::Handle {
        events: vec![Event::PointerButton {
            pos: end_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers,
        }],
    })?;
    let snap = bridge
        .wait_for_frame_after(last_step, Duration::from_secs(5))
        .await?;
    if let Some(f) = &snap.frame {
        last_step = f.step;
    }
    Ok(json!({
        "ok": true,
        "start_id": start_id,
        "end_id": end_id,
        "start_pos": [start_pos.x, start_pos.y],
        "end_pos": [end_pos.x, end_pos.y],
        "steps": steps,
        "step": last_step,
    }))
}

async fn resize_inner(bridge: &Bridge, args: ResizeArgs) -> anyhow::Result<Value> {
    let prev_step = bridge
        .state
        .snapshot()
        .await
        .frame
        .as_ref()
        .map(|f| f.step)
        .unwrap_or(0);
    bridge.send(InspectorCommand::Resize {
        width: args.width,
        height: args.height,
    })?;
    let snap = bridge
        .wait_for_frame_after(prev_step, Duration::from_secs(5))
        .await?;
    Ok(json!({
        "ok": true,
        "width": args.width,
        "height": args.height,
        "step": snap.frame.as_ref().map(|f| f.step),
    }))
}

async fn wait_for_inner(bridge: &Bridge, args: WaitForArgs) -> anyhow::Result<Value> {
    if args.role.is_none() && args.label_contains.is_none() {
        bail!("wait_for requires at least `role` or `label_contains`");
    }
    let filter = QueryFilter {
        role: args.role.clone(),
        label_contains: args.label_contains.clone(),
        visible_only: true,
        limit: args.min_matches as usize,
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(args.timeout_secs);
    loop {
        let matches: Vec<NodeView> = bridge
            .state
            .with_tree(|t| match t {
                Some(tree) => tree::query(tree, &filter),
                None => Vec::new(),
            })
            .await;
        if matches.len() as u32 >= args.min_matches {
            return Ok(json!({ "ok": true, "matched": matches }));
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!(
                "wait_for timed out after {}s (role={:?}, label_contains={:?}, found {})",
                args.timeout_secs,
                args.role,
                args.label_contains,
                matches.len()
            );
        }
        let _ = tokio::time::timeout(remaining, bridge.state.notified()).await;
    }
}

async fn step_inner(bridge: &Bridge, args: StepArgs) -> anyhow::Result<Value> {
    let count = if args.count == 0 { 1 } else { args.count };
    let mut last_step = bridge
        .state
        .snapshot()
        .await
        .frame
        .as_ref()
        .map(|f| f.step)
        .unwrap_or(0);
    for _ in 0..count {
        bridge.send(InspectorCommand::Step)?;
        let snap: StateSnapshot = bridge
            .wait_for_frame_after(last_step, Duration::from_secs(5))
            .await?;
        if let Some(f) = &snap.frame {
            last_step = f.step;
        }
        if snap.finished.is_some() {
            break;
        }
    }
    Ok(json!({ "ok": true, "step": last_step }))
}

async fn type_text_inner(bridge: &Bridge, args: TypeTextArgs) -> anyhow::Result<Value> {
    // Optionally focus a target widget by clicking it first.
    let focused_locator = if args.id.is_some() || args.role.is_some() || args.label_contains.is_some() {
        let click_args = ClickArgs {
            target: Target {
                id: args.id.clone(),
                role: args.role.clone(),
                label_contains: args.label_contains.clone(),
                pos: None,
            },
            button: "primary".to_string(),
            count: 1,
            modifiers: PressKeyModifiers::default(),
        };
        Some(click_inner(bridge, click_args).await?)
    } else {
        None
    };

    let mut last_step = bridge
        .state
        .snapshot()
        .await
        .frame
        .as_ref()
        .map(|f| f.step)
        .unwrap_or(0);

    let mut chars_sent = 0u32;
    for ch in args.text.chars() {
        if ch.is_control() {
            continue;
        }
        bridge.send(InspectorCommand::Handle {
            events: vec![Event::Text(ch.to_string())],
        })?;
        let snap = bridge
            .wait_for_frame_after(last_step, Duration::from_secs(5))
            .await?;
        if let Some(f) = &snap.frame {
            last_step = f.step;
        }
        chars_sent += 1;
    }

    Ok(json!({
        "ok": true,
        "chars_sent": chars_sent,
        "focused": focused_locator,
        "step": last_step,
    }))
}

async fn press_key_inner(bridge: &Bridge, args: PressKeyArgs) -> anyhow::Result<Value> {
    let key = egui::Key::from_name(&args.key)
        .ok_or_else(|| anyhow!("unknown key `{}`", args.key))?;
    let modifiers = args.modifiers.to_egui();
    let prev_step = bridge
        .state
        .snapshot()
        .await
        .frame
        .as_ref()
        .map(|f| f.step)
        .unwrap_or(0);
    bridge.send(InspectorCommand::Handle {
        events: vec![
            Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            },
            Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers,
            },
        ],
    })?;
    let snap = bridge
        .wait_for_frame_after(prev_step, Duration::from_secs(5))
        .await?;
    Ok(json!({
        "ok": true,
        "key": args.key,
        "step": snap.frame.as_ref().map(|f| f.step),
    }))
}
