//! MCP tool definitions + dispatch, built on the official `rmcp` SDK.
//!
//! Each tool is an async method annotated with `#[tool]`. The macro derives the input schema
//! from the typed parameter struct (via `schemars::JsonSchema`) and wires the method into a
//! [`ToolRouter`].
//!
//! The tools are split across two types so the app-driving half can be reused:
//!
//! - [`UiServer`] owns the core UI/inspection commands (`click`, `type_text`, `screenshot`,
//!   `query_tree`, `batch`, …). It is transport-agnostic: it drives whatever [`Bridge`] lives
//!   in the shared [`AppState`], and exposes [`UiServer::tools`] / [`UiServer::dispatch`] so
//!   another MCP server can embed it next to *its own* connection tools.
//! - [`Server`] is egui-mcp's own server: it adds the TCP connection lifecycle (`attach` /
//!   `disconnect` / `status`), spins up a [`UiServer`] on a successful `attach` (dropping it on
//!   `disconnect`), and delegates every non-lifecycle call to it — so the app-driving tools are
//!   listed only while connected, with a `tools/list_changed` notification on each transition.
//!   A server built on a different transport (e.g. one tunnelling over its own channel) would
//!   write its own connection tools and reuse [`UiServer`] the same way.
//!
//! App-driving tools go through `UiServer::run_inner`, which holds the [`AppState`] lock for
//! one call and returns a `no app connected` error when nothing is attached.
//!
//! Locator resolution is done here, MCP-side: a tool fetches a fresh AccessKit tree
//! (`GetTree`), resolves the locator to a screen position, synthesizes the matching
//! `egui::Event`s, and sends them via `ApplyEvents`. The app-side plugin stays low-level.
//!
//! Recoverable failures (no app connected, node not found, etc.) are returned as a tool
//! result with `isError: true`, not as a JSON-RPC error — per MCP spec.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, anyhow, bail};
use base64::Engine as _;
use egui::Event;
use egui::epaint::mutex::Mutex as SyncMutex;
use rmcp::{
    ErrorData as McpError, Peer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::ToolCallContext, wrapper::Parameters},
    model::{
        CallToolRequestParams, CallToolResult, Content, Implementation, InitializeRequestParams,
        InitializeResult, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo,
        Tool,
    },
    schemars,
    service::{RequestContext, RoleServer},
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::bridge::{Bridge, TreeSnapshot};
use crate::tree::{self, Locator, NodeView, QueryFilter};

// ---------------------------------------------------------------------------------------
// App state + Server wrapper
// ---------------------------------------------------------------------------------------

/// Holds the single in-flight bridge to a live egui app. Shared (via `Arc`) between a
/// connection-managing server and the [`UiServer`] that drives the app.
#[derive(Default)]
pub struct AppState {
    bridge: Mutex<Option<Bridge>>,

    /// The connected client, captured on `initialize`, so a connection change can push a
    /// `tools/list_changed` notification.
    peer: Mutex<Option<Peer<RoleServer>>>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Install a bridge built elsewhere (e.g. tunnelled over another transport), so the
    /// [`UiServer`] tools drive it as if it had been `attach`ed. Replaces any current bridge.
    pub async fn install_bridge(&self, bridge: Bridge) {
        *self.bridge.lock().await = Some(bridge);
    }

    /// Clear the current bridge, if any. Returns whether one was installed.
    pub async fn clear_bridge(&self) -> bool {
        self.bridge.lock().await.take().is_some()
    }

    /// Whether a bridge is currently installed.
    pub async fn is_connected(&self) -> bool {
        self.bridge.lock().await.is_some()
    }

    /// Remember the client peer, so [`Self::notify_tools_changed`] can reach it.
    pub async fn set_peer(&self, peer: Peer<RoleServer>) {
        *self.peer.lock().await = Some(peer);
    }

    /// Tell the client the visible tool set changed, so it re-fetches `tools/list`.
    pub async fn notify_tools_changed(&self) {
        let peer = self.peer.lock().await.clone();
        if let Some(peer) = peer
            && let Err(e) = peer.notify_tool_list_changed().await
        {
            tracing::warn!("failed to send tools/list_changed: {e}");
        }
    }
}

