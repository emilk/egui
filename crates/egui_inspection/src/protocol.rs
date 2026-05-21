//! Wire protocol shared between an egui peer (an `egui_kittest::Harness` or a live
//! `eframe` app running [`crate::InspectionPlugin`]) and an external inspector
//! (the standalone `kittest_inspector` UI binary, or the `egui_kittest_mcp` server).
//!
//! The egui peer writes [`HarnessMessage`]s (frames plus blocking-state updates) into the
//! transport. The inspector writes [`InspectorCommand`]s back to drive the peer. Shutdown
//! is detected on either side via EOF — no explicit goodbye message.
//!
//! Messages are framed as a 4-byte big-endian length followed by a MessagePack-encoded body
//! (`rmp-serde`). Transport-neutral: the same framing works on stdio, unix sockets, and TCP.
//!
//! Living in its own crate (rather than `egui_kittest`) lets eframe pull the protocol in
//! without picking up the test harness, and lets external tools depend on it directly.

use std::io::{self, Read, Write};

use egui::accesskit;

/// Wire-protocol version sent in [`PeerHello::protocol_version`]. Bump whenever a
/// non-additive change is made to [`HarnessMessage`] / [`InspectorCommand`] / their
/// payload structs. The inspector should refuse peers with a higher major version than
/// it understands.
pub const PROTOCOL_VERSION: u32 = 1;

/// What kind of egui peer the inspector is talking to. Determines which controls the
/// inspector UI should render (Step / Pause buttons make no sense against a live app).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PeerKind {
    /// A deterministic `egui_kittest::Harness` — supports stepping, pause/play, panic
    /// capture, and source highlighting.
    Kittest,
    /// A live `eframe` app running [`crate::InspectionPlugin`] — no deterministic run
    /// loop, no panic capture, no source view.
    Live,
}

/// Which optional [`InspectorCommand`] variants the peer honors. The inspector should
/// hide / disable UI for commands whose capability is `false`.
///
/// `Handle` is always supported (no flag) — every peer accepts event injection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Capabilities {
    /// Peer honors [`InspectorCommand::Step`].
    pub step: bool,
    /// Peer honors [`InspectorCommand::Run`].
    pub run: bool,
    /// Peer honors [`InspectorCommand::Play`] / [`InspectorCommand::Pause`].
    pub play_pause: bool,
    /// Peer honors [`InspectorCommand::Screenshot`].
    pub screenshot: bool,
    /// Peer honors [`InspectorCommand::SetContinuousScreenshots`] — i.e. it can be asked to
    /// attach a fresh [`FrameScreenshot`] to every outgoing [`Frame`] until told to stop.
    pub continuous_screenshots: bool,
    /// Peer honors [`InspectorCommand::Resize`].
    pub resize: bool,
}

impl Capabilities {
    /// Capabilities of a deterministic kittest harness: all execution-control commands plus
    /// both one-shot and continuous screenshot modes. The harness ships with continuous on
    /// by default (matching the pre-flag behavior of always-fresh frames); the inspector
    /// can flip it off via [`InspectorCommand::SetContinuousScreenshots`]`(false)` to skip
    /// the per-step render cost when it only needs the accesskit tree.
    pub const KITTEST: Self = Self {
        step: true,
        run: true,
        play_pause: true,
        screenshot: true,
        continuous_screenshots: true,
        resize: true,
    };

    /// Capabilities of a live `eframe` app: no execution-control (no own run loop), but
    /// the integration honors viewport-level screenshot and resize requests, and the
    /// plugin can be flipped into per-frame screenshot mode.
    pub const LIVE: Self = Self {
        step: false,
        run: false,
        play_pause: false,
        screenshot: true,
        continuous_screenshots: true,
        resize: true,
    };
}

