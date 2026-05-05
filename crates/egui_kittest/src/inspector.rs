//! [`InspectorPlugin`] — connect a [`crate::Harness`] to a `kittest_inspector` process for
//! live debugging.
//!
//! The plugin spawns the inspector as a child process and communicates over stdin/stdout
//! using the [`crate::inspector_api`] wire protocol. A background reader thread receives
//! [`InspectorCommand`]s from the inspector and pushes them into an mpsc channel, so the
//! plugin can check for commands non-blockingly during `Play` mode and block for them in
//! `Paused` mode.
//!
//! Auto-registered on harness creation when the [`INSPECTOR_ENV_VAR`] env var is truthy.

use std::collections::HashMap;
use std::io::{BufReader, BufWriter};
use std::panic::Location;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::sync::{LazyLock, OnceLock};
use std::thread;

use egui::accesskit;
use egui::mutex::Mutex;

use crate::inspector_api::{
    Frame, HarnessMessage, InspectorCommand, SourceView, read_message, write_message,
};
use crate::{Harness, Plugin, TestResult};

/// Environment variable: when set to a truthy value, every harness auto-launches an inspector.
pub const INSPECTOR_ENV_VAR: &str = "KITTEST_INSPECTOR";

/// Environment variable: explicit path to the `kittest_inspector` binary.
pub const INSPECTOR_PATH_ENV_VAR: &str = "KITTEST_INSPECTOR_PATH";

/// Errors that can occur attaching or talking to the inspector.
#[derive(Debug)]
pub enum InspectorError {
    /// Failed to launch the `kittest_inspector` binary.
    Launch(std::io::Error),
    /// Failed to set up the child's stdio pipes.
    Pipe(String),
}

impl std::fmt::Display for InspectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Launch(err) => write!(
                f,
                "failed to launch kittest_inspector (set {INSPECTOR_PATH_ENV_VAR} or put it on PATH): {err}"
            ),
            Self::Pipe(msg) => write!(f, "inspector pipe setup failed: {msg}"),
        }
    }
}

impl std::error::Error for InspectorError {}

/// Harness execution state as driven by the inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Block at `after_step` / `after_run` waiting for a command.
    Paused,
    /// Run until the next `after_step` fires, then pause.
    StepOnce,
    /// Run until the next `after_run` fires, then pause.
    RunOnce,
    /// Run freely; drain commands non-blockingly at each hook. Transitions to `Paused` on
    /// `Pause`, to `StepOnce` on `Step`, to `RunOnce` on `Run`.
    Playing,
}

/// Plugin that streams frames to an external `kittest_inspector` binary.
///
/// Typical use is to let [`Harness::from_builder`] auto-register this plugin based on the
/// [`INSPECTOR_ENV_VAR`] environment variable. For manual wiring, construct one with
/// [`Self::launch`] and pass to [`crate::HarnessBuilder::with_plugin`].
pub struct InspectorPlugin {
    conn: Connection,
    mode: Mode,
}

impl InspectorPlugin {
    /// Launch a `kittest_inspector` child process and attach this plugin to it.
    ///
    /// # Errors
    /// If the inspector binary cannot be launched or its stdio pipes fail to set up.
    pub fn launch(label: Option<String>) -> Result<Self, InspectorError> {
        Ok(Self {
            conn: Connection::launch(label)?,
            mode: Mode::Paused,
        })
    }
}

impl<S> Plugin<S> for InspectorPlugin {
    fn after_step(
        &mut self,
        harness: &mut Harness<'_, S>,
        accesskit_update: &accesskit::TreeUpdate,
    ) {
        self.handle_after_step(harness, accesskit_update);
    }

    /// When in `RunOnce`, `after_run` is the blocking point the user asked for. Nothing has
    /// re-rendered since the last `after_step`, so we only signal the state change via a
    /// `Blocked(true)` event (no duplicate frame) and then block.
    fn after_run(
        &mut self,
        harness: &mut Harness<'_, S>,
        _result: Result<u64, &crate::ExceededMaxStepsError>,
    ) {
        if self.mode == Mode::RunOnce {
            self.mode = Mode::Paused;
            self.conn.send_blocked(true);
            self.block_until_resume(harness);
        }
    }