/// The reusable core: an MCP server exposing the egui UI/inspection tools (`click`,
/// `type_text`, `screenshot`, `query_tree`, `batch`, …) against the [`Bridge`] in a shared
/// [`AppState`].
///
/// It owns no connection logic. A host installs a bridge through [`AppState::install_bridge`]
/// (however it was obtained — TCP, a tunnelled transport, …), then either serves this directly
/// or embeds it: [`UiServer::tools`] lists the commands and [`UiServer::dispatch`] runs one, so
/// another MCP server can offer its own connection tools alongside this shared command set.
#[derive(Clone)]
pub struct UiServer {
    state: Arc<AppState>,
    router: ToolRouter<Self>,
}

impl UiServer {
    /// Build a UI server over a shared [`AppState`].
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            router: Self::tool_router(),
        }
    }

    /// The UI/inspection tools, for merging into an embedding server's `list_tools`.
    pub fn tools(&self) -> Vec<Tool> {
        self.router.list_all()
    }

    /// Look up one of the UI tools by name (for an embedding server's `get_tool`).
    pub fn get_tool(&self, name: &str) -> Option<Tool> {
        self.router.get(name).cloned()
    }

    /// Run one UI tool, for an embedding server's `call_tool` to delegate to.
    ///
    /// # Errors
    /// If the tool name is unknown (recoverable failures are reported in the `CallToolResult`).
    pub async fn dispatch(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tcc = ToolCallContext::new(self, request, context);
        self.router.call(tcc).await
    }

    /// Acquire the bridge lock, run `f(bridge)`, and shape the result into a
    /// `CallToolResult`. Returns `is_error: true` if no app is connected.
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
            return Ok(text_error("no app connected — call `attach` first"));
        };
        match f(bridge).await {
            Ok(v) => Ok(text_ok(&v)),
            Err(e) => Ok(text_error(format!("{e:#}"))),
        }
    }
}

/// egui-mcp's own MCP server: the TCP connection lifecycle (`attach` / `disconnect` /
/// `status`) plus a [`UiServer`] that exists only while an app is attached.
///
/// `ui` is `None` until `attach` connects (and is dropped on `disconnect`), so the app-driving
/// tools are listed only while connected; each transition pushes a `tools/list_changed`
/// notification. It is shared behind an `Arc<Mutex<…>>` because the `#[tool]` methods that flip
/// it only get `&self`.
#[derive(Clone)]
pub struct Server {
    state: Arc<AppState>,
    ui: Arc<SyncMutex<Option<UiServer>>>,
    lifecycle_router: ToolRouter<Self>,
}

impl Server {
    pub fn new() -> Self {
        Self::from_state(AppState::new())
    }

    /// Build a server around a shared [`AppState`], so a host can hold the
    /// same state and install a bridge into it out-of-band.
    pub fn from_state(state: Arc<AppState>) -> Self {
        Self {
            state,
            ui: Arc::new(SyncMutex::new(None)),
            lifecycle_router: Self::lifecycle_router(),
        }
    }

    /// The shared app state (holds the active bridge).
    pub fn state(&self) -> Arc<AppState> {
        Arc::clone(&self.state)
    }

