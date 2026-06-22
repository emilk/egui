//! Request/response wire protocol for inspecting a running egui app.
//!
//! Shared between an egui peer (a live `eframe` app running [`crate::InspectionPlugin`]) and
//! an external inspector (the `egui_mcp` server, or any other compatible tool).
//!
//! Every connection opens with a fixed binary handshake — [`PROTOCOL_MAGIC`] (4 bytes) plus
//! [`PROTOCOL_VERSION`] (4 big-endian bytes), written by the peer when a client connects — so
//! the client can reject a non-inspection or incompatible peer before decoding any
//! `MessagePack`. After the handshake the inspector sends [`Request`]s and the peer replies
//! with exactly one [`Response`] each.
//!
//! Messages are framed as a 4-byte big-endian length followed by a `MessagePack`-encoded body
//! (`rmp-serde`). Transport-neutral: the same framing works on TCP and any byte stream.
//!
//! Living in its own crate (rather than `egui_mcp`) lets eframe pull the protocol + plugin
//! in without depending on the MCP server, and lets external tools depend on the protocol
//! types directly.

use std::io::{self, Read, Write};

use egui::accesskit;

/// Wire-protocol version, sent in the connection handshake (see [`write_handshake`]).
///
/// Bump on any non-additive change to [`Request`] / [`Response`].
pub const PROTOCOL_VERSION: u32 = 1;

/// Magic bytes that open every connection, identifying the egui inspection protocol.
pub const PROTOCOL_MAGIC: [u8; 4] = *b"eins";

/// Sent inspector → peer. The peer replies with exactly one [`Response`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Request {
    /// Read the peer's label. Reply: [`Response::Info`].
    GetInfo,

    /// Read the current AccessKit tree. Reply: [`Response::Tree`].
    ///
    /// Servicing this triggers one repaint so the reply reflects the current frame.
    GetTree,

    /// Capture the current framebuffer as PNG. Reply: [`Response::Screenshot`].
    ///
    /// The peer issues an [`egui::ViewportCommand::Screenshot`] and replies once the
    /// resulting [`egui::Event::Screenshot`] arrives (one extra frame).
    ///
    /// `pixels_per_point` is the requested output resolution in pixels per logical point: the
    /// captured framebuffer (native resolution = the app's `pixels_per_point` px per point) is
    /// downscaled to this many px per point before encoding. `1.0` yields a logical-point-sized
    /// image so screenshot pixels align with the logical coordinates used everywhere else. Never
    /// upscaled beyond native, so values above the app's `pixels_per_point` have no effect.
    /// `None` captures at the framebuffer's native resolution, with no downscaling.
    GetScreenshot { pixels_per_point: Option<f32> },

    /// Inject raw egui input events and run a frame. Reply: [`Response::Done`], returned only
    /// *after* the events have been applied by a frame — so a subsequent [`Self::GetTree`]
    /// observes their effect. This is the single channel for all interaction
    /// (click / hover / drag / scroll / type / keypress): the inspector synthesizes the
    /// appropriate [`egui::Event`]s and sends them here.
    ApplyEvents { events: Vec<egui::Event> },

    /// Resize the peer's viewport to the given logical-point dimensions
    /// (via [`egui::ViewportCommand::InnerSize`]). Reply: [`Response::Done`]. This is the one
    /// action that isn't expressible as an [`egui::Event`].
    Resize { width: u32, height: u32 },
}

/// Sent peer → inspector, exactly one per [`Request`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Response {
    /// Reply to [`Request::GetInfo`].
    Info {
        /// Human-readable identifier (app name), if the peer set one.
        label: Option<String>,

        /// egui version string (e.g. `"0.31.0"`).
        egui_version: String,
    },

    /// Reply to [`Request::GetTree`].
    Tree {
        /// Monotonically increasing frame counter.
        step: u64,

        /// `physical_pixel = logical_point * pixels_per_point`. AccessKit bounds are in
        /// logical coords; a screenshot is in physical pixels — multiply to align them.
        pixels_per_point: f32,

        /// The current full AccessKit tree. egui rebuilds the complete node set every pass,
        /// so this is a full snapshot, not an incremental update. `None` if AccessKit hasn't
        /// produced a tree yet.
        accesskit: Option<accesskit::TreeUpdate>,
    },

    /// Reply to [`Request::GetScreenshot`].
    Screenshot(EncodedPng),

    /// Reply to [`Request::ApplyEvents`] / [`Request::Resize`] — the action was *executed*
    /// (not merely received): the events were processed by a frame, or the resize dispatched.
    Done,

    /// The peer failed to service the request (recoverable; the connection stays open).
    Error { message: String },
}

/// A PNG-encoded image with its pixel dimensions.
///
/// Construct one with [`Self::from_color_image`] / [`Self::from_rgba`] (requires the `png`
/// feature). The data type itself is always available so the inspector side can carry it
/// without pulling in the encoder.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncodedPng {
    /// `[width, height]` in physical pixels.
    pub size: [u32; 2],

    /// PNG-encoded image bytes. `serde_bytes` encodes this as a msgpack `bin` blob (one type
    /// tag + raw bytes) instead of the default per-byte `Vec<u8>` path.
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

