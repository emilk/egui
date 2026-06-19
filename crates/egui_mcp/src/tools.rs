//! MCP tool definitions + dispatch, built on the official `rmcp` SDK.
//!
//! Each tool is an async method annotated with `#[tool]`. The macro derives the input schema
//! from the typed parameter struct (via `schemars::JsonSchema`) and wires the method into a
//! [`ToolRouter`].
//!
//! The tools are split across two types so the app-driving half can be reused:
//!
//! - [`UiServer`] owns the core UI/inspection commands (`click`, `type_text`, `screenshot`,
//!   `query_tree`, `batch`, …). It holds a single [`Bridge`] and nothing else — constructing
//!   one *is* the "connected" state. It is transport-agnostic: pair it with the router from
//!   [`UiServer::router`] to list and dispatch the commands, so another MCP server can embed
//!   it next to *its own* connection tools, driving whatever [`Bridge`] it built.
//! - [`Server`] is egui-mcp's own server: it owns the UI tools' [`ToolRouter`] plus an
//!   `Option<UiServer>` that is `Some` only while attached, and adds the TCP connection
//!   lifecycle (`attach` / `disconnect` / `status`). Because the router lives on the server
//!   rather than the (absent) [`UiServer`], the app-driving tools are always listed — MCP
//!   clients that cache the initial tool list discover them before `attach` — while calls made
//!   before `attach` return `no app connected`.
//!
//! Locator resolution is done here, MCP-side: a tool fetches a fresh AccessKit tree
//! (`GetTree`), resolves the locator to a logical-point position, synthesizes the matching
//! `egui::Event`s, and sends them via `ApplyEvents`. The app-side plugin stays low-level.
//!
//! Recoverable failures (no app connected, node not found, etc.) are returned as a plain
//! `ToolError`, which `rmcp` renders into a tool result with `isError: true` — not as a
//! JSON-RPC error, per MCP spec.

use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use egui::Event;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        tool::ToolCallContext,
        wrapper::{Json, Parameters},
    },
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
// UiServer + Server
// ---------------------------------------------------------------------------------------

/// The reusable core: the egui UI/inspection tools (`click`, `type_text`, `screenshot`,
/// `query_tree`, `batch`, …) bound to one live [`Bridge`].
///
/// It holds *only* the bridge — so a `UiServer` always has an app to drive, and its mere
/// existence is the "connected" state (a host represents "disconnected" as the absence of a
/// `UiServer`, e.g. [`Server`]'s `Option<UiServer>`).
///
/// It owns no connection logic and no router. A host builds the bridge however it likes (TCP,
/// a tunnelled transport, …), wraps it with [`UiServer::new`], and pairs it with the router
/// from [`UiServer::router`]: `router.list_all()` lists the commands for the host's
/// `list_tools`, and [`UiServer::dispatch`] runs one — so another MCP server can offer its own
/// connection tools alongside this shared command set.
pub struct UiServer {
    bridge: Bridge,
}

impl UiServer {
    /// Wrap a live [`Bridge`] as a UI server.
    pub fn new(bridge: Bridge) -> Self {
        Self { bridge }
    }

    /// Build the router over the UI/inspection tools.
    ///
    /// The router is independent of any `UiServer` instance — list its tools while
    /// disconnected, then dispatch against a `UiServer` once one exists (this is exactly what
    /// [`Server`] does, and what an embedding server should do too).
    pub fn router() -> ToolRouter<Self> {
        Self::tool_router()
    }

    /// Run one UI tool against this server's bridge, via a router from [`UiServer::router`].
    ///
    /// For an embedding server's `call_tool` to delegate to.
    ///
    /// # Errors
    /// If the tool name is unknown (recoverable failures are reported in the `CallToolResult`).
    pub async fn dispatch(
        &self,
        router: &ToolRouter<Self>,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tcc = ToolCallContext::new(self, request, context);
        router.call(tcc).await
    }

    /// The bridge this server drives.
    fn bridge(&self) -> &Bridge {
        &self.bridge
    }
}