    /// Snapshot the current [`UiServer`], if an app is attached.
    fn ui(&self) -> Option<UiServer> {
        self.ui.lock().clone()
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

/// Resolve a [`Target`] to an optional node id + a logical-point position, fetching a fresh
/// tree first. Use [`resolve_in_tree`] directly when resolving several targets against one
/// snapshot (e.g. a drag's start and end).
async fn resolve_target(
    bridge: &Bridge,
    target: &Target,
) -> anyhow::Result<(Option<String>, egui::Pos2)> {
    // A raw `pos` target needs no tree.
    if let Some(p) = target.pos {
        return Ok((None, egui::Pos2::new(p.x, p.y)));
    }
    let snap = bridge.fetch_tree().await?;
    resolve_in_tree(&snap, target)
}

/// Resolve a [`Target`] against an already-fetched tree snapshot.
fn resolve_in_tree(
    snap: &TreeSnapshot,
    target: &Target,
) -> anyhow::Result<(Option<String>, egui::Pos2)> {
    if let Some(p) = target.pos {
        return Ok((None, egui::Pos2::new(p.x, p.y)));
    }
    let locator = target
        .as_locator()
        .ok_or_else(|| anyhow!("target requires `id`, `role`, `label_contains`, or `pos`"))?;
    let tree = snap
        .tree
        .as_ref()
        .ok_or_else(|| anyhow!("no accesskit tree yet"))?;
    let node = tree::resolve_node(tree, &locator).ok_or_else(|| anyhow!("node not found"))?;
    let view = tree::node_view(&node);
    let bounds = view
        .bounds
        .ok_or_else(|| anyhow!("node has no bounds — can't target"))?;
    let (cx, cy) = bounds.center();
    let center = egui::Pos2::new(
        cx as f32 / snap.pixels_per_point,
        cy as f32 / snap.pixels_per_point,
    );
    Ok((Some(view.id), center))
}

// ---------------------------------------------------------------------------------------
// Args structs
// ---------------------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AttachArgs {
    /// Host the app's inspection port is on. Defaults to `127.0.0.1`.
    #[serde(default = "default_host")]
    pub host: String,

    /// Port the app is listening on. Defaults to `5719` (`egui_inspection`'s default).
    #[serde(default = "default_port")]
    pub port: u16,

    /// How long to keep retrying the connection, in seconds. Defaults to 10.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

fn default_host() -> String {
    "127.0.0.1".to_owned()
}

fn default_port() -> u16 {
    5719
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct EmptyArgs {}

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

fn default_one() -> u32 {
    1
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

/// Lifecycle tools — connection management, served directly by [`Server`].
#[tool_router(router = lifecycle_router)]
impl Server {
    #[tool(
        description = "Connect to a running egui app's inspection port (an app built with \
                        eframe's `inspection` feature, launched with `EGUI_INSPECTION` set). \
                        Defaults to 127.0.0.1:5719. Retries until `timeout_secs` elapses. \
                        On success the app-driving tools become available."
    )]
    async fn attach(
        &self,
        Parameters(args): Parameters<AttachArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.state.bridge.lock().await;
        if guard.is_some() {
            return Ok(text_error(
                "already connected — call `disconnect` first before attaching again",
            ));
        }
        let timeout = args.timeout_secs.map(Duration::from_secs);
        let bridge = match Bridge::connect(&args.host, args.port, timeout).await {
            Ok(b) => b,
            Err(e) => return Ok(text_error(format!("attach failed: {e:#}"))),
        };
        let info = bridge.peer_info.clone();
        *guard = Some(bridge);
        // Spin up the UI tools now that there's an app to drive, and announce them.
        *self.ui.lock() = Some(UiServer::new(Arc::clone(&self.state)));
        drop(guard);
        self.state.notify_tools_changed().await;
        Ok(text_ok(&json!({ "ok": true, "attached": info })))
    }

    #[tool(
        description = "Disconnect from the attached app. After this, the app-driving tools \
                        are hidden until `attach` is called again."
    )]
    async fn disconnect(&self, _p: Parameters<EmptyArgs>) -> Result<CallToolResult, McpError> {
        if self.state.clear_bridge().await {
            *self.ui.lock() = None;
            self.state.notify_tools_changed().await;
            Ok(text_ok(&json!({ "ok": true })))
        } else {
            Ok(text_error("no app connected"))
        }
    }

    #[tool(description = "Report whether an app is connected and its peer info.")]
    async fn status(&self, _p: Parameters<EmptyArgs>) -> Result<CallToolResult, McpError> {
        let guard = self.state.bridge.lock().await;
        let body = match guard.as_ref() {
            None => json!({ "state": "idle" }),
            Some(bridge) => json!({ "state": "connected", "peer": bridge.peer_info }),
        };
        Ok(text_ok(&body))
    }
}