    /// Test ended — send `Finished` (carrying the panic location in its `SourceView` when
    /// the panic's file matches the test entry), then block until the user dismisses with a
    /// Step / Run / Play command. The dismiss unblocks us; the harness finishes dropping on
    /// the way out.
    fn on_test_result(&mut self, harness: &mut Harness<'_, S>, result: TestResult<'_>) {
        if self.conn.broken {
            return;
        }

        let (ok, message, panic_loc) = match result {
            TestResult::Pass => (true, None, None),
            TestResult::Fail { message, location } => (
                false,
                message.map(str::to_owned),
                location.map(|loc| (loc.file.clone(), loc.line)),
            ),
        };

        let source = build_source_view(
            harness.entry_location(),
            harness.consumed_event_locations(),
            panic_loc.as_ref(),
        );
        self.conn.write(&HarnessMessage::Finished {
            ok,
            message,
            source,
        });
        // Park here until the user dismisses with Step/Run/Play. `block_until_resume` exits
        // on any of those (they all transition out of `Paused`); `Pause` is a no-op; `Handle`
        // still works so the user can poke at the final UI on failure. The mode mutation it
        // leaves behind is harmless — the plugin is about to drop.
        self.mode = Mode::Paused;
        self.block_until_resume(harness);
    }
}

impl InspectorPlugin {
    /// Send a frame for this step and apply the current mode's blocking / draining policy.
    /// `after_run` is handled separately — it only transitions `RunOnce → Paused`.
    fn handle_after_step<S>(&mut self, harness: &mut Harness<'_, S>, tree: &accesskit::TreeUpdate) {
        if self.conn.broken {
            return;
        }

        // Blocking points at after_step are: Paused (always) and StepOnce (one-shot).
        // RunOnce keeps running past every after_step until after_run completes; Playing
        // runs freely.
        let will_block_here = matches!(self.mode, Mode::Paused | Mode::StepOnce);

        self.send_frame(harness, Some(tree.clone()));
        self.conn.send_blocked(will_block_here);

        if self.mode == Mode::StepOnce {
            self.mode = Mode::Paused;
        }

        match self.mode {
            Mode::Paused => self.block_until_resume(harness),
            Mode::StepOnce | Mode::RunOnce => {
                // Non-blocking: keep running.
            }
            Mode::Playing => {
                self.drain_playing(harness);
                if self.mode == Mode::Paused {
                    // A `Pause` came in while playing — block now.
                    self.conn.send_blocked(true);
                    self.block_until_resume(harness);
                }
            }
        }
    }

    /// Block on the command channel until a command transitions us out of [`Mode::Paused`].
    /// `Handle` commands execute a `step_no_side_effects` and send a fresh frame, then we
    /// keep blocking.
    fn block_until_resume<S>(&mut self, harness: &mut Harness<'_, S>) {
        while self.mode == Mode::Paused && !self.conn.broken {
            match self.conn.command_rx.recv() {
                Ok(InspectorCommand::Step) => self.mode = Mode::StepOnce,
                Ok(InspectorCommand::Run) => self.mode = Mode::RunOnce,
                Ok(InspectorCommand::Play) => self.mode = Mode::Playing,
                Ok(InspectorCommand::Pause) => { /* already paused */ }
                Ok(InspectorCommand::Handle { events }) => {
                    self.apply_handle(harness, events);
                }
                Err(_) => {
                    // Reader thread is gone — no more commands will arrive. Stop blocking
                    // so the test can continue to drop cleanly.
                    self.conn.broken = true;
                    return;
                }
            }
        }
    }

