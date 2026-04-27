//! Wire protocol shared between [`crate::Harness`] and the external `kittest_inspector`
//! binary (lives at <https://github.com/rerun-io/kittest_inspector>).
//!
//! The harness spawns the inspector as a child process with piped stdin/stdout. The two
//! sides communicate asynchronously: the harness writes [`HarnessMessage`]s (frames plus
//! blocking-state updates) into the child's stdin, and the inspector writes
//! [`InspectorCommand`]s to the harness's stdout whenever the user drives the UI. Shutdown
//! is detected on either side via EOF — no explicit goodbye message.
//!
//! Messages are framed as a 4-byte big-endian length followed by a MessagePack-encoded body
//! (`rmp-serde`). Anything the inspector wants to log goes to a file on disk (see the
//! inspector crate's `log_diag`), keeping stdout reserved for protocol traffic.
//!
//! Living inside `egui_kittest` (rather than the inspector crate) lets the inspector be
//! released independently — it just consumes whichever protocol version ships with the
//! egui release it was built against.

use std::io::{self, Read, Write};

use egui::accesskit;

/// One source file plus the test-source lines the inspector should highlight inside it.
///
/// The harness captures `#[track_caller]` locations for the `.run()`/`.step()` call that
/// produced the frame and for each event consumed by it. The inspector highlights
/// [`Self::call_site_line`] for the runner call and [`Self::event_lines`] for each event.
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

/// A single rendered frame plus the accesskit tree update produced by the harness step.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Frame {
    /// Monotonically increasing step counter.
    pub step: u64,
    /// Image width in physical pixels.
    pub width: u32,
    /// Image height in physical pixels.
    pub height: u32,
    /// `physical_pixel = logical_point * pixels_per_point`. AccessKit bounds are in logical
    /// coords, the rendered image is in physical pixels — multiply by this to align them.
    pub pixels_per_point: f32,
    /// Tightly packed RGBA8 pixels (length = `width * height * 4`).
    pub rgba: Vec<u8>,
    /// Latest accesskit tree update, if any.
    pub accesskit: Option<accesskit::TreeUpdate>,
    /// Optional human-readable label (e.g. test name).
    pub label: Option<String>,
    /// The test source file associated with this frame + the lines to highlight inside it.
    pub source: Option<SourceView>,
}

/// Sent harness → inspector. Frames carry rendered images; `Blocked` signals when the
/// harness's blocking state changes without a visual update (e.g. at `after_run`, where
/// nothing has re-rendered since the last `after_step`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HarnessMessage {
    /// A new frame (image + tree + source) is available.
    Frame(Box<Frame>),
    /// The harness is now either blocked (`true`) waiting for an [`InspectorCommand`], or
    /// running freely (`false`).
    Blocked(bool),
    /// The test has ended. Implies [`Self::Blocked`]`(true)`: the harness blocks after
    /// sending this, and any subsequent `Step` / `Run` / `Play` command dismisses the result
    /// and lets the harness drop.
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

/// Sent inspector → harness at any time to drive test execution.
///
/// The harness blocks at `after_step` / `after_run` hooks (and at those hooks only). Which
/// command it waits for, and whether it returns to blocking after executing one, depends on
/// the command that last arrived — see each variant's docs.
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
    /// Queue these events on the harness and run a single `step_no_side_effects` call. Does
    /// not change the harness's Pause / Play / Run state — after the step the harness resumes
    /// whatever mode it was in.
    Handle { events: Vec<egui::Event> },
}

const MAX_MESSAGE_BYTES: usize = 256 * 1024 * 1024; // 256 MiB sanity cap

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