/// App-driving tools — each needs an attached app (see the module docs).
#[tool_router]
impl UiServer {
    #[tool(
        description = "Capture the current frame as a PNG screenshot. Requires the app window \
                        to be visible — a fully-occluded or minimized window can't render a \
                        frame to capture (notably on macOS), so the call times out; bring the \
                        window to the foreground first."
    )]
    async fn screenshot(&self, _p: Parameters<EmptyArgs>) -> Result<CallToolResult, McpError> {
        let guard = self.state.bridge.lock().await;
        let Some(bridge) = guard.as_ref() else {
            return Ok(text_error("no app connected — call `attach` first"));
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
                let snap = bridge.fetch_tree().await?;
                let results = match snap.tree {
                    Some(tree) => tree::query(&tree, &filter),
                    None => Vec::new(),
                };
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
                let snap = bridge.fetch_tree().await?;
                let view = snap.tree.and_then(|tree| {
                    tree::resolve_node(&tree, &locator).map(|n| tree::node_view(&n))
                });
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
                        or aliases `left`/`right`). `count: 2` → double-click, `3` → triple."
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
                        pump a few frames so tooltips / hover popups settle."
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
                        logical points: positive Y scrolls content down; positive X right."
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

    #[tool(description = "Resize the app's viewport to the given logical-point dimensions.")]
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

    #[tool(
        description = "Type text into the currently focused widget. Sends one `Event::Text` \
                        per character (each applied in its own frame). Optionally first \
                        focuses a node (by `id` or `role`/`label_contains`) via a click."
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
                        items it produced (e.g. screenshots). `batch` cannot be nested. \
                        Use this to act and observe in one call, e.g. a `click` then a \
                        `query_tree` or `screenshot`."
    )]
    async fn batch(
        &self,
        Parameters(args): Parameters<BatchArgs>,
    ) -> Result<CallToolResult, McpError> {
        if args.actions.iter().any(|a| a.name == "batch") {
            return Ok(text_error("nested `batch` is not allowed"));
        }
        let mut content: Vec<Content> = Vec::new();
        let mut any_error = false;
        for action in args.actions {
            let result = Box::pin(self.dispatch_internal(&action.name, action.args)).await;
            let mut step_texts: Vec<String> = Vec::new();
            let mut step_images: Vec<Content> = Vec::new();
            for item in &result.content {
                if let Some(text) = content_as_text(item) {
                    step_texts.push(text.to_owned());
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

impl ServerHandler for Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        )
        .with_server_info(Implementation::new("egui-mcp", env!("CARGO_PKG_VERSION")))
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }
        // Remember the peer so attach / disconnect can push `tools/list_changed`.
        self.state.set_peer(context.peer.clone()).await;
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        // Lifecycle tools always; the UI server's tools only while an app is attached.
        let mut tools = self.lifecycle_router.list_all();
        if let Some(ui) = self.ui() {
            tools.extend(ui.tools());
        }
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Lifecycle tools run on `self`; everything else is delegated to the UI server, which
        // only exists while an app is attached.
        if self.lifecycle_router.has_route(&request.name) {
            let tcc = ToolCallContext::new(self, request, context);
            return self.lifecycle_router.call(tcc).await;
        }
        match self.ui() {
            Some(ui) => ui.dispatch(request, context).await,
            None => Ok(text_error("no app connected — call `attach` first")),
        }
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        if let Some(tool) = self.lifecycle_router.get(name) {
            return Some(tool.clone());
        }
        self.ui()?.get_tool(name)
    }
}

// ---------------------------------------------------------------------------------------
// Batch internal dispatch
// ---------------------------------------------------------------------------------------

impl UiServer {
    /// Route a UI tool call by name. Used by `batch` to recurse without going back through
    /// the rmcp router. Only the app-driving tools are reachable — connection management lives
    /// on the embedding server, not here.
    async fn dispatch_internal(&self, name: &str, args: Value) -> CallToolResult {
        match name {
            "screenshot" => unpack_then(args, |p| async move { self.screenshot(p).await }).await,
            "query_tree" => unpack_then(args, |p| async move { self.query_tree(p).await }).await,
            "get_node" => unpack_then(args, |p| async move { self.get_node(p).await }).await,
            "click" => unpack_then(args, |p| async move { self.click(p).await }).await,
            "hover" => unpack_then(args, |p| async move { self.hover(p).await }).await,
            "scroll" => unpack_then(args, |p| async move { self.scroll(p).await }).await,
            "drag" => unpack_then(args, |p| async move { self.drag(p).await }).await,
            "resize" => unpack_then(args, |p| async move { self.resize(p).await }).await,
            "wait_for" => unpack_then(args, |p| async move { self.wait_for(p).await }).await,
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
    width: u32,
    height: u32,
}

async fn screenshot_inner(bridge: &Bridge) -> anyhow::Result<(ScreenshotMeta, String)> {
    let png = bridge.screenshot().await?;
    let meta = ScreenshotMeta {
        width: png.size[0],
        height: png.size[1],
    };
    Ok((
        meta,
        base64::engine::general_purpose::STANDARD.encode(&png.bytes),
    ))
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

    // Press/release pairs share one frame's input time, so egui's multi-click detection
    // turns `count` into double/triple clicks.
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
    bridge.apply_events(events).await?;
    Ok(json!({
        "ok": true,
        "clicked_id": node_id,
        "pos": [center.x, center.y],
        "button": args.button,
        "count": count,
    }))
}

async fn hover_inner(bridge: &Bridge, args: HoverArgs) -> anyhow::Result<Value> {
    let (node_id, pos) = resolve_target(bridge, &args.target).await?;
    bridge.apply_events(vec![Event::PointerMoved(pos)]).await?;
    // Pump extra frames so tooltips / hover popups settle.
    for _ in 0..args.settle_frames {
        let _ = bridge.fetch_tree().await?;
    }
    Ok(json!({
        "ok": true,
        "hovered_id": node_id,
        "pos": [pos.x, pos.y],
    }))
}

async fn scroll_inner(bridge: &Bridge, args: ScrollArgs) -> anyhow::Result<Value> {
    let (node_id, pos) = resolve_target(bridge, &args.target).await?;
    let modifiers = args.modifiers.to_egui();
    let events = vec![
        Event::PointerMoved(pos),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::Vec2::new(args.delta.x, args.delta.y),
            phase: egui::TouchPhase::Move,
            modifiers,
        },
    ];
    bridge.apply_events(events).await?;
    Ok(json!({
        "ok": true,
        "scrolled_id": node_id,
        "pos": [pos.x, pos.y],
        "delta": [args.delta.x, args.delta.y],
    }))
}

async fn drag_inner(bridge: &Bridge, args: DragArgs) -> anyhow::Result<Value> {
    // Resolve both endpoints against one tree snapshot — no input happens between them.
    let snap = bridge.fetch_tree().await?;
    let (start_id, start_pos) = resolve_in_tree(&snap, &args.start)?;
    let (end_id, end_pos) = resolve_in_tree(&snap, &args.end)?;
    let modifiers = args.modifiers.to_egui();
    let steps = args.steps.max(1);

    bridge
        .apply_events(vec![
            Event::PointerMoved(start_pos),
            Event::PointerButton {
                pos: start_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers,
            },
        ])
        .await?;

    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let waypoint = egui::Pos2::new(
            start_pos.x + (end_pos.x - start_pos.x) * t,
            start_pos.y + (end_pos.y - start_pos.y) * t,
        );
        bridge
            .apply_events(vec![Event::PointerMoved(waypoint)])
            .await?;
    }