    /// Drain any commands that are already queued without blocking. Called at every hook
    /// while in [`Mode::Playing`].
    fn drain_playing<S>(&mut self, harness: &mut Harness<'_, S>) {
        loop {
            match self.conn.command_rx.try_recv() {
                Ok(InspectorCommand::Pause) => self.mode = Mode::Paused,
                Ok(InspectorCommand::Step) => self.mode = Mode::StepOnce,
                Ok(InspectorCommand::Run) => self.mode = Mode::RunOnce,
                Ok(InspectorCommand::Play) => { /* already playing */ }
                Ok(InspectorCommand::Handle { events }) => {
                    self.apply_handle(harness, events);
                }
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.conn.broken = true;
                    return;
                }
            }
        }
    }

    /// Queue inspector-driven events and advance one frame without firing plugin hooks, then
    /// send a fresh frame so the inspector sees the effect. `Handle` never changes the
    /// harness's Paused/Play/Run mode, so we don't emit a `Blocked` event here.
    fn apply_handle<S>(&mut self, harness: &mut Harness<'_, S>, events: Vec<egui::Event>) {
        for event in events {
            harness.input_mut().events.push(event);
        }
        // `step_no_side_effects` returns the tree directly — we can't receive it via
        // `after_step` because nested plugin dispatches are suppressed.
        let tree = harness.step_no_side_effects();
        self.send_frame(harness, Some(tree));
    }

    /// Render the current harness state and push it to the inspector.
    fn send_frame<S>(&mut self, harness: &mut Harness<'_, S>, tree: Option<accesskit::TreeUpdate>) {
        if self.conn.broken {
            return;
        }
        let image = match harness.render() {
            Ok(img) => img,
            Err(err) => {
                #[expect(clippy::print_stderr)]
                {
                    eprintln!("egui_kittest inspector: render failed: {err}");
                }
                self.conn.broken = true;
                return;
            }
        };
        let ppp = harness.ctx.pixels_per_point();
        let source = build_source_view(
            harness.entry_location(),
            harness.consumed_event_locations(),
            None,
        );
        self.conn.send_frame(&image, ppp, tree, source);
    }
}

/// The inspector's child-process connection + step counter. Private — [`InspectorPlugin`] is
/// the public wrapper.
struct Connection {
    writer: BufWriter<ChildStdin>,
    command_rx: mpsc::Receiver<InspectorCommand>,
    _reader_thread: thread::JoinHandle<()>,
    _child: Child,
    step: u64,
    label: Option<String>,
    broken: bool,
}

impl Connection {
    fn launch(label: Option<String>) -> Result<Self, InspectorError> {
        let bin = std::env::var(INSPECTOR_PATH_ENV_VAR)
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("kittest_inspector"));