/// egui-mcp's own MCP server: the TCP connection lifecycle plus the UI tools.
///
/// It owns the UI tools' [`ToolRouter`] directly and the attached [`UiServer`] (if any) behind
/// a lock. The router is independent of the connection, so the app-driving tools stay
/// discoverable while disconnected; calling one before `attach` returns a recoverable
/// `no app connected` error.
#[derive(Clone)]
pub struct Server {
    /// The attached UI server, `Some` only while connected. Behind a shared lock so the
    /// `&self` handler methods (and clones rmcp may make) can attach/detach it.
    ui: Arc<Mutex<Option<UiServer>>>,
    ui_router: ToolRouter<UiServer>,
    lifecycle_router: ToolRouter<Self>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            ui: Arc::new(Mutex::new(None)),
            ui_router: UiServer::router(),
            lifecycle_router: Self::lifecycle_router(),
        }
    }

    fn tools(&self) -> Vec<Tool> {
        let mut tools = self.lifecycle_router.list_all();
        tools.extend(self.ui_router.list_all());
        tools
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

fn text_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(msg.into())])
}

/// A recoverable tool failure (no app connected, node not found, bad argument, a bridge I/O
/// error, …), carried as a plain message string.
///
/// It is *not* a JSON-RPC protocol error: a `String` already implements `rmcp`'s `IntoContents`,
/// so when a `#[tool]` method returns `Err(ToolError)`, `rmcp` renders it into a `CallToolResult`
/// with `isError: true` (per the MCP spec). `String` is also the [`Bridge`]'s error type, so the
/// inner handlers `?`-propagate bridge failures with no conversion.
type ToolError = String;

/// The result of an app-driving tool handler — see [`ToolError`].
type ToolResult<T> = Result<T, ToolError>;

// ---------------------------------------------------------------------------------------
// Target — locator OR raw position, shared by click / hover / scroll / drag.
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct Target {
    /// Node id from `query_tree`.
    #[serde(default)]
    pub id: Option<String>,

    /// Role name, e.g. `Button`, `Label`, `TextInput` (case-insensitive).
    /// An unrecognized role errors with the roles present in the tree.
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
        Locator::from_fields(
            self.id.as_deref(),
            self.role.clone(),
            self.label_contains.clone(),
        )
    }
}

/// Resolve a [`Target`] to an optional node id + a logical-point position, fetching a fresh
/// tree first. Use [`resolve_in_tree`] directly when resolving several targets against one
/// snapshot (e.g. a drag's start and end).
async fn resolve_target(
    bridge: &Bridge,
    target: &Target,
) -> ToolResult<(Option<String>, egui::Pos2)> {
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
) -> ToolResult<(Option<String>, egui::Pos2)> {
    if let Some(p) = target.pos {
        return Ok((None, egui::Pos2::new(p.x, p.y)));
    }
    if let Some(role) = &target.role {
        tree::validate_role(role, snap.tree.as_ref())?;
    }
    let locator = target
        .as_locator()
        .ok_or("target requires `id`, `role`, `label_contains`, or `pos`")?;
    let tree = snap.tree.as_ref().ok_or("no accesskit tree yet")?;
    let node = tree::resolve_unique(tree, &locator, snap.pixels_per_point)?;
    let view = tree::node_view(&node, snap.pixels_per_point);
    let bounds = view.bounds.ok_or("node has no bounds — can't target")?;
    // `node_view` already returns logical-point bounds, so the center needs no further scaling.
    let (cx, cy) = bounds.center();
    let center = egui::Pos2::new(cx as f32, cy as f32);
    Ok((Some(view.id), center))
}

// ---------------------------------------------------------------------------------------
// Args structs
// ---------------------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AttachArgs {
    /// Host the app's inspection port is on.
    /// Defaults to `127.0.0.1`.
    #[serde(default = "default_host")]
    pub host: String,

    /// Port the app is listening on.
    /// Defaults to `5719` (`egui_inspection`'s default).
    #[serde(default = "default_port")]
    pub port: u16,

    /// How long to keep retrying the connection, in seconds.
    /// Defaults to 10.
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
pub struct ScreenshotArgs {
    /// Output resolution in pixels per logical point.
    /// Defaults to `1.0`, which makes screenshot pixels line up 1:1 with the logical coordinates used by `click`/`query_tree`.
    /// Higher values give a sharper image, capped at the display's native scale (no upscaling).
    #[serde(default = "default_pixels_per_point")]
    pub pixels_per_point: f32,

