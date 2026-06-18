//! Bridge between the MCP server and a running egui app, speaking the request/response
//! [`egui_inspection`] protocol.
//!
//! The app is the *listener* (it opened an inspection port); the bridge is a client that
//! connects to it. Each MCP tool call turns into one or more [`Request`]s, each answered by
//! exactly one [`Response`] over the same connection (serialized by a mutex).
//!
//! Two ways to build a bridge:
//! - [`Bridge::connect`] dials a TCP `host:port` (with connect-retry) and reads the protocol
//!   handshake.
//! - [`Bridge::from_transport`] wraps an arbitrary async byte transport, so a host that owns
//!   its own channel can drive the same tools.

use std::time::Duration;

use anyhow::{Context as _, anyhow, bail};
use egui_inspection::protocol::{
    EncodedPng, Request, Response, decode_frame_body, decode_frame_len, decode_handshake,
    encode_frame,
};
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

/// How long to keep retrying the initial TCP connect before giving up. Covers the case where
/// the user launches the app and attaches in quick succession.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Identity of the connected peer, captured at connect time.
#[derive(Debug, Clone, Serialize)]
pub struct PeerInfo {
    /// `host:port` for a TCP connection, or a description for a custom transport.
    pub transport: String,
    pub protocol_version: u32,
    pub label: Option<String>,
}

/// The reader + writer halves of the connection, behind one lock so requests don't interleave.
struct Conn {
    reader: Box<dyn AsyncRead + Unpin + Send>,
    writer: Box<dyn AsyncWrite + Unpin + Send>,
}

/// A live connection to an egui inspection peer.
pub struct Bridge {
    conn: Mutex<Conn>,
    pub peer_info: PeerInfo,
}

impl Bridge {
    /// Connect to a TCP `host:port`, retrying until `timeout` (default 10s) elapses, then read
    /// the protocol handshake and the peer's label.
    ///
    /// # Errors
    /// If the connection can't be established before the timeout, or the handshake fails.
    pub async fn connect(host: &str, port: u16, timeout: Option<Duration>) -> anyhow::Result<Self> {
        let addr = format!("{host}:{port}");
        let timeout = timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT);
        let deadline = tokio::time::Instant::now() + timeout;

        let stream = loop {
            match TcpStream::connect(&addr).await {
                Ok(s) => break s,
                Err(e) => {
                    if tokio::time::Instant::now() >= deadline {
                        return Err(anyhow!(e)).with_context(|| {
                            format!(
                                "connect to {addr} (is the app running with EGUI_INSPECTION set?)"
                            )
                        });
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        };
        let _ = stream.set_nodelay(true);
        let (reader, writer) = stream.into_split();
        let mut bridge = Self::from_transport(
            reader,
            writer,
            PeerInfo {
                transport: addr,
                protocol_version: 0,
                label: None,
            },
        );

        bridge.peer_info.protocol_version = bridge
            .read_handshake()
            .await
            .context("inspection handshake")?;
        bridge.peer_info.label = bridge.fetch_label().await.context("read peer label")?;
        Ok(bridge)
    }

    /// Read and validate the connection handshake (magic + version), which the peer writes
    /// first thing on every connection.
    ///
    /// # Errors
    /// If the magic bytes don't match (not an egui inspection peer), or on I/O failure.
    pub async fn read_handshake(&self) -> anyhow::Result<u32> {
        let mut conn = self.conn.lock().await;
        let mut bytes = [0u8; 8];
        conn.reader
            .read_exact(&mut bytes)
            .await
            .context("read handshake")?;
        Ok(decode_handshake(bytes)?)
    }

    /// Wrap an arbitrary async byte transport (for hosts that tunnel the protocol).
    pub fn from_transport<R, W>(reader: R, writer: W, peer_info: PeerInfo) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        Self {
            conn: Mutex::new(Conn {
                reader: Box::new(reader),
                writer: Box::new(writer),
            }),
            peer_info,
        }
    }

    /// Send one request and await its single response.
    ///
    /// # Errors
    /// On I/O failure writing the request or reading the response.
    pub async fn request(&self, req: Request) -> anyhow::Result<Response> {
        let mut conn = self.conn.lock().await;
        write_framed(&mut conn.writer, &req)
            .await
            .context("send request")?;
        read_framed(&mut conn.reader).await.context("read response")
    }

    async fn fetch_label(&self) -> anyhow::Result<Option<String>> {
        match self.request(Request::GetInfo).await? {
            Response::Info { label, .. } => Ok(label),
            Response::Error { message } => bail!("{message}"),
            _ => bail!("unexpected response to GetInfo"),
        }
    }

    /// Fetch the current AccessKit tree, built into a queryable [`accesskit_consumer::Tree`].
    ///
    /// # Errors
    /// On I/O failure or an unexpected response.
    pub async fn fetch_tree(&self) -> anyhow::Result<TreeSnapshot> {
        match self.request(Request::GetTree).await? {
            Response::Tree {
                step,
                pixels_per_point,
                accesskit,
            } => Ok(TreeSnapshot {
                step,
                pixels_per_point,
                tree: accesskit.map(|update| accesskit_consumer::Tree::new(update, false)),
            }),
            Response::Error { message } => bail!("{message}"),
            _ => bail!("unexpected response to GetTree"),
        }
    }

    /// Send a request that expects a bare [`Response::Done`]. `what` names the request for
    /// error messages.
    async fn request_done(&self, req: Request, what: &str) -> anyhow::Result<()> {
        match self.request(req).await? {
            Response::Done => Ok(()),
            Response::Error { message } => bail!("{message}"),
            _ => bail!("unexpected response to {what}"),
        }
    }

    /// Inject input events; resolves after the events have been processed by a frame.
    ///
    /// # Errors
    /// On I/O failure or an unexpected response.
    pub async fn apply_events(&self, events: Vec<egui::Event>) -> anyhow::Result<()> {
        self.request_done(Request::ApplyEvents { events }, "ApplyEvents")
            .await
    }

    /// Resize the viewport (logical points).
    ///
    /// # Errors
    /// On I/O failure or an unexpected response.
    pub async fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        self.request_done(Request::Resize { width, height }, "Resize")
            .await
    }

    /// Capture a screenshot.
    ///
    /// # Errors
    /// On I/O failure or an unexpected response.
    pub async fn screenshot(&self) -> anyhow::Result<EncodedPng> {
        match self.request(Request::GetScreenshot).await? {
            Response::Screenshot(png) => Ok(png),
            Response::Error { message } => bail!("{message}"),
            _ => bail!("unexpected response to GetScreenshot"),
        }
    }
}

/// A freshly-fetched AccessKit tree plus the geometry needed to map it to screen pixels.
pub struct TreeSnapshot {
    pub step: u64,
    pub pixels_per_point: f32,
    pub tree: Option<accesskit_consumer::Tree>,
}

// The wire format (length-prefix + MessagePack, capped at `MAX_MESSAGE_BYTES`) is defined
// once in `egui_inspection::protocol`; these are just the async read/write around it.
async fn write_framed<W, T>(writer: &mut W, value: &T) -> anyhow::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    writer.write_all(&encode_frame(value)?).await?;
    writer.flush().await?;
    Ok(())
}

async fn read_framed<R, T>(reader: &mut R) -> anyhow::Result<T>
where
    R: AsyncRead + Unpin,
    T: for<'de> serde::Deserialize<'de>,
{
    let mut header = [0u8; 4];
    reader.read_exact(&mut header).await?;
    let mut body = vec![0u8; decode_frame_len(header)?];
    reader.read_exact(&mut body).await?;
    Ok(decode_frame_body(&body)?)
}