        // Important: do NOT inherit stderr. The cargo-test / nextest stderr capture pipe can
        // close between tests while the inspector is still alive; a later `eprintln!` in the
        // inspector would then panic ("failed printing to stderr: Broken pipe") and take the
        // window down. The inspector keeps its own log file for diagnostics.
        let mut child = Command::new(&bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(InspectorError::Launch)?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| InspectorError::Pipe("missing child stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| InspectorError::Pipe("missing child stdout".into()))?;

        let (command_tx, command_rx) = mpsc::channel::<InspectorCommand>();
        let reader_thread = thread::Builder::new()
            .name("kittest_inspector_reader".into())
            .spawn(move || run_reader(BufReader::new(stdout), &command_tx))
            .map_err(|err| InspectorError::Pipe(format!("spawn reader thread: {err}")))?;

        Ok(Self {
            writer: BufWriter::new(stdin),
            command_rx,
            _reader_thread: reader_thread,
            _child: child,
            step: 0,
            label,
            broken: false,
        })
    }

    fn send_frame(
        &mut self,
        image: &image::RgbaImage,
        pixels_per_point: f32,
        accesskit: Option<accesskit::TreeUpdate>,
        source: Option<SourceView>,
    ) {
        if self.broken {
            return;
        }
        self.step = self.step.saturating_add(1);
        let frame = Frame {
            step: self.step,
            width: image.width(),
            height: image.height(),
            pixels_per_point,
            rgba: image.as_raw().clone(),
            accesskit,
            label: self.label.clone(),
            source,
        };
        self.write(&HarnessMessage::Frame(Box::new(frame)));
    }

    /// Tell the inspector the harness's blocking state changed.
    fn send_blocked(&mut self, blocking: bool) {
        self.write(&HarnessMessage::Blocked(blocking));
    }

    fn write(&mut self, msg: &HarnessMessage) {
        if self.broken {
            return;
        }
        if let Err(err) = write_message(&mut self.writer, msg) {
            #[expect(clippy::print_stderr)]
            {
                eprintln!("egui_kittest inspector: send failed: {err}");
            }
            self.broken = true;
        }
    }
}

/// Reader-thread entry point: forward every decoded [`InspectorCommand`] into the mpsc
/// channel until EOF or the receiver is dropped.
fn run_reader(mut reader: BufReader<ChildStdout>, tx: &mpsc::Sender<InspectorCommand>) {
    loop {
        match read_message::<_, InspectorCommand>(&mut reader) {
            Ok(cmd) => {
                if tx.send(cmd).is_err() {
                    return;
                }
            }
            Err(_) => return,
        }
    }
}

/// Build the [`SourceView`] payload for a frame: pick the `.run()`/`.step()` caller's file
/// as the anchor, and record each event's line within that same file. `panic_loc` is set
/// only on the final frame after a failed test — and only included in the output if the
/// panic's file matches the anchor (otherwise there's no highlight to attach).
///
/// `#[track_caller]` chains through the entire event-queuing API, so each `Location` points
/// directly at the user's test source — no backtrace walking needed.
fn build_source_view(
    call_site: Option<&'static Location<'static>>,
    event_sites: &[&'static Location<'static>],
    panic_loc: Option<&(String, u32)>,
) -> Option<SourceView> {
    let call = call_site?;
    let path = call.file().to_owned();
    let event_lines = event_sites
        .iter()
        .filter(|loc| loc.file() == path)
        .map(|loc| loc.line())
        .collect();
    let panic_line = panic_loc
        .filter(|(file, _)| file == &path)
        .map(|(_, line)| *line);
    Some(SourceView {
        path: path.clone(),
        contents: read_source_file(&path),
        call_site_line: Some(call.line()),
        event_lines,
        panic_line,
    })
}

/// Read the full contents of a source file, cached per path (including negative results).
///
/// `path` comes from `std::panic::Location::file()`, which the compiler reports relative to
/// the *workspace* root. Cargo runs tests with CWD set to the *crate* root, so for a
/// workspace crate at `<workspace>/crates/foo/` the compiler-reported path is
/// `crates/foo/src/…` but CWD is `<workspace>/crates/foo/`. We try as-is first (handles
/// absolute paths and single-crate layouts), then walk up from CWD looking for an ancestor
/// where `ancestor.join(path)` resolves.
fn read_source_file(path: &str) -> Option<String> {
    static CACHE: LazyLock<Mutex<HashMap<String, Option<String>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let mut cache = CACHE.lock();
    cache
        .entry(path.to_owned())
        .or_insert_with(|| resolve_and_read(path))
        .clone()
}

fn resolve_and_read(path: &str) -> Option<String> {
    if let Ok(contents) = std::fs::read_to_string(path) {
        return Some(contents);
    }
    if std::path::Path::new(path).is_absolute() {
        return None;
    }
    let mut cursor = std::env::current_dir().ok()?;
    // `pop()` returns false once we've hit the root, which terminates the search.
    while cursor.pop() {
        if let Ok(contents) = std::fs::read_to_string(cursor.join(path)) {
            return Some(contents);
        }
    }
    None
}

/// Read [`INSPECTOR_ENV_VAR`] once and cache. Exposed to [`crate::Harness::from_builder`]
/// so it can auto-register an [`InspectorPlugin`].
pub(crate) fn env_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| match std::env::var(INSPECTOR_ENV_VAR) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    })
}
