//! Bridge between the MCP server and a running kittest harness child process.
//!
//! Lifecycle:
//! 1. [`Bridge::launch`] binds a unix domain socket, spawns the target binary with
//!    [`crate::HANDSHAKE_ENV_VAR`] + `KITTEST_INSPECTOR=1` +
//!    `KITTEST_INSPECTOR_PATH=<self>`, and waits for the shim to connect.
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
use serde::Serialize;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::UnixListener;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, Notify, mpsc};
use tokio::task::JoinHandle;
use tokio::time::timeout;

/// Hard cap matching `inspector_api::MAX_MESSAGE_BYTES` so framing-level DoS is bounded.
const MAX_MESSAGE_BYTES: usize = 256 * 1024 * 1024;

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
    /// Temp dir holding the unix socket — kept alive while the bridge is.
    _socket_dir: tempfile::TempDir,
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
    Attached { socket_path: PathBuf },
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
    #[expect(dead_code, reason = "consumed by upcoming capability-gating in tool handlers")]
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
    pub async fn launch(
        bin: PathBuf,
        args: Vec<String>,
        env: Vec<(String, String)>,
        cwd: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let self_path = std::env::current_exe()
            .context("get current_exe for KITTEST_INSPECTOR_PATH")?;

        let socket_dir = tempfile::Builder::new()
            .prefix("kittest-mcp-")
            .tempdir()
            .context("create temp dir for handshake socket")?;
        let socket_path = socket_dir.path().join("kittest.sock");

        let listener = UnixListener::bind(&socket_path)
            .with_context(|| format!("bind {}", socket_path.display()))?;

        let mut cmd = Command::new(&bin);
        cmd.args(&args)
            .env("KITTEST_INSPECTOR", "1")
            .env("KITTEST_INSPECTOR_PATH", &self_path)
            .env(crate::HANDSHAKE_ENV_VAR, &socket_path)
            .stdin(std::process::Stdio::null())
            // Harness inspector path: the child's stdout/stderr aren't ours — they get
            // captured by the shim. We don't need them in the MCP server.
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        for (k, v) in &env {
            cmd.env(k, v);
        }
        if let Some(d) = &cwd {
            cmd.current_dir(d);
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("spawn {}", bin.display()))?;
        let pid = child.id().unwrap_or(0);

        // Accept with a short timeout. If the binary fails to start, exits early, or
        // doesn't have the inspector wired up, we surface that instead of hanging forever.
        let (stream, _addr) = match timeout(Duration::from_secs(10), listener.accept()).await {
            Ok(Ok(pair)) => pair,
            Ok(Err(e)) => {
                let _ = child.kill().await;
                bail!("accept on handshake socket: {e}");
            }
            Err(_) => {
                let _ = child.kill().await;
                // Try to report the child's exit status if it died early.
                let status_hint = match child.try_wait() {
                    Ok(Some(s)) => format!(" (child exited {s})"),
                    _ => String::new(),
                };
                bail!("timed out waiting for inspector handshake{status_hint}");
            }
        };

        let (reader, writer) = stream.into_split();
        let state = SharedState::new();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let child_arc: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(Some(child)));

        let reader_task = tokio::spawn(read_loop(reader, state.clone(), child_arc.clone()));
        let writer_task = tokio::spawn(write_loop(writer, cmd_rx));

        Ok(Self {
            state,
            cmd_tx,
            _reader_task: reader_task,
            _writer_task: writer_task,
            child: child_arc,
            _socket_dir: socket_dir,
            peer_info: PeerInfo::Launched { bin, args, pid },
        })
    }

    /// Bind a unix socket and return the path immediately. The caller is responsible for
    /// starting the app with `EGUI_INSPECTION_SOCKET` set to this path. Call
    /// [`Self::accept_pending`] once the app is running.
    ///
    /// Returns the temp-dir handle (must be kept alive) and the listener.
    pub async fn prepare_attach() -> anyhow::Result<(tempfile::TempDir, UnixListener, PathBuf)> {
        let socket_dir = tempfile::Builder::new()
            .prefix("egui-inspection-")
            .tempdir()
            .context("create temp dir for inspection socket")?;
        let socket_path = socket_dir.path().join("inspection.sock");
        let listener = UnixListener::bind(&socket_path)
            .with_context(|| format!("bind {}", socket_path.display()))?;
        Ok((socket_dir, listener, socket_path))
    }

    /// Finish an attach started with [`Self::prepare_attach`] — wait for an inbound
    /// connection and spawn the reader/writer tasks.
    ///
    /// `child` is the optional child process that was spawned with the socket env var
    /// pre-set. Passing it here lets `kill` reach it and `kill_on_drop` clean up if the
    /// bridge is dropped.
    pub async fn accept_pending(
        socket_dir: tempfile::TempDir,
        listener: UnixListener,
        socket_path: PathBuf,
        child: Option<Child>,
        accept_timeout: Duration,
    ) -> anyhow::Result<Self> {
        let (stream, _addr) = match timeout(accept_timeout, listener.accept()).await {
            Ok(Ok(pair)) => pair,
            Ok(Err(e)) => bail!("accept on inspection socket: {e}"),
            Err(_) => bail!("timed out waiting for inbound connection at {}", socket_path.display()),
        };

        let (reader, writer) = stream.into_split();
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
            _socket_dir: socket_dir,
            peer_info: PeerInfo::Attached { socket_path },
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
    mut reader: tokio::net::unix::OwnedReadHalf,
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
    mut writer: tokio::net::unix::OwnedWriteHalf,
    mut rx: mpsc::UnboundedReceiver<InspectorCommand>,
) {
    while let Some(cmd) = rx.recv().await {
        if let Err(e) = write_message(&mut writer, &cmd).await {
            tracing::debug!("inspector socket write ended: {e}");
            break;
        }
    }
}

async fn read_message(stream: &mut tokio::net::unix::OwnedReadHalf) -> anyhow::Result<HarnessMessage> {
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

async fn write_message(
    stream: &mut tokio::net::unix::OwnedWriteHalf,
    msg: &InspectorCommand,
) -> anyhow::Result<()> {
    let bytes = rmp_serde::to_vec(msg).map_err(|e| anyhow!("encode: {e}"))?;
    let len = u32::try_from(bytes.len())?;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&bytes).await?;
    stream.flush().await?;
    Ok(())
}
