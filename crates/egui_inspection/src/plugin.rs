//! [`InspectionPlugin`] — an [`egui::Plugin`] that streams frames + AccessKit tree updates
//! to an inspector over a local socket and applies received commands back into the
//! running app.
//!
//! Connection model:
//! - The inspector binds a local socket. The egui peer dials it.
//! - The plugin spawns one reader thread and one writer thread, each owning one half of the
//!   stream. UI-thread hooks (`input_hook` / `output_hook`) only touch in-process channels
//!   and the reader-side command queue.
//! - If the writer channel is saturated, the plugin drops the oldest frame in favor of the
//!   newest so the UI thread never blocks on a slow inspector.
//!
//! Live apps don't own a deterministic run loop, so `Step` / `Run` / `Play` / `Pause`
//! commands are no-ops. `Handle { events }` is honored by appending the events to the next
//! `RawInput`. After every received command the reader thread calls
//! `Context::request_repaint` so the integration wakes up even when the UI is otherwise
//! idle — without this, queued events would sit in the channel until the next mouse move.
//!
//! # Reference cycle
//!
//! The plugin holds a clone of `egui::Context` so the reader thread can wake the UI loop.
//! `egui::Context` is `Arc<RwLock<…>>` and the context owns its plugins, so this creates an
//! intentional cycle: the context will not drop until the process exits. Acceptable for a
//! live-debugging inspector — the typical workflow is "attach for the lifetime of the
//! process, then exit." For deterministic shutdown, kill the process.

use std::io::{BufReader, BufWriter};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use egui::{Context, FullOutput, RawInput};
use interprocess::local_socket::{RecvHalf, SendHalf, Stream, prelude::*};

use crate::INSPECTION_SOCKET_ENV_VAR;
use crate::transport::socket_name;
use crate::protocol::{
    Capabilities, Frame, FrameScreenshot, HarnessMessage, InspectorCommand, PROTOCOL_VERSION,
    PeerHello, PeerKind, read_message, write_message,
};

/// Errors that can occur attaching to an inspector.
#[derive(Debug)]
pub enum InspectionError {
    /// Failed to dial the inspector socket.
    Connect(std::io::Error),
    /// Failed to set up reader / writer threads.
    Pipe(String),
}

impl std::fmt::Display for InspectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(err) => write!(
                f,
                "failed to connect to egui_inspection socket (set {INSPECTION_SOCKET_ENV_VAR}): {err}"
            ),
            Self::Pipe(msg) => write!(f, "egui_inspection pipe setup failed: {msg}"),
        }
    }
}

impl std::error::Error for InspectionError {}

/// Bounded outbound queue depth. If the inspector falls behind we drop oldest frames
/// rather than block the UI thread.
const OUTBOUND_QUEUE_DEPTH: usize = 8;

/// Shared between [`InspectionPlugin::setup`] and the reader thread so the reader can wake
/// the UI loop after each received command. Written exactly once in `setup`.
type SharedCtx = Arc<OnceLock<Context>>;

/// `egui::Plugin` that streams the running app's state to an inspector.
pub struct InspectionPlugin {
    /// Incoming commands from the inspector.
    command_rx: Arc<Mutex<mpsc::Receiver<InspectorCommand>>>,
    /// Outbound messages → writer thread → socket. Bounded; oldest is dropped on overflow.
    outbound_tx: mpsc::SyncSender<HarnessMessage>,
    /// Filled in `Plugin::setup`; read by the reader thread to call `request_repaint` after
    /// every received command.
    shared_ctx: SharedCtx,
    /// Monotonic frame counter.
    step: u64,
    /// Frame data (accesskit + meta) captured in `output_hook`, held until the matching
    /// `Event::Screenshot` arrives in the next `input_hook`. Emitting only on pair-up keeps
    /// the inspector's screenshot and accesskit tree in lockstep — the alternative (emit
    /// accesskit now, screenshot later) shows widget boxes that don't match the rendered
    /// frame they overlay.
    pending_frame: Option<Frame>,
    /// `true` between dispatching `ViewportCommand::Screenshot` and observing the reply
    /// `Event::Screenshot`. While set, the plugin keeps requesting repaints so the
    /// integration eventually paints a visible frame and the screenshot fulfills (the eframe
    /// wgpu path skips capture when the viewport reports `visible=false`).
    awaiting_screenshot: bool,
    /// Set by [`InspectorCommand::Screenshot`]; consumed by the next `output_hook` which
    /// dispatches a `ViewportCommand::Screenshot` and stashes the frame.
    one_shot_screenshot: bool,
    /// When `true`, every `output_hook` requests a `ViewportCommand::Screenshot` and holds
    /// the frame until the screenshot returns. Toggled by
    /// [`InspectorCommand::SetContinuousScreenshots`].
    continuous_screenshots: bool,
    /// Background threads — held so they live as long as the plugin.
    _reader_thread: thread::JoinHandle<()>,
    _writer_thread: thread::JoinHandle<()>,
}

