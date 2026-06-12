//! Request/response wire protocol for inspecting a running egui app.
//!
//! Shared between an egui peer (a live `eframe` app running [`crate::InspectionPlugin`]) and
//! an external inspector (the `egui_mcp` server, or any other tool — e.g. `re_mcp` tunnelling
//! it over gRPC).
//!
//! The model is strictly **request/response**: the inspector sends a [`Request`], the peer
//! replies with exactly one [`Response`]. There is no streaming, no broadcast, and no
//! peer-initiated message. This maps cleanly onto a TCP socket (one framed request, one
//! framed response) *and* onto a unary RPC, which is what lets `re_mcp` carry it over gRPC
//! without reframing.
//!
//! Messages are framed as a 4-byte big-endian length followed by a `MessagePack`-encoded body
//! (`rmp-serde`). Transport-neutral: the same framing works on TCP and any byte stream.
//!
//! Living in its own crate (rather than `egui_mcp`) lets eframe pull the protocol + plugin
//! in without depending on the MCP server, and lets external tools depend on the protocol
//! types directly.

use std::io::{self, Read, Write};

use egui::accesskit;

/// Wire-protocol version reported by [`Response::Info`]. Bump on any non-additive change to
/// [`Request`] / [`Response`]. An inspector should refuse a peer whose major version it
/// doesn't understand.
pub const PROTOCOL_VERSION: u32 = 1;

/// Sent inspector → peer. The peer replies with exactly one [`Response`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Request {
    /// Handshake / liveness probe. Reply: [`Response::Info`]. An inspector issues this first
    /// to confirm it's talking to a compatible egui inspection peer.
    Info,

    /// Read the current AccessKit tree. Reply: [`Response::Tree`].
    ///
    /// Servicing this triggers one repaint so the reply reflects the current frame.
    GetTree,

    /// Capture the current framebuffer as PNG. Reply: [`Response::Screenshot`].
    ///
    /// The peer issues an [`egui::ViewportCommand::Screenshot`] and replies once the
    /// resulting [`egui::Event::Screenshot`] arrives (one extra frame).
    Screenshot,

    /// Inject raw egui input events and run a frame. Reply: [`Response::Ack`], returned only
    /// *after* the events have been processed by a frame — so a subsequent [`Self::GetTree`]
    /// observes their effect. This is the single channel for all interaction
    /// (click / hover / drag / scroll / type / keypress): the inspector synthesizes the
    /// appropriate [`egui::Event`]s and sends them here.
    HandleEvents { events: Vec<egui::Event> },

    /// Resize the peer's viewport to the given logical-point dimensions
    /// (via [`egui::ViewportCommand::InnerSize`]). Reply: [`Response::Ack`]. This is the one
    /// action that isn't expressible as an [`egui::Event`].
    Resize { width: u32, height: u32 },
}

/// Sent peer → inspector, exactly one per [`Request`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Response {
    /// Reply to [`Request::Info`].
    Info {
        /// [`PROTOCOL_VERSION`] of the peer.
        protocol_version: u32,

        /// Human-readable identifier (app name), if the peer set one.
        label: Option<String>,
    },

    /// Reply to [`Request::GetTree`] (and produced as the post-frame state by tooling that
    /// pairs a [`Request::HandleEvents`] with a follow-up [`Request::GetTree`]).
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

    /// Reply to [`Request::Screenshot`].
    Screenshot {
        /// Image width in physical pixels.
        width: u32,

        /// Image height in physical pixels.
        height: u32,

        /// PNG-encoded image bytes. `serde_bytes` encodes this as a msgpack `bin` blob
        /// (one type tag + raw bytes) instead of the default per-byte `Vec<u8>` path.
        #[serde(with = "serde_bytes")]
        png: Vec<u8>,
    },

    /// Reply to [`Request::HandleEvents`] / [`Request::Resize`] — the action was applied.
    Ack,

    /// The peer failed to service the request (recoverable; the connection stays open).
    Error { message: String },
}

/// Hard cap on a single framed message. Matches the sanity limit enforced by both ends.
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024 * 1024; // 256 MiB

fn invalid_data(err: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.to_string())
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
