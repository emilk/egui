//! Wire protocol shared between [`crate::Harness`] and the external `kittest_inspector`
//! binary (lives at <https://github.com/rerun-io/kittest_inspector>).
//!
//! The harness spawns the inspector as a child process with piped stdin/stdout. For each
//! step, the harness writes a [`HarnessMessage`] to the child's stdin and reads an
//! [`InspectorReply`] from its stdout. The inspector decides whether to reply immediately
//! (playing) or to wait for the user to click Play/Next (paused).
//!
//! Messages are framed as a 4-byte big-endian length followed by a bincode-encoded body.
//! Anything the inspector wants to log goes to stderr (which the harness inherits), keeping
//! stdout reserved for protocol traffic.
//!
//! Living inside `egui_kittest` (rather than the inspector crate) lets the inspector be
//! released independently — it just consumes whichever protocol version ships with the
//! egui release it was built against.

use std::io::{self, Read, Write};

use egui::accesskit;

/// One source file plus the test-source lines the inspector should highlight inside it.
///
/// The harness walks each captured backtrace (for the `.run()` call that produced the frame
/// and each event consumed by it), finds the topmost common test-source file across all of
/// them, reads that file, and emits its contents here. Highlights are line numbers within
/// that file: [`call_site_line`] for the runner call, [`event_lines`] for each event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceView {
    /// Absolute or crate-relative path as reported by the backtrace resolver.
    pub path: String,
    /// Entire file contents, lines separated by `\n`. `None` if the file couldn't be read.
    pub contents: Option<String>,
    /// Line number of the `.run()` / `.step()` call that produced this frame.
    pub call_site_line: Option<u32>,
    /// Line numbers of events consumed by this frame's step, in queue order.
    pub event_lines: Vec<u32>,
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

/// Sent harness → inspector after every step, and once when the harness disconnects.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HarnessMessage {
    /// A new frame is available.
    Frame(Box<Frame>),
    /// The harness is shutting down (e.g. `Drop`).
    Goodbye,
}

/// Sent inspector → harness in response to a [`HarnessMessage::Frame`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum InspectorReply {
    /// Resume the harness. `events` contains any user input captured in the inspector
    /// (via Control mode) that should be queued for the next step.
    Continue { events: Vec<egui::Event> },
}

const MAX_MESSAGE_BYTES: usize = 256 * 1024 * 1024; // 256 MiB sanity cap

/// Read a length-prefixed bincode message.
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
    let config = bincode::config::standard();
    let (value, _) = bincode::serde::decode_from_slice(&buf, config)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    Ok(value)
}

/// Write a length-prefixed bincode message.
///
/// # Errors
/// I/O or encode failures.
pub fn write_message<W, T>(mut writer: W, value: &T) -> io::Result<()>
where
    W: Write,
    T: serde::Serialize,
{
    let config = bincode::config::standard();
    let bytes = bincode::serde::encode_to_vec(value, config)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    let len = u32::try_from(bytes.len())
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    writer.write_all(&len.to_be_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}
