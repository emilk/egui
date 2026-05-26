//! Bridge between the MCP server and a running egui peer (a spawned kittest harness or an
//! attached live app), both reached over the same `egui_inspection` local socket.
//!
//! Lifecycle:
//! 1. [`Bridge::launch`] (kittest harness) / [`Bridge::prepare_attach`] + [`Bridge::accept_pending`]
//!    (live app) bind a local socket and point the peer at it via
//!    [`egui_inspection::INSPECTION_SOCKET_ENV_VAR`]; the peer dials in directly.
//! 2. A reader task decodes [`HarnessMessage`]s from the socket and updates [`SharedState`].
//! 3. A writer task drains [`InspectorCommand`]s queued by MCP tool handlers and writes
//!    them to the socket.
//! 4. Tool handlers observe [`SharedState`] via [`Bridge::snapshot`] and wait for new
//!    frames or `Finished` via [`Bridge::wait_for_frame_after`].

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, anyhow, bail};
use egui_inspection::protocol::{Frame, HarnessMessage, InspectorCommand, SourceView};
use egui_inspection::transport::{SocketTarget, generate_socket_target, socket_name};
use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::tokio::{Listener, RecvHalf, SendHalf, prelude::*};
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, Notify, mpsc};
use tokio::task::JoinHandle;
use tokio::time::timeout;

/// Hard cap matching `inspector_api::MAX_MESSAGE_BYTES` so framing-level DoS is bounded.
const MAX_MESSAGE_BYTES: usize = 256 * 1024 * 1024;

/// Accept timeout for [`Bridge::launch`]. Generous because the spawned `cargo test` / `cargo
/// run` child typically compiles before its harness dials in.
const LAUNCH_ACCEPT_TIMEOUT_SECS: u64 = 120;

/// One in-flight peer (a spawned kittest harness or an attached live app) + the tasks
/// that talk to it.
pub struct Bridge {
    pub state: Arc<SharedState>,
    /// Outgoing command queue → writer task → socket.
    cmd_tx: mpsc::UnboundedSender<InspectorCommand>,
    /// Tokio task handles. Aborted on `Drop`; the child is killed too.
    _reader_task: JoinHandle<()>,
    _writer_task: JoinHandle<()>,
    /// `Child` wrapped in a `Mutex` so a `kill` tool can take it. `None` in attach mode —
    /// we don't own the lifecycle of an externally-started app.
    child: Arc<Mutex<Option<Child>>>,
    /// Local-socket target — kept alive while the bridge is so its backing socket file
    /// (on unix) survives.
    _socket_target: SocketTarget,
    /// How this bridge was created (informational).
    pub peer_info: PeerInfo,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PeerInfo {
    /// Bridge spawned a child harness process.
    Launched {
        bin: PathBuf,
        args: Vec<String>,
        pid: u32,
    },
    /// Bridge bound a socket and accepted an incoming connection from a live app.
    Attached { socket: String },
}

/// Mutable state observed by MCP tool handlers.
///
/// Guarded by a `Mutex` (not `RwLock`) because writers and readers contend on the same
/// fields and acquire-cost is dominated by the rare `Frame` arrival, not lock contention.
pub struct SharedState {
    inner: Mutex<Inner>,
    /// Notified whenever `inner` changes in a way a waiter might care about (new frame,
    /// blocked transition, finished). Coarse-grained on purpose.
    notify: Notify,
}

#[derive(Default)]
struct Inner {
    /// Set on receipt of [`HarnessMessage::Hello`]. `None` until the peer connects.
    pub hello: Option<egui_inspection::protocol::PeerHello>,
    pub latest_frame: Option<Box<Frame>>,
    pub blocked: bool,
    pub finished: Option<FinishedInfo>,
    /// Latest accesskit tree (re-built each time a `TreeUpdate` arrives).
    pub accesskit_tree: Option<accesskit_consumer::Tree>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FinishedInfo {
    pub ok: bool,
    pub message: Option<String>,
    pub source: Option<SourceView>,
}

/// Snapshot returned to tool handlers so they can drop the mutex before responding.
#[derive(Clone)]
pub struct StateSnapshot {
    /// Peer identity + capabilities, captured at connect time. Used by tool handlers to
    /// gate commands the peer doesn't honor (Step/Run/Pause against a live app, etc.).
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "consumed by upcoming capability-gating in tool handlers")
    )]
    pub hello: Option<egui_inspection::protocol::PeerHello>,
    pub frame: Option<Box<Frame>>,
    pub blocked: bool,
    pub finished: Option<FinishedInfo>,
}