    /// If set, also write the PNG to this path on the machine running the MCP server (in addition to returning it inline).
    #[serde(default)]
    pub save_path: Option<String>,
}

fn default_pixels_per_point() -> f32 {
    1.0
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

    /// `2` → double-click; `3` → triple-click.
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
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScrollArgs {
    #[serde(flatten)]
    pub target: Target,

    /// Logical points.
    /// Positive Y scrolls down (reveals content below); positive X scrolls right.
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
    /// Role name, e.g. `Button`, `Label`, `TextInput` (case-insensitive).
    /// An unrecognized role errors with the roles present in the tree.
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub label_contains: Option<String>,
    #[serde(default = "default_wait_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_min_matches")]
    pub min_matches: u32,

    /// Also wait until at least this many frames have rendered since the call began.
    /// Use it to let animations, tooltips, or other time/frame-driven UI settle (e.g. after a `hover`); `0` means don't wait for frames.
    #[serde(default)]
    pub min_steps: u64,
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

    /// Optional focus target: node id from `query_tree` to focus before typing.
    /// Omit all locator fields to type into whatever is currently focused.
    #[serde(default)]
    pub id: Option<String>,

    /// Role name, e.g. `Button`, `Label`, `TextInput` (case-insensitive).
    /// An unrecognized role errors with the roles present in the tree.
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
// Structured tool outputs — MCP requires `structuredContent` to be a JSON object, so
// collection/optional results are wrapped in a named field rather than returned bare.
// ---------------------------------------------------------------------------------------

/// `query_tree` result: the matching nodes.
#[derive(Debug, Serialize, JsonSchema)]
pub struct QueryTreeResult {
    pub nodes: Vec<NodeView>,
}

/// `get_node` result: the resolved node, or `null` if the id didn't match.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GetNodeResult {
    pub node: Option<NodeView>,
}

/// Lifecycle tools — connection management, served directly by [`Server`].
#[tool_router(router = lifecycle_router)]
impl Server {
    /// Connect to a running egui app's inspection port (an app built with eframe's `inspection` feature, launched with `EGUI_INSPECTION` set).
    /// Defaults to 127.0.0.1:5719.
    /// Retries until `timeout_secs` elapses.
    /// On success the app-driving tools start working (they are always listed, but error until an app is attached).
    #[tool]
    async fn attach(
        &self,
        Parameters(args): Parameters<AttachArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut guard = self.ui.lock().await;
        if guard.is_some() {
            return Ok(text_error(
                "already connected — call `disconnect` first before attaching again",
            ));
        }
        let timeout = args.timeout_secs.map(Duration::from_secs);
        let bridge = match Bridge::connect(&args.host, args.port, timeout).await {
            Ok(b) => b,
            Err(e) => return Ok(text_error(format!("attach failed: {e}"))),
        };
        let info = bridge.peer_info.clone();
        *guard = Some(UiServer::new(bridge));
        Ok(CallToolResult::structured(
            json!({ "ok": true, "attached": info }),
        ))
    }

    /// Disconnect from the attached app.
    /// App-driving tools remain available but return an error until `attach` is called again.
    #[tool]
    async fn disconnect(&self, _p: Parameters<EmptyArgs>) -> Result<CallToolResult, McpError> {
        if self.ui.lock().await.take().is_some() {
            Ok(CallToolResult::structured(json!({ "ok": true })))
        } else {
            Ok(text_error("no app connected"))
        }
    }

    /// Report whether an app is connected and its peer info.
    #[tool]
    async fn status(&self, _p: Parameters<EmptyArgs>) -> Result<CallToolResult, McpError> {
        let guard = self.ui.lock().await;
        let body = match guard.as_ref() {
            None => json!({ "state": "idle" }),
            Some(ui) => json!({ "state": "connected", "peer": ui.bridge().peer_info }),
        };
        Ok(CallToolResult::structured(body))
    }
}

/// App-driving tools — each needs an attached app.
#[tool_router]
impl UiServer {
    /// Capture the current frame as a PNG screenshot.
    /// Defaults to logical-point resolution (`pixels_per_point: 1.0`) so pixels align with `click`/`query_tree` coordinates; pass a higher `pixels_per_point` for detail, or `save_path` to also write it to disk.
    /// Requires the app window to be visible — a fully-occluded or minimized window can't render a frame to capture (notably on macOS), so the call times out; bring the window to the foreground first.
    #[tool]
    async fn screenshot(
        &self,
        Parameters(args): Parameters<ScreenshotArgs>,
    ) -> ToolResult<CallToolResult> {
        let bridge = self.bridge();
        let png = bridge.screenshot(args.pixels_per_point).await?;
        let saved = match &args.save_path {
            Some(path) => {
                std::fs::write(path, &png.bytes).map_err(|e| format!("save to `{path}`: {e}"))?;
                Some(path.as_str())
            }
            None => None,
        };
        let png_b64 = base64::engine::general_purpose::STANDARD.encode(&png.bytes);
        let meta = json!({ "width": png.size[0], "height": png.size[1], "saved_to": saved });
        Ok(CallToolResult::success(vec![
            Content::text(meta.to_string()),
            Content::image(png_b64, "image/png"),
        ]))
    }

    // The return type is spelled `Result<Json<…>, ToolError>` rather than the `ToolResult` alias
    // on purpose: `#[tool]` derives the output schema by syntactically matching `Json<T>` /
    // `Result<Json<T>, _>`, and the alias would hide it, silently dropping the schema.
    /// Walk the widget tree and return nodes matching the filter.
    /// `role`, if given, is a role name (e.g. `Button`, `Label`), matched case-insensitively; an unknown role errors with the roles present in the tree.
    /// Use the returned `id` with `click`, `type_text`, or `get_node`.
    #[tool]
    async fn query_tree(
        &self,
        Parameters(filter): Parameters<QueryFilter>,
    ) -> Result<Json<QueryTreeResult>, ToolError> {
        let bridge = self.bridge();
        let snap = bridge.fetch_tree().await?;
        if let Some(role) = &filter.role {
            tree::validate_role(role, snap.tree.as_ref())?;
        }
        let nodes = match snap.tree {
            Some(tree) => tree::query(&tree, &filter, snap.pixels_per_point),
            None => Vec::new(),
        };
        Ok(Json(QueryTreeResult { nodes }))
    }

    /// Return a single node by id (from `query_tree`).
    // Spelled-out `Result<Json<…>, ToolError>` (not the `ToolResult` alias) so `#[tool]` derives
    // the output schema — see `query_tree`.
    #[tool]
    async fn get_node(
        &self,
        Parameters(args): Parameters<GetNodeArgs>,
    ) -> Result<Json<GetNodeResult>, ToolError> {
        let bridge = self.bridge();
        let id = args
            .id
            .trim()
            .parse::<u64>()
            .map_err(|e| format!("invalid id `{}`: {e}", args.id))?;
        let locator = Locator::Id { id };
        let snap = bridge.fetch_tree().await?;
        let ppp = snap.pixels_per_point;
        // `get_node` is a lookup, not an action: a missing id is `null`, not an error.
        let node = match snap.tree {
            Some(tree) => tree::resolve_unique(&tree, &locator, ppp)
                .ok()
                .map(|n| tree::node_view(&n, ppp)),
            None => None,
        };
        Ok(Json(GetNodeResult { node }))
    }

    /// Click the center of a node's bounding box, or a raw `pos` in logical points.
    /// Specify either a locator (`id` from `query_tree` or `role`/`label_contains`) or `pos: { x, y }`.
    /// `button` defaults to `primary` (accepts `primary`/`secondary`/`middle`/`extra1`/`extra2`, or aliases `left`/`right`).
    /// `count: 2` → double-click, `3` → triple.
    #[tool]
    async fn click(&self, Parameters(args): Parameters<ClickArgs>) -> ToolResult<CallToolResult> {
        let bridge = self.bridge();
        Ok(CallToolResult::structured(click_inner(bridge, args).await?))
    }

    /// Move the pointer over a node (or raw `pos`) without clicking.
    /// Tooltips and hover popups only appear after a short delay — follow with `wait_for` (e.g. its `min_steps`) to let them settle before reading the tree or screenshotting.
    #[tool]
    async fn hover(&self, Parameters(args): Parameters<HoverArgs>) -> ToolResult<CallToolResult> {
        let bridge = self.bridge();
        let (node_id, pos) = resolve_target(bridge, &args.target).await?;
        bridge.apply_events(vec![Event::PointerMoved(pos)]).await?;
        Ok(CallToolResult::structured(json!({
            "ok": true,
            "hovered_id": node_id,
            "pos": [pos.x, pos.y],
        })))
    }

    /// Send a mouse wheel scroll over a node (or raw `pos`).
    /// `delta` is in logical points: positive Y scrolls down (reveals content below); positive X scrolls right.
    #[tool]
    async fn scroll(&self, Parameters(args): Parameters<ScrollArgs>) -> ToolResult<CallToolResult> {
        let bridge = self.bridge();
        let (node_id, pos) = resolve_target(bridge, &args.target).await?;
        let modifiers = args.modifiers.to_egui();
        let events = vec![
            Event::PointerMoved(pos),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                // Match scroll direction to the convention of playwright and other tools by inverting it
                delta: egui::Vec2::new(-args.delta.x, -args.delta.y),
                phase: egui::TouchPhase::Move,
                modifiers,
            },
        ];
        bridge.apply_events(events).await?;
        Ok(CallToolResult::structured(json!({
            "ok": true,
            "scrolled_id": node_id,
            "pos": [pos.x, pos.y],
            "delta": [args.delta.x, args.delta.y],
        })))
    }

    /// Primary-button drag from `start` to `end`.
    /// Each target accepts the same fields as `click`: locator (`id`/`role`/`label_contains`) or `pos: {x, y}`.
    /// `steps` controls how many intermediate pointer-move events are emitted between press and release.
    #[tool]
    async fn drag(&self, Parameters(args): Parameters<DragArgs>) -> ToolResult<CallToolResult> {
        let bridge = self.bridge();
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
        Ok(CallToolResult::structured(json!({
            "ok": true,
            "start_id": start_id,
            "end_id": end_id,
            "start_pos": [start_pos.x, start_pos.y],
            "end_pos": [end_pos.x, end_pos.y],
            "steps": steps,
        })))
    }

    /// Resize the app's viewport to the given logical-point dimensions.
    #[tool]
    async fn resize(&self, Parameters(args): Parameters<ResizeArgs>) -> ToolResult<CallToolResult> {
        let bridge = self.bridge();
        bridge.resize(args.width, args.height).await?;
        Ok(CallToolResult::structured(
            json!({ "ok": true, "width": args.width, "height": args.height }),
        ))
    }

    /// Poll the widget tree until its conditions hold, or until `timeout_secs` elapses.
    /// Waits until at least `min_matches` visible nodes match the `role`/`label_contains` filter (when one is given) *and* at least `min_steps` frames have rendered since the call began.
    /// Requires `role`, `label_contains`, or a non-zero `min_steps`.
    #[tool]
    async fn wait_for(
        &self,
        Parameters(args): Parameters<WaitForArgs>,
    ) -> ToolResult<CallToolResult> {
        let bridge = self.bridge();
        let has_filter = args.role.is_some() || args.label_contains.is_some();
        if !has_filter && args.min_steps == 0 {
            return Err(
                "wait_for requires `role`, `label_contains`, or a non-zero `min_steps`".to_owned(),
            );
        }
        let filter = QueryFilter {
            role: args.role.clone(),
            label_contains: args.label_contains.clone(),
            visible_only: true,
            limit: args.min_matches as usize,
        };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(args.timeout_secs);
        let mut start_step = None;
        loop {
            let snap = bridge.fetch_tree().await?;
            // Baseline off the first observed frame, so `min_steps` counts frames since the call.
            let steps_waited = snap
                .step
                .saturating_sub(*start_step.get_or_insert(snap.step));
            // Fail fast on a typo'd role rather than polling until timeout, listing the roles
            // currently in the tree. A valid-but-absent role passes and keeps polling.
            if let Some(role) = &filter.role {
                tree::validate_role(role, snap.tree.as_ref())?;
            }
            let matches: Vec<NodeView> = match (has_filter, snap.tree) {
                (true, Some(tree)) => tree::query(&tree, &filter, snap.pixels_per_point),
                _ => Vec::new(),
            };
            let matched_ok = !has_filter || matches.len() as u32 >= args.min_matches;
            if matched_ok && steps_waited >= args.min_steps {
                return Ok(CallToolResult::structured(
                    json!({ "ok": true, "matched": matches, "steps_waited": steps_waited }),
                ));
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "wait_for timed out after {}s (role={:?}, label_contains={:?}, found {}, min_steps={}, steps_waited={})",
                    args.timeout_secs,
                    args.role,
                    args.label_contains,
                    matches.len(),
                    args.min_steps,
                    steps_waited,
                ));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Type text into the currently focused widget.
    /// Optionally focus a node first (by `id` or `role`/`label_contains`) — this uses an AccessKit focus request, not a click, so it won't move the cursor or clear an existing text selection.
    #[tool]
    async fn type_text(
        &self,
        Parameters(args): Parameters<TypeTextArgs>,
    ) -> ToolResult<CallToolResult> {
        let bridge = self.bridge();
        // Optionally focus a target first. Unlike a click, an AccessKit focus request doesn't
        // move the text cursor or reset the selection.
        let focused_id = match Locator::from_fields(
            args.id.as_deref(),
            args.role.clone(),
            args.label_contains,
        ) {
            Some(locator) => {
                let snap = bridge.fetch_tree().await?;
                if let Some(role) = &args.role {
                    tree::validate_role(role, snap.tree.as_ref())?;
                }
                let tree = snap.tree.as_ref().ok_or("no accesskit tree yet")?;
                let node = tree::resolve_unique(tree, &locator, snap.pixels_per_point)?;
                let id = tree::accesskit_id(&node);
                bridge
                    .apply_events(vec![Event::AccessKitActionRequest(
                        accesskit::ActionRequest {
                            action: accesskit::Action::Focus,
                            target_tree: accesskit::TreeId::ROOT,
                            target_node: accesskit::NodeId(id),
                            data: None,
                        },
                    )])
                    .await?;
                Some(id.to_string())
            }
            None => None,
        };

        if !args.text.is_empty() {
            bridge.apply_events(vec![Event::Text(args.text)]).await?;
        }

        Ok(CallToolResult::structured(json!({
            "ok": true,
            "focused_id": focused_id,
        })))
    }

    /// Send a key press (down + up) to the focused widget.
    /// `key` is an egui key name such as `Backspace`, `Delete`, `Enter`, `Tab`, `A`–`Z`, `ArrowLeft`, `ArrowRight`, `Home`, `End`, `Escape`.
    #[tool]
    async fn press_key(
        &self,
        Parameters(args): Parameters<PressKeyArgs>,
    ) -> ToolResult<CallToolResult> {
        let bridge = self.bridge();
        let key =
            egui::Key::from_name(&args.key).ok_or_else(|| format!("unknown key `{}`", args.key))?;
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
        Ok(CallToolResult::structured(
            json!({ "ok": true, "key": args.key }),
        ))
    }

    /// Execute a sequence of app-driving tool calls in one round trip (the connection tools `attach`/`disconnect`/`status` are not available here).
    /// Stops on the first error.
    /// Results are emitted in execution order, interleaved: each step contributes one JSON text item followed by any image items it produced (e.g. screenshots).
    /// `batch` cannot be nested.
    /// Use this to act and observe in one call, e.g. a `click` then a `query_tree` or `screenshot`.
    #[tool]
    async fn batch(
        &self,
        Parameters(args): Parameters<BatchArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        if args.actions.iter().any(|a| a.name == "batch") {
            return Ok(text_error("nested `batch` is not allowed"));
        }
        let mut content: Vec<Content> = Vec::new();
        let mut any_error = false;
        // Re-enter the UI router by name — same path as a top-level call, so each step parses
        // its args and runs exactly like a direct invocation.
        let router = Self::router();
        for action in args.actions {
            let mut request = CallToolRequestParams::new(action.name.clone());
            if let Value::Object(map) = action.args {
                request = request.with_arguments(map);
            }
            let result = self
                .dispatch(&router, request, ctx.clone())
                .await
                .unwrap_or_else(|e| text_error(e.message.to_string()));
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

/// Operating guidance sent to clients at initialize (the MCP `instructions` field). The
/// per-tool descriptions cover each command in isolation; this establishes the cross-cutting
/// workflow — the observe→act→verify loop and the prefer-locators convention — that an agent
/// otherwise has to infer.
const INSTRUCTIONS: &str = r#"This mcp drives a live egui app: it reads the app's accessibility tree and synthesizes real input events. Work in an observe → act → verify loop.

Getting oriented:
- Call `attach` first (check `status` if unsure); the app-driving tools return "no app connected" until then.
- Start most tasks with `query_tree` to discover widgets and their ids, and/or `screenshot` to see the rendered frame.

Targeting widgets:
- Prefer locators — an `id` from `query_tree`, or `role`/`label_contains` — over a raw `pos`. Locators resolve to the widget's current position and survive layout changes; reach for `pos` only when nothing matches.
- A locator in an action must match exactly one node. If `role`/`label_contains` matches several, the call errors and lists the candidates — narrow the filter or target a specific `id`. Use `query_tree` when you want every match.

Acting and verifying:
- After an action that changes the UI, confirm it landed: `query_tree` for the expected state, `screenshot` to look, or `wait_for` to poll until async or animated UI settles.
- Use `batch` to act and observe in one round trip (e.g. `click` then `screenshot`), avoiding an extra turn.

Conventions:
- Everything is in logical points, one shared coordinate frame: raw `pos`, `resize` dimensions, the `bounds` from `query_tree`/`get_node`, and a default (`pixels_per_point: 1.0`) `screenshot`. So a node's `bounds` center is exactly where to `click`, and a pixel in the screenshot is a logical point. There is no fixed screen size; use `resize` to set the viewport."#;

impl ServerHandler for Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("egui-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            tools: self.tools(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Lifecycle tools run on `self`; everything else is delegated to the attached UI server.
        if self.lifecycle_router.has_route(&request.name) {
            let tcc = ToolCallContext::new(self, request, context);
            return self.lifecycle_router.call(tcc).await;
        }
        let guard = self.ui.lock().await;
        let Some(ui) = guard.as_ref() else {
            return Ok(text_error("no app connected — call `attach` first"));
        };
        ui.dispatch(&self.ui_router, request, context).await
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        if let Some(tool) = self.lifecycle_router.get(name) {
            return Some(tool.clone());
        }
        self.ui_router.get(name).cloned()
    }
}

// ---------------------------------------------------------------------------------------
// Batch result flattening
// ---------------------------------------------------------------------------------------

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
// Shared action helpers — `click_inner` is reused by `type_text`'s focus-click;
// `parse_pointer_button` by `click_inner`.
// ---------------------------------------------------------------------------------------

fn parse_pointer_button(name: &str) -> ToolResult<egui::PointerButton> {
    match name.to_ascii_lowercase().as_str() {
        "primary" | "left" => Ok(egui::PointerButton::Primary),
        "secondary" | "right" => Ok(egui::PointerButton::Secondary),
        "middle" => Ok(egui::PointerButton::Middle),
        "extra1" => Ok(egui::PointerButton::Extra1),
        "extra2" => Ok(egui::PointerButton::Extra2),
        other => Err(format!(
            "unknown button `{other}` — expected primary/secondary/middle/extra1/extra2 (or left/right)"
        )),
    }
}

async fn click_inner(bridge: &Bridge, args: ClickArgs) -> ToolResult<Value> {
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

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use rmcp::ServerHandler as _;

    use super::*;

    /// Snapshot the entire agent-facing surface: the server `instructions` plus every tool's
    /// name, description, and input/output schemas — exactly what an MCP client is shown on
    /// connect. Guards descriptions, schemas, and structured-output shapes against accidental
    /// change. Run `INSTA_UPDATE=always cargo nextest run -p egui_mcp` to accept intended edits.
    ///
    /// The server version is intentionally excluded so the snapshot is stable across releases.
    #[test]
    fn agent_surface_snapshot() {
        let server = Server::new();

        let mut surface = String::new();
        surface.push_str("# Server instructions\n\n");
        surface.push_str(
            server
                .get_info()
                .instructions
                .as_deref()
                .unwrap_or("(none)"),
        );
        surface.push_str("\n\n# Tools\n");

        let mut tools = server.tools();
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        for tool in &tools {
            write!(surface, "\n## {}\n\n", tool.name).unwrap();
            surface.push_str(&serde_json::to_string_pretty(tool).expect("serialize tool"));
            surface.push('\n');
        }

        insta::assert_snapshot!("agent_surface", surface);
    }
}