    bridge
        .apply_events(vec![Event::PointerButton {
            pos: end_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers,
        }])
        .await?;
    Ok(json!({
        "ok": true,
        "start_id": start_id,
        "end_id": end_id,
        "start_pos": [start_pos.x, start_pos.y],
        "end_pos": [end_pos.x, end_pos.y],
        "steps": steps,
    }))
}

async fn resize_inner(bridge: &Bridge, args: ResizeArgs) -> anyhow::Result<Value> {
    bridge.resize(args.width, args.height).await?;
    Ok(json!({ "ok": true, "width": args.width, "height": args.height }))
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
        let snap = bridge.fetch_tree().await?;
        let matches: Vec<NodeView> = match snap.tree {
            Some(tree) => tree::query(&tree, &filter),
            None => Vec::new(),
        };
        if matches.len() as u32 >= args.min_matches {
            return Ok(json!({ "ok": true, "matched": matches }));
        }
        if tokio::time::Instant::now() >= deadline {
            bail!(
                "wait_for timed out after {}s (role={:?}, label_contains={:?}, found {})",
                args.timeout_secs,
                args.role,
                args.label_contains,
                matches.len()
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn type_text_inner(bridge: &Bridge, args: TypeTextArgs) -> anyhow::Result<Value> {
    // Optionally focus a target widget by clicking it first.
    let focused_locator =
        if args.id.is_some() || args.role.is_some() || args.label_contains.is_some() {
            let click_args = ClickArgs {
                target: Target {
                    id: args.id.clone(),
                    role: args.role.clone(),
                    label_contains: args.label_contains.clone(),
                    pos: None,
                },
                button: "primary".to_owned(),
                count: 1,
                modifiers: PressKeyModifiers::default(),
            };
            Some(click_inner(bridge, click_args).await?)
        } else {
            None
        };

    let mut chars_sent = 0u32;
    for ch in args.text.chars() {
        if ch.is_control() {
            continue;
        }
        bridge
            .apply_events(vec![Event::Text(ch.to_string())])
            .await?;
        chars_sent += 1;
    }

    Ok(json!({
        "ok": true,
        "chars_sent": chars_sent,
        "focused": focused_locator,
    }))
}

async fn press_key_inner(bridge: &Bridge, args: PressKeyArgs) -> anyhow::Result<Value> {
    let key =
        egui::Key::from_name(&args.key).ok_or_else(|| anyhow!("unknown key `{}`", args.key))?;
    let modifiers = args.modifiers.to_egui();
    bridge
        .apply_events(vec![
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
        ])
        .await?;
    Ok(json!({ "ok": true, "key": args.key }))
}