/// First [`HarnessMessage`] sent on every connection. Identifies the peer and declares
/// which optional commands it will honor. The inspector should treat the absence of a
/// `Hello` (i.e. a `Frame` arriving first) as a protocol error.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerHello {
    /// [`PROTOCOL_VERSION`] of the peer.
    pub protocol_version: u32,
    pub peer_kind: PeerKind,
    pub capabilities: Capabilities,
    /// Whether the peer starts in continuous-screenshot mode (i.e. attaches a
    /// [`FrameScreenshot`] to every `Frame` until told otherwise). Inspectors should treat
    /// this as the authoritative initial state rather than relying on per-peer defaults.
    /// Only meaningful when [`Capabilities::continuous_screenshots`] is `true`.
    pub continuous_screenshots: bool,
    /// Human-readable identifier (test name, app name). Replaces the per-`Frame` label.
    pub label: Option<String>,
}

/// One source file plus the test-source lines the inspector should highlight inside it.
///
/// The harness captures `#[track_caller]` locations for the `.run()`/`.step()` call that
/// produced the frame and for each event consumed by it. The inspector highlights
/// [`Self::call_site_line`] for the runner call and [`Self::event_lines`] for each event.
///
/// Only populated by `egui_kittest`. Live apps (via [`crate::InspectionPlugin`]) leave this
/// `None` on every [`Frame`] — they have no test source to point at.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceView {
    /// Absolute or crate-relative path as reported by `std::panic::Location::file`.
    pub path: String,
    /// Entire file contents, lines separated by `\n`. `None` if the file couldn't be read.
    pub contents: Option<String>,
    /// Line number of the `.run()` / `.step()` call that produced this frame.
    pub call_site_line: Option<u32>,
    /// Line numbers of events consumed by this frame's step, in queue order.
    pub event_lines: Vec<u32>,
    /// Line number of a panic captured in this file. The inspector highlights this line in
    /// red. Set on the [`HarnessMessage::Finished`] source view when a panic was captured.
    pub panic_line: Option<u32>,
}

/// Rendered framebuffer attached to a [`Frame`]. Absent on accesskit-only frames (live
/// apps default to "tree-only" until the inspector asks for screenshots).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrameScreenshot {
    /// Image width in physical pixels.
    pub width: u32,
    /// Image height in physical pixels.
    pub height: u32,
    /// Tightly packed RGBA8 pixels (length = `width * height * 4`). `serde_bytes` encodes
    /// this as a msgpack `bin` blob (one type tag + raw bytes) instead of the default
    /// `Vec<u8>` path of one type tag *per byte*, which would roughly double on-wire size.
    #[serde(with = "serde_bytes")]
    pub rgba: Vec<u8>,
}

/// A single update from the egui peer: accesskit tree + optional screenshot.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Frame {
    /// Monotonically increasing step counter.
    pub step: u64,
    /// `physical_pixel = logical_point * pixels_per_point`. AccessKit bounds are in logical
    /// coords, the screenshot is in physical pixels — multiply by this to align them.
    pub pixels_per_point: f32,
    /// Rendered framebuffer for this step, when available. `None` for live-app frames
    /// outside continuous-screenshot mode that didn't receive an `Event::Screenshot` reply
    /// (i.e. accesskit-only updates). Kittest harnesses populate this on every frame.
    pub screenshot: Option<FrameScreenshot>,
    /// Latest accesskit tree update, if any.
    pub accesskit: Option<accesskit::TreeUpdate>,
    /// The test source file associated with this frame + the lines to highlight inside it.
    /// `None` for live apps.
    pub source: Option<SourceView>,
}