impl InspectionPlugin {
    /// If [`INSPECTION_SOCKET_ENV_VAR`] is set, return a plugin connected to it.
    /// Returns `Ok(None)` when the env var is unset.
    ///
    /// # Errors
    /// When the env var is set but the socket can't be dialed.
    pub fn from_env(label: Option<String>) -> Result<Option<Self>, InspectionError> {
        let Ok(name) = std::env::var(INSPECTION_SOCKET_ENV_VAR) else {
            return Ok(None);
        };
        Self::attach(&name, label).map(Some)
    }

    /// Dial the given local socket (see [`crate::transport::socket_name`]) and attach.
    ///
    /// # Errors
    /// When the socket can't be dialed or a thread can't be spawned.
    pub fn attach(socket: &str, label: Option<String>) -> Result<Self, InspectionError> {
        let name = socket_name(socket).map_err(InspectionError::Connect)?;
        let stream = Stream::connect(name).map_err(InspectionError::Connect)?;
        let (reader_stream, writer_stream) = stream.split();

        let shared_ctx: SharedCtx = Arc::new(OnceLock::new());

        let (command_tx, command_rx) = mpsc::channel::<InspectorCommand>();
        let reader_ctx = shared_ctx.clone();
        let reader_thread = thread::Builder::new()
            .name("egui_inspection_reader".into())
            .spawn(move || run_reader(BufReader::new(reader_stream), &command_tx, &reader_ctx))
            .map_err(|err| InspectionError::Pipe(format!("spawn reader thread: {err}")))?;

        let (outbound_tx, outbound_rx) = mpsc::sync_channel::<HarnessMessage>(OUTBOUND_QUEUE_DEPTH);
        let writer_thread = thread::Builder::new()
            .name("egui_inspection_writer".into())
            .spawn(move || run_writer(BufWriter::new(writer_stream), outbound_rx))
            .map_err(|err| InspectionError::Pipe(format!("spawn writer thread: {err}")))?;

        // Hello must be the first message on the wire. Send via the writer-thread queue
        // (rather than directly on the stream) so ordering against later frames is
        // preserved even under contention.
        let hello = HarnessMessage::Hello(PeerHello {
            protocol_version: PROTOCOL_VERSION,
            peer_kind: PeerKind::Live,
            capabilities: Capabilities::LIVE,
            // Live apps start accesskit-only; inspector flips on via
            // `SetContinuousScreenshots(true)` when it wants images.
            continuous_screenshots: false,
            label,
        });
        outbound_tx
            .send(hello)
            .map_err(|err| InspectionError::Pipe(format!("send Hello: {err}")))?;

        Ok(Self {
            command_rx: Arc::new(Mutex::new(command_rx)),
            outbound_tx,
            shared_ctx,
            step: 0,
            pending_frame: None,
            awaiting_screenshot: false,
            one_shot_screenshot: false,
            continuous_screenshots: false,
            _reader_thread: reader_thread,
            _writer_thread: writer_thread,
        })
    }

    /// Best-effort send. Drops oldest frame on overflow so the UI thread never blocks.
    fn send(&self, msg: HarnessMessage) {
        match self.outbound_tx.try_send(msg) {
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(msg)) => {
                // Queue saturated — try once more in case the writer just drained a slot.
                // If still full we drop the message. UI thread never blocks.
                let _ = self.outbound_tx.try_send(msg);
            }
            Err(mpsc::TrySendError::Disconnected(_)) => { /* writer is gone */ }
        }
    }
}