impl SharedState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner::default()),
            notify: Notify::new(),
        })
    }

    pub async fn snapshot(&self) -> StateSnapshot {
        let g = self.inner.lock().await;
        StateSnapshot {
            hello: g.hello.clone(),
            frame: g.latest_frame.clone(),
            blocked: g.blocked,
            finished: g.finished.clone(),
        }
    }

    /// Read-only access to the accesskit tree via a closure. The tree isn't `Clone`, so
    /// callers project the data they need (node list, lookup by id) before returning.
    pub async fn with_tree<R>(
        &self,
        f: impl FnOnce(Option<&accesskit_consumer::Tree>) -> R,
    ) -> R {
        let g = self.inner.lock().await;
        f(g.accesskit_tree.as_ref())
    }

    /// Await the next state-change notification. Used by tools that poll (e.g. `wait_for`)
    /// to wake on a new frame / blocked transition without busy-looping.
    pub async fn notified(&self) {
        self.notify.notified().await;
    }
}

impl Bridge {
    /// Spawn a kittest harness binary and bridge to it. Binds a local socket, spawns the
    /// child with [`egui_inspection::INSPECTION_SOCKET_ENV_VAR`] pointed at it, and accepts
    /// the harness's inbound connection — the same mechanism as [`Self::prepare_attach`] +
    /// [`Self::accept_pending`], which it reuses.
    pub async fn launch(
        bin: PathBuf,
        args: Vec<String>,
        env: Vec<(String, String)>,
        cwd: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let (listener, socket_target) = Self::prepare_attach().await?;

        let mut cmd = Command::new(&bin);
        cmd.args(&args)
            .env(egui_inspection::INSPECTION_SOCKET_ENV_VAR, &socket_target.name)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            // Inherit stderr so harness panics and cargo build errors surface where the
            // operator can see them, instead of being silently swallowed.
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true);
        for (k, v) in &env {
            cmd.env(k, v);
        }
        if let Some(d) = &cwd {
            cmd.current_dir(d);
        }

        let child = cmd
            .spawn()
            .with_context(|| format!("spawn {}", bin.display()))?;
        let pid = child.id().unwrap_or(0);