/// Sent egui-peer → inspector. Always begins with a single [`Self::Hello`]. After that,
/// frames carry rendered images; `Blocked` signals when the harness's blocking state
/// changes without a visual update (e.g. at `after_run`, where nothing has re-rendered
/// since the last `after_step`). Live apps never send `Blocked`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HarnessMessage {
    /// Identifies the peer and declares its capabilities. Sent exactly once, as the very
    /// first message on the connection, before any [`Self::Frame`].
    Hello(PeerHello),
    /// A new frame (image + tree + source) is available.
    Frame(Box<Frame>),
    /// The peer is now either blocked (`true`) waiting for an [`InspectorCommand`], or
    /// running freely (`false`).
    Blocked(bool),
    /// The test has ended. Implies [`Self::Blocked`]`(true)`: the harness blocks after
    /// sending this, and any subsequent `Step` / `Run` / `Play` command dismisses the result
    /// and lets the harness drop.
    ///
    /// Live apps never send this.
    Finished {
        /// `true` on pass; `false` if a panic was in progress when the harness dropped.
        ok: bool,
        /// Panic message, if captured (requires `egui_kittest::install_panic_hook()`).
        message: Option<String>,
        /// Final-frame source context: the test entry point's file, with the panic line (if
        /// any and if it matches that file) recorded in [`SourceView::panic_line`].
        source: Option<SourceView>,
    },
}

/// Sent inspector → egui peer at any time to drive execution.
///
/// `egui_kittest` blocks at `after_step` / `after_run` hooks (and at those hooks only).
/// Which command it waits for, and whether it returns to blocking after executing one,
/// depends on the command that last arrived — see each variant's docs.
///
/// Live apps (via [`crate::InspectionPlugin`]) treat `Step` / `Run` / `Play` / `Pause` as
/// no-ops — they don't own a deterministic run loop. `Handle` is honored on the next frame.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum InspectorCommand {
    /// Advance one frame, then block at the next `after_step`.
    Step,
    /// Run until the next `after_run` hook fires, then block.
    Run,
    /// Run freely until a [`Self::Pause`], [`Self::Step`], or [`Self::Run`] command arrives.
    /// Frames keep streaming while playing — the inspector may send [`Self::Handle`] at any
    /// point without interrupting play.
    Play,
    /// Cancel [`Self::Play`] (no-op when already blocked).
    Pause,
    /// Queue these events on the peer and run a single step. Does not change the peer's
    /// Pause / Play / Run state.
    Handle { events: Vec<egui::Event> },
    /// Request a full-framebuffer screenshot for the next frame.
    ///
    /// Live apps (via [`crate::InspectionPlugin`]) issue a
    /// [`egui::ViewportCommand::Screenshot`], intercept the resulting
    /// [`egui::Event::Screenshot`], and emit a [`HarnessMessage::Frame`] with
    /// [`Frame::screenshot`] populated. The deterministic kittest path already attaches a
    /// screenshot to every frame, so it treats this as a no-op.
    Screenshot,
    /// Toggle continuous screenshot mode. While `true`, the peer attaches a fresh
    /// [`FrameScreenshot`] to every outgoing [`Frame`] until told otherwise. Useful for
    /// inspectors that always want a current image (mirror the app's window) without
    /// having to issue per-step [`Self::Screenshot`] requests.
    ///
    /// Kittest harnesses ignore this (they already screenshot every frame).
    SetContinuousScreenshots(bool),
    /// Resize the peer's viewport / harness to the given logical-point dimensions.
    ///
    /// Live apps issue a [`egui::ViewportCommand::InnerSize`]. The deterministic kittest
    /// path calls `Harness::set_size`.
    Resize { width: u32, height: u32 },
}

/// Hard cap on a single framed message. Matches the sanity limit enforced by both ends.
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024 * 1024; // 256 MiB

/// Read a length-prefixed MessagePack message.
///
/// # Errors
/// I/O or decode failures.
pub fn read_message<R, T>(mut reader: R) -> io::Result<T>
where
    R: Read,
    T: for<'de> serde::Deserialize<'de>,
{
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("message too large: {len} bytes"),
        ));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    rmp_serde::from_slice(&buf)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

/// Write a length-prefixed MessagePack message.
///
/// # Errors
/// I/O or encode failures.
pub fn write_message<W, T>(mut writer: W, value: &T) -> io::Result<()>
where
    W: Write,
    T: serde::Serialize,
{
    let bytes = rmp_serde::to_vec(value)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    let len = u32::try_from(bytes.len())
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    writer.write_all(&len.to_be_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}
