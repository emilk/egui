//! Wire protocol for `kittest_inspector`.
//!
//! The harness launches `kittest_inspector` as a child process with piped stdin/stdout.
//! For each step, the harness writes a [`HarnessMessage`] to the child's stdin and reads an
//! [`InspectorReply`] from its stdout. The inspector decides whether to reply immediately
//! (playing) or to wait for the user to click Play/Next (paused).
//!
//! Messages are framed as a 4-byte big-endian length followed by a bincode-encoded body.
//! Anything the inspector wants to log goes to stderr (which the harness inherits), keeping
//! stdout reserved for protocol traffic.

use std::io::{self, Read, Write};

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
}

/// Sent harness → inspector after every step, and once when the harness disconnects.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HarnessMessage {
    /// A new frame is available.
    Frame(Frame),
    /// The harness is shutting down (e.g. `Drop`).
    Goodbye,
}

/// Sent inspector → harness in response to a [`HarnessMessage::Frame`].
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum InspectorReply {
    /// Resume the harness (it will continue running steps and may send another frame soon).
    Continue,
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