        // The child usually compiles before it runs (cargo test/run), so allow a generous
        // window before giving up on the handshake.
        let accept_timeout = Duration::from_secs(LAUNCH_ACCEPT_TIMEOUT_SECS);
        let mut bridge =
            Self::accept_pending(listener, socket_target, Some(child), accept_timeout).await?;
        bridge.peer_info = PeerInfo::Launched { bin, args, pid };
        Ok(bridge)
    }

    /// Bind a local socket and return it immediately. The caller is responsible for
    /// starting the app with `EGUI_INSPECTION_SOCKET` set to the returned target's name.
    /// Call [`Self::accept_pending`] once the app is running.
    ///
    /// Returns the listener and the socket target (must be kept alive on unix).
    pub async fn prepare_attach() -> anyhow::Result<(Listener, SocketTarget)> {
        let socket_target =
            generate_socket_target().context("allocate inspection socket")?;
        let name = socket_name(&socket_target.name)
            .with_context(|| format!("parse socket name {}", socket_target.name))?;
        let listener = ListenerOptions::new()
            .name(name)
            .create_tokio()
            .with_context(|| format!("bind {}", socket_target.name))?;
        Ok((listener, socket_target))
    }

    /// Finish an attach started with [`Self::prepare_attach`] — wait for an inbound
    /// connection and spawn the reader/writer tasks.
    ///
    /// `child` is the optional child process that was spawned with the socket env var
    /// pre-set. Passing it here lets `kill` reach it and `kill_on_drop` clean up if the
    /// bridge is dropped.
    pub async fn accept_pending(
        listener: Listener,
        socket_target: SocketTarget,
        child: Option<Child>,
        accept_timeout: Duration,
    ) -> anyhow::Result<Self> {
        let stream = match timeout(accept_timeout, listener.accept()).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => bail!("accept on inspection socket: {e}"),
            Err(_) => bail!(
                "timed out waiting for inbound connection at {}",
                socket_target.name
            ),
        };

        let socket = socket_target.name.clone();
        let (reader, writer) = stream.split();
        let state = SharedState::new();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let child_arc: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(child));
        let reader_task = tokio::spawn(read_loop(reader, state.clone(), child_arc.clone()));
        let writer_task = tokio::spawn(write_loop(writer, cmd_rx));

        Ok(Self {
            state,
            cmd_tx,
            _reader_task: reader_task,
            _writer_task: writer_task,
            child: child_arc,
            _socket_target: socket_target,
            peer_info: PeerInfo::Attached { socket },
        })
    }

    pub fn send(&self, cmd: InspectorCommand) -> anyhow::Result<()> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| anyhow!("inspector writer task is gone"))
    }

    /// Wait for either a new frame whose `step > prev_step`, or a `Finished` signal,
    /// whichever comes first. Returns the resulting snapshot or times out.
    pub async fn wait_for_frame_after(
        &self,
        prev_step: u64,
        wait: Duration,
    ) -> anyhow::Result<StateSnapshot> {
        let deadline = tokio::time::Instant::now() + wait;
        loop {
            let snap = self.state.snapshot().await;
            if snap.finished.is_some() {
                return Ok(snap);
            }
            if let Some(f) = &snap.frame {
                if f.step > prev_step {
                    return Ok(snap);
                }
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                bail!("timed out waiting for next frame after step {prev_step}");
            }
            let _ = timeout(remaining, self.state.notify.notified()).await;
        }
    }

    pub async fn kill(&self) {
        if let Some(mut c) = self.child.lock().await.take() {
            let _ = c.kill().await;
        }
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        // Best-effort: ensure the child is reaped. `kill_on_drop(true)` on `Command` also
        // guarantees this, but we set it explicitly for the case where someone replaces the
        // `Child` and forgets the flag.
        if let Ok(mut g) = self.child.try_lock() {
            if let Some(mut c) = g.take() {
                let _ = c.start_kill();
            }
        }
    }
}

async fn read_loop(
    mut reader: RecvHalf,
    state: Arc<SharedState>,
    child: Arc<Mutex<Option<Child>>>,
) {
    loop {
        let msg = match read_message(&mut reader).await {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!("inspector socket read ended: {e}");
                break;
            }
        };
        apply_message(&state, msg).await;
    }
    // Reader ended → harness is gone. Make sure we eventually reap the child.
    if let Some(mut c) = child.lock().await.take() {
        let _ = c.kill().await;
    }
    // Wake any waiter so they can observe disconnection.
    state.notify.notify_waiters();
}

async fn apply_message(state: &SharedState, msg: HarnessMessage) {
    let mut g = state.inner.lock().await;
    match msg {
        HarnessMessage::Hello(hello) => {
            g.hello = Some(hello);
        }
        HarnessMessage::Frame(frame) => {
            if let Some(update) = &frame.accesskit {
                let mut noop = NoopChangeHandler;
                match g.accesskit_tree.as_mut() {
                    Some(tree) => tree.update_and_process_changes(update.clone(), &mut noop),
                    None => {
                        g.accesskit_tree =
                            Some(accesskit_consumer::Tree::new(update.clone(), false));
                    }
                }
            }
            g.latest_frame = Some(frame);
        }
        HarnessMessage::Blocked(b) => g.blocked = b,
        HarnessMessage::Finished {
            ok,
            message,
            source,
        } => {
            g.finished = Some(FinishedInfo {
                ok,
                message,
                source,
            });
            g.blocked = true;
        }
    }
    drop(g);
    state.notify.notify_waiters();
}