/// Hard cap on a single framed message. Matches the sanity limit enforced by both ends.
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024 * 1024; // 256 MiB

fn invalid_data(err: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.to_string())
}

/// Write the connection handshake: [`PROTOCOL_MAGIC`] followed by [`PROTOCOL_VERSION`]
/// (big-endian). The peer sends this first thing on every accepted connection.
///
/// # Errors
/// On I/O failure.
pub fn write_handshake<W: Write>(mut writer: W) -> io::Result<()> {
    writer.write_all(&PROTOCOL_MAGIC)?;
    writer.write_all(&PROTOCOL_VERSION.to_be_bytes())?;
    writer.flush()
}

/// Validate the 8 handshake bytes and return the peer's protocol version.
///
/// The bytes are [`PROTOCOL_MAGIC`] (4) followed by a big-endian version (4). Pure (no I/O) so
/// sync ([`read_handshake`]) and async readers share the validation, mirroring
/// [`decode_frame_len`].
///
/// # Errors
/// If the magic bytes don't match (not an egui inspection peer).
pub fn decode_handshake(bytes: [u8; 8]) -> io::Result<u32> {
    let (magic, version) = bytes.split_at(4);
    if magic != PROTOCOL_MAGIC {
        return Err(invalid_data(
            "not an egui_inspection peer (bad handshake magic)",
        ));
    }
    Ok(u32::from_be_bytes(
        version.try_into().expect("split_at(4) leaves 4 bytes"),
    ))
}

/// Read and validate the connection handshake, returning the peer's protocol version.
///
/// # Errors
/// If the magic bytes don't match (not an egui inspection peer), or on I/O failure.
pub fn read_handshake<R: Read>(mut reader: R) -> io::Result<u32> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes)?;
    decode_handshake(bytes)
}

/// Encode a value into a length-prefixed `MessagePack` frame (4-byte big-endian length + body).
///
/// This is the wire format; sync ([`read_message`]/[`write_message`]) and async transports
/// share it via these helpers so the framing stays in lockstep.
///
/// # Errors
/// On encode failure or a body exceeding `u32::MAX`.
pub fn encode_frame<T: serde::Serialize>(value: &T) -> io::Result<Vec<u8>> {
    let body = rmp_serde::to_vec(value).map_err(invalid_data)?;
    let len = u32::try_from(body.len()).map_err(invalid_data)?;
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}

/// Decode a 4-byte frame header into a body length, rejecting anything over [`MAX_MESSAGE_BYTES`].
///
/// # Errors
/// When the declared length exceeds the cap.
pub fn decode_frame_len(header: [u8; 4]) -> io::Result<usize> {
    let len = u32::from_be_bytes(header) as usize;
    if len > MAX_MESSAGE_BYTES {
        return Err(invalid_data(format!("message too large: {len} bytes")));
    }
    Ok(len)
}

/// Decode a frame body (the bytes after the length prefix) into a value.
///
/// # Errors
/// On decode failure.
pub fn decode_frame_body<T: for<'de> serde::Deserialize<'de>>(body: &[u8]) -> io::Result<T> {
    rmp_serde::from_slice(body).map_err(invalid_data)
}

/// Encode a value as a bare `MessagePack` body, *without* the 4-byte length prefix of
/// [`encode_frame`].
///
/// For transports that delimit messages themselves — e.g. a gRPC unary call carrying the
/// bytes in a `bytes` field — the length prefix is redundant. Pair with [`decode_body`].
///
/// # Errors
/// On encode failure.
pub fn encode_body<T: serde::Serialize>(value: &T) -> io::Result<Vec<u8>> {
    rmp_serde::to_vec(value).map_err(invalid_data)
}

/// Decode a bare `MessagePack` body produced by [`encode_body`] into a value.
///
/// # Errors
/// On decode failure.
pub fn decode_body<T: for<'de> serde::Deserialize<'de>>(body: &[u8]) -> io::Result<T> {
    rmp_serde::from_slice(body).map_err(invalid_data)
}

/// Read one length-prefixed `MessagePack` message.
///
/// # Errors
/// I/O or decode failures.
pub fn read_message<R, T>(mut reader: R) -> io::Result<T>
where
    R: Read,
    T: for<'de> serde::Deserialize<'de>,
{
    let mut header = [0u8; 4];
    reader.read_exact(&mut header)?;
    let mut body = vec![0u8; decode_frame_len(header)?];
    reader.read_exact(&mut body)?;
    decode_frame_body(&body)
}

/// Write one length-prefixed `MessagePack` message.
///
/// # Errors
/// I/O or encode failures.
pub fn write_message<W, T>(mut writer: W, value: &T) -> io::Result<()>
where
    W: Write,
    T: serde::Serialize,
{
    writer.write_all(&encode_frame(value)?)?;
    writer.flush()
}
