//! Connect a [`crate::Harness`] to a `kittest_inspector` process for live debugging.
//!
//! The harness spawns the inspector as a child process with piped stdin/stdout. After every
//! step the harness writes a frame + accesskit tree update to the child's stdin and reads a
//! reply from its stdout, blocking until the user resumes (when paused).

use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Write as _};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::LazyLock;

use egui::accesskit;
use egui::mutex::Mutex;
use kittest_inspector::{
    Frame, HarnessMessage, InspectorReply, SourceView, read_message, write_message,
};

use crate::node::EventSite;

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

/// An attached inspector. Owned by the [`crate::Harness`].
pub(crate) struct Inspector {
    writer: BufWriter<ChildStdin>,
    reader: BufReader<ChildStdout>,
    /// Keep the child alive until the harness drops.
    _child: Child,
    step: u64,
    label: Option<String>,
    /// True once the connection has failed; we stop trying to send.
    broken: bool,
}

impl Inspector {
    /// Launch a new `kittest_inspector` child process.
    ///
    /// Search order for the binary:
    /// 1. The path in `KITTEST_INSPECTOR_PATH` if set.
    /// 2. `kittest_inspector` from `PATH`.
    pub fn launch(label: Option<String>) -> Result<Self, InspectorError> {
        let bin = std::env::var(INSPECTOR_PATH_ENV_VAR)
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("kittest_inspector"));

        // Important: do NOT inherit stderr. The cargo-test / nextest stderr capture pipe
        // can close between tests while the inspector is still alive; a later `eprintln!`
        // in the inspector would then panic ("failed printing to stderr: Broken pipe") and
        // take the window down. The inspector keeps its own log file at
        // `{temp}/kittest_inspector.log` for diagnostics.
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

        Ok(Self {
            writer: BufWriter::new(stdin),
            reader: BufReader::new(stdout),
            _child: child,
            step: 0,
            label,
            broken: false,
        })
    }

    /// Send the current frame + accesskit tree and block until the inspector replies.
    /// Returns any user events captured by the inspector (empty on failure / no control input).
    pub fn send_step(
        &mut self,
        image: &image::RgbaImage,
        pixels_per_point: f32,
        accesskit: Option<accesskit::TreeUpdate>,
        call_site: &EventSite,
        event_sites: &[EventSite],
    ) -> Vec<egui::Event> {
        if self.broken {
            return Vec::new();
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
            source: build_source_view(call_site, event_sites),
        };
        if let Err(err) =
            write_message(&mut self.writer, &HarnessMessage::Frame(Box::new(frame)))
        {
            #[expect(clippy::print_stderr)]
            {
                eprintln!("egui_kittest inspector: send failed: {err}");
            }
            self.broken = true;
            return Vec::new();
        }
        match read_message::<_, InspectorReply>(&mut self.reader) {
            Ok(InspectorReply::Continue { events }) => events,
            Err(err) => {
                #[expect(clippy::print_stderr)]
                {
                    eprintln!("egui_kittest inspector: read failed: {err}");
                }
                self.broken = true;
                Vec::new()
            }
        }
    }

    pub fn say_goodbye(&mut self) {
        if self.broken {
            return;
        }
        let _ = write_message(&mut self.writer, &HarnessMessage::Goodbye);
        let _ = self.writer.flush();
    }
}

impl Drop for Inspector {
    fn drop(&mut self) {
        self.say_goodbye();
    }
}

/// Build the [`SourceView`] payload for a frame: find the topmost test-source file common to
/// the runner call (`call_site`) and all consumed events, then read that file once and record
/// each event's line inside it.
fn build_source_view(call_site: &EventSite, event_sites: &[EventSite]) -> Option<SourceView> {
    let call_frames = call_site.as_deref().map(user_frames);
    let event_frames: Vec<Vec<UserFrame>> = event_sites
        .iter()
        .map(|s| s.as_deref().map(user_frames).unwrap_or_default())
        .collect();

    // Build the list of candidate-source frame vecs. The call site is required — without it
    // there's nothing anchoring the "frame producer".
    let call_frames = call_frames?;
    if call_frames.is_empty() {
        return None;
    }

    // Pick the topmost file (latest in outer-most order) that appears in every non-empty
    // event stack as well as the call-site stack. Ignore completely-empty event stacks
    // (e.g. events driven by the inspector itself).
    let non_empty_events: Vec<&Vec<UserFrame>> =
        event_frames.iter().filter(|v| !v.is_empty()).collect();
    let path = pick_common_file(&call_frames, &non_empty_events)?;

    let call_site_line = innermost_line_for(&call_frames, &path);
    let event_lines: Vec<u32> = event_frames
        .iter()
        .filter_map(|frames| innermost_line_for(frames, &path))
        .collect();

    Some(SourceView {
        path: path.clone(),
        contents: read_source_file(&path),
        call_site_line,
        event_lines,
    })
}

/// One resolved user-code frame (file + line).
#[derive(Debug, Clone)]
struct UserFrame {
    file: String,
    line: u32,
}

/// Resolve a backtrace and return its user-code frames, innermost first.
fn user_frames(bt: &backtrace::Backtrace) -> Vec<UserFrame> {
    let mut bt = bt.clone();
    bt.resolve();
    let mut out = Vec::new();
    for frame in bt.frames() {
        for symbol in frame.symbols() {
            let Some(path) = symbol.filename() else { continue };
            let Some(line) = symbol.lineno() else { continue };
            let path = path.to_string_lossy().into_owned();
            if !is_user_code(&path) {
                continue;
            }
            out.push(UserFrame { file: path, line });
            break;
        }
    }
    out
}

/// A frame's file is "user code" if it isn't inside the Rust toolchain, a cargo registry
/// dependency, or the `egui_kittest` / `kittest_inspector` crates themselves. This keeps the
/// common-file search honest: we skip past the harness's own plumbing.
fn is_user_code(path: &str) -> bool {
    const EXCLUDE: &[&str] = &[
        "/rustc/",
        "/toolchains/",
        "/.cargo/registry/",
        "/.cargo/git/",
        "egui_kittest/src/",
        "kittest_inspector/src/",
    ];
    !EXCLUDE.iter().any(|needle| path.contains(needle))
}

/// Among files common to `call_frames` and every stack in `event_frames`, pick the one that
/// is **outermost** (furthest from the event origin) in the call-site stack. Intuition: the
/// outermost common file is the test function itself; inner ones are helpers.
fn pick_common_file(call_frames: &[UserFrame], event_frames: &[&Vec<UserFrame>]) -> Option<String> {
    // Walk the call-site stack outermost-first.
    for frame in call_frames.iter().rev() {
        if event_frames
            .iter()
            .all(|frames| frames.iter().any(|f| f.file == frame.file))
        {
            return Some(frame.file.clone());
        }
    }
    None
}

/// Return the line number of the innermost frame in `frames` whose file matches `path`.
fn innermost_line_for(frames: &[UserFrame], path: &str) -> Option<u32> {
    frames.iter().find(|f| f.file == path).map(|f| f.line)
}

/// Read the full contents of a source file, cached per path (including negative results).
fn read_source_file(path: &str) -> Option<String> {
    static CACHE: LazyLock<Mutex<HashMap<String, Option<String>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let mut cache = CACHE.lock();
    cache
        .entry(path.to_owned())
        .or_insert_with(|| std::fs::read_to_string(path).ok())
        .clone()
}

/// Read [`INSPECTOR_ENV_VAR`] once and cache.
pub(crate) fn env_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| match std::env::var(INSPECTOR_ENV_VAR) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    })
}