struct NoopChangeHandler;

impl accesskit_consumer::TreeChangeHandler for NoopChangeHandler {
    fn node_added(&mut self, _: &accesskit_consumer::Node<'_>) {}
    fn node_updated(
        &mut self,
        _: &accesskit_consumer::Node<'_>,
        _: &accesskit_consumer::Node<'_>,
    ) {
    }
    fn focus_moved(
        &mut self,
        _: Option<&accesskit_consumer::Node<'_>>,
        _: Option<&accesskit_consumer::Node<'_>>,
    ) {
    }
    fn node_removed(&mut self, _: &accesskit_consumer::Node<'_>) {}
}

async fn write_loop(
    mut writer: SendHalf,
    mut rx: mpsc::UnboundedReceiver<InspectorCommand>,
) {
    while let Some(cmd) = rx.recv().await {
        if let Err(e) = write_message(&mut writer, &cmd).await {
            tracing::debug!("inspector socket write ended: {e}");
            break;
        }
    }
}

async fn read_message<R: AsyncRead + Unpin>(stream: &mut R) -> anyhow::Result<HarnessMessage> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_MESSAGE_BYTES {
        bail!("message too large: {len} bytes");
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    rmp_serde::from_slice(&buf).map_err(|e| anyhow!("decode: {e}"))
}

async fn write_message<W: AsyncWrite + Unpin>(
    stream: &mut W,
    msg: &InspectorCommand,
) -> anyhow::Result<()> {
    let bytes = rmp_serde::to_vec(msg).map_err(|e| anyhow!("encode: {e}"))?;
    let len = u32::try_from(bytes.len())?;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&bytes).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_inspection::protocol::{
        Capabilities, PROTOCOL_VERSION, PeerHello, PeerKind, write_message,
    };
    use interprocess::local_socket::Stream;
    use interprocess::local_socket::prelude::*;

    /// Full cross-platform transport round-trip: a tokio `interprocess` listener (the bridge
    /// side) accepts a connection from a sync `interprocess` client (the egui peer side), and
    /// a framed `Hello` is decoded into shared state. Runs against whatever local-socket
    /// backend the host uses (unix domain socket on unix, named pipe on Windows).
    #[tokio::test]
    async fn handshake_roundtrip() {
        let (listener, target) = Bridge::prepare_attach().await.unwrap();
        let name = target.name.clone();

        // Connect + write from a blocking thread, mirroring how an egui peer dials in.
        let client = std::thread::spawn(move || {
            let n = socket_name(&name).unwrap();
            let mut stream = Stream::connect(n).unwrap();
            let hello = HarnessMessage::Hello(PeerHello {
                protocol_version: PROTOCOL_VERSION,
                peer_kind: PeerKind::Live,
                capabilities: Capabilities::LIVE,
                continuous_screenshots: false,
                label: Some("test".to_owned()),
            });
            write_message(&mut stream, &hello).unwrap();
            // Hold the connection open until the bridge has read the message.
            std::thread::sleep(Duration::from_millis(500));
        });

        let bridge = Bridge::accept_pending(listener, target, None, Duration::from_secs(5))
            .await
            .unwrap();

        // The reader task applies the Hello asynchronously; poll briefly for it.
        let mut hello = None;
        for _ in 0..50 {
            if let Some(h) = bridge.state.snapshot().await.hello {
                hello = Some(h);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        client.join().unwrap();

        let hello = hello.expect("bridge should receive Hello over the transport");
        assert_eq!(hello.peer_kind, PeerKind::Live);
        assert_eq!(hello.label.as_deref(), Some("test"));
    }
}