impl egui::Plugin for InspectionPlugin {
    fn debug_name(&self) -> &'static str {
        "egui_inspection"
    }

    fn setup(&mut self, ctx: &Context) {
        // We rely on the AccessKit tree to describe the UI structure to the inspector.
        ctx.enable_accesskit();
        // Hand the context to the reader thread so it can wake the UI loop when commands
        // arrive on an otherwise-idle app. `set` only succeeds the first time, which is
        // what we want — `setup` is documented to run once per plugin registration.
        let _ = self.shared_ctx.set(ctx.clone());
    }

    fn input_hook(&mut self, input: &mut RawInput) {
        // Capture any screenshot reply the integration produced in response to our previous
        // `ViewportCommand::Screenshot`. If we're holding a frame waiting for this
        // screenshot, attach the pixels and emit the pair now. Without a pending frame the
        // screenshot is stray (we never dispatched) and we drop it. We observe (don't
        // consume) — apps using the same event keep getting it.
        for ev in &input.events {
            if let egui::Event::Screenshot { image, .. } = ev {
                self.awaiting_screenshot = false;
                if let Some(mut frame) = self.pending_frame.take() {
                    let [w, h] = [image.size[0] as u32, image.size[1] as u32];
                    let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                    frame.screenshot = Some(FrameScreenshot {
                        width: w,
                        height: h,
                        rgba,
                    });
                    self.send(HarnessMessage::Frame(Box::new(frame)));
                }
                break;
            }
        }

        // Drain any commands the inspector sent since the previous frame.
        let mut got_command = false;
        let rx = self.command_rx.lock().expect("poisoned");
        while let Ok(cmd) = rx.try_recv() {
            got_command = true;
            match cmd {
                InspectorCommand::Handle { events } => {
                    input.events.extend(events);
                }
                InspectorCommand::Screenshot => {
                    self.one_shot_screenshot = true;
                }
                InspectorCommand::SetContinuousScreenshots(on) => {
                    self.continuous_screenshots = on;
                }
                InspectorCommand::Resize { width, height } => {
                    if let Some(ctx) = self.shared_ctx.get() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                            width as f32,
                            height as f32,
                        )));
                    }
                }
                // The live-app path doesn't own a deterministic run loop, so the
                // step/run/play/pause commands are no-ops here. The deterministic side
                // lives in `egui_kittest::InspectorPlugin`.
                InspectorCommand::Step
                | InspectorCommand::Run
                | InspectorCommand::Play
                | InspectorCommand::Pause => {}
            }
        }

        // Reactive-mode apps only paint on input. The reader thread's `request_repaint`
        // woke us for the current frame, but viewport-command replies (`Event::Screenshot`)
        // and synthetic `Handle` events both need at least one *more* frame to be observed
        // by the host app and round-trip back into a `Frame` we can emit. Without an extra
        // repaint scheduled now, the app goes idle until an unrelated wake-up (mouse move,
        // timer) and the inspector sees a multi-second stall.
        //
        // While a screenshot is outstanding (or continuous mode is on), keep requesting
        // repaints every frame — eframe's wgpu path skips screenshot capture when the
        // viewport reports `visible=false`, so a backgrounded window won't fulfill the
        // request until it next becomes visible. We can't force visibility from here without
        // disturbing focus, but pumping repaints keeps the app alive so the moment the OS
        // reports visibility (cursor enters, app brought forward, system unhide) the queued
        // action fires.
        if got_command || self.awaiting_screenshot || self.continuous_screenshots {
            if let Some(ctx) = self.shared_ctx.get() {
                ctx.request_repaint();
            }
        }
    }

    fn output_hook(&mut self, output: &mut FullOutput) {
        self.step = self.step.saturating_add(1);
        let want_screenshot = self.continuous_screenshots || self.one_shot_screenshot;
        self.one_shot_screenshot = false;

        // Pull the AccessKit tree update out of the PlatformOutput. We *clone* rather than
        // take so the host integration still receives it for the real accessibility stack.
        let tree = output.platform_output.accesskit_update.clone();

        let frame = Frame {
            step: self.step,
            pixels_per_point: output.pixels_per_point,
            screenshot: None,
            accesskit: tree,
            source: None,
        };

        if !want_screenshot {
            // No screenshot needed — emit immediately.
            self.send(HarnessMessage::Frame(Box::new(frame)));
            return;
        }

        // Want a screenshot. If the previous frame's request is still outstanding, drop
        // this output entirely (the screenshot reply would otherwise pair with a stale
        // accesskit tree). Slow inspector → matched-pair frames > throughput; the user
        // explicitly opted into this delay by enabling continuous screenshots.
        if self.awaiting_screenshot {
            return;
        }

        // Hold the frame; dispatch a screenshot request for what was just rendered. The
        // matching `Event::Screenshot` arrives in the next `input_hook`, where we attach
        // pixels and emit.
        self.pending_frame = Some(frame);
        if let Some(ctx) = self.shared_ctx.get() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.awaiting_screenshot = true;
        }
    }
}

/// Reader-thread entry point: forward every decoded [`InspectorCommand`] into the channel
/// until EOF or the receiver is dropped. After each enqueue, wake the UI thread so an
/// otherwise-idle app actually processes the command on its next frame.
fn run_reader(
    mut reader: BufReader<RecvHalf>,
    tx: &mpsc::Sender<InspectorCommand>,
    ctx: &SharedCtx,
) {
    loop {
        match read_message::<_, InspectorCommand>(&mut reader) {
            Ok(cmd) => {
                if tx.send(cmd).is_err() {
                    return;
                }
                if let Some(ctx) = ctx.get() {
                    ctx.request_repaint();
                }
            }
            Err(_) => return,
        }
    }
}

/// Writer-thread entry point: drain the outbound queue, framing each message to the socket.
fn run_writer(
    mut writer: BufWriter<SendHalf>,
    rx: mpsc::Receiver<HarnessMessage>,
) {
    while let Ok(msg) = rx.recv() {
        if write_message(&mut writer, &msg).is_err() {
            return;
        }
    }
}
