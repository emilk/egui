use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use log::{info, warn};
use tungstenite::{Message, accept};

use crate::{
    agent_bridge::AgentBridge,
    runtime::{AgentCommand, AgentEnvelope, AgentRuntime, AgentUpdate, ControlAction, MessageRole},
};

/// Error type returned by AgentEnvelope WebSocket bridge helpers.
pub type AgentWsServerResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Default address used by the local UI WebSocket bridge.
pub const DEFAULT_AGENT_WS_ADDR: &str = "127.0.0.1:9001";

const ACCEPT_SLEEP: Duration = Duration::from_millis(50);
const READ_TIMEOUT: Duration = Duration::from_millis(200);

struct ClientSlot {
    connected: Arc<AtomicBool>,
}

impl ClientSlot {
    fn new(connected: Arc<AtomicBool>) -> Self {
        Self { connected }
    }
}

impl Drop for ClientSlot {
    fn drop(&mut self) {
        self.connected.store(false, Ordering::SeqCst);
    }
}

/// Handle used to manage a background AgentEnvelope WebSocket bridge thread.
pub struct AgentWsServerHandle {
    addr: SocketAddr,
    url: String,
    running: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl AgentWsServerHandle {
    /// The websocket URL clients can connect to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The socket address the server is bound to.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Signal the server to stop and wait for the thread to exit.
    pub fn stop(mut self) {
        self.shutdown();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }

    /// Signal the server to stop without waiting for the thread to exit.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Drop for AgentWsServerHandle {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Local WebSocket bridge for the egui UI layer (AgentEnvelope protocol, not MCP spec).
pub struct AgentWsServer;

impl AgentWsServer {
    /// Start a blocking WebSocket bridge on the provided socket address.
    pub fn serve(addr: &str) -> AgentWsServerResult<()> {
        Self::serve_inner(addr, None)
    }

    /// Start a blocking WebSocket bridge backed by an [`AgentBridge`].
    pub fn serve_with_bridge(addr: &str, bridge: Arc<AgentBridge>) -> AgentWsServerResult<()> {
        Self::serve_inner(addr, Some(bridge))
    }

    fn serve_inner(addr: &str, bridge: Option<Arc<AgentBridge>>) -> AgentWsServerResult<()> {
        let listener = TcpListener::bind(addr)?;
        info!("agent_ws listening on ws://{addr}");
        let client_slot = Arc::new(AtomicBool::new(false));
        for stream in listener.incoming() {
            let stream = stream?;
            if client_slot
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                let client_slot = ClientSlot::new(Arc::clone(&client_slot));
                let bridge = bridge.clone();
                let _ = thread::Builder::new()
                    .name("eframe_agent_ws_client".to_string())
                    .spawn(move || {
                        if let Err(err) = handle_client(stream, None, Some(client_slot), bridge) {
                            warn!("client disconnected: {err}");
                        }
                    })
                    .expect("failed to spawn agent ws client thread");
            } else {
                warn!("rejecting agent WS client: already connected");
            }
        }
        Ok(())
    }

    /// Spawn a background WebSocket bridge on the provided socket address.
    pub fn spawn(addr: &str) -> AgentWsServerResult<AgentWsServerHandle> {
        Self::spawn_inner(addr, None)
    }

    /// Spawn a background WebSocket bridge backed by an [`AgentBridge`].
    pub fn spawn_with_bridge(
        addr: &str,
        bridge: Arc<AgentBridge>,
    ) -> AgentWsServerResult<AgentWsServerHandle> {
        Self::spawn_inner(addr, Some(bridge))
    }

    /// Spawn a background WebSocket bridge backed by an [`AgentRuntime`].
    pub fn spawn_with_runtime(
        addr: &str,
        runtime: Arc<dyn AgentRuntime>,
    ) -> AgentWsServerResult<(AgentWsServerHandle, Arc<AgentBridge>)> {
        let bridge = Arc::new(AgentBridge::new(runtime));
        let handle = Self::spawn_with_bridge(addr, Arc::clone(&bridge))?;
        Ok((handle, bridge))
    }

    fn spawn_inner(
        addr: &str,
        bridge: Option<Arc<AgentBridge>>,
    ) -> AgentWsServerResult<AgentWsServerHandle> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let url = format!("ws://{addr}");
        let running = Arc::new(AtomicBool::new(true));
        let running_thread = Arc::clone(&running);
        let client_slot = Arc::new(AtomicBool::new(false));
        let bridge = Arc::new(bridge);

        let join = thread::Builder::new()
            .name("eframe_agent_ws_server".to_string())
            .spawn(move || {
                while running_thread.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            if client_slot
                                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                                .is_ok()
                            {
                                let running_client = Arc::clone(&running_thread);
                                let client_slot = ClientSlot::new(Arc::clone(&client_slot));
                                let bridge = bridge.clone();
                                let _ = thread::Builder::new()
                                    .name("eframe_agent_ws_client".to_string())
                                    .spawn(move || {
                                        if let Err(err) = handle_client(
                                            stream,
                                            Some(running_client),
                                            Some(client_slot),
                                            bridge.as_ref().clone(),
                                        ) {
                                            warn!("client disconnected: {err}");
                                        }
                                    })
                                    .expect("failed to spawn agent ws client thread");
                            } else {
                                warn!("rejecting agent WS client: already connected");
                            }
                        }
                        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(ACCEPT_SLEEP);
                        }
                        Err(err) => {
                            warn!("agent WS accept failed: {err}");
                            thread::sleep(ACCEPT_SLEEP);
                        }
                    }
                }
            })
            .expect("failed to spawn agent ws server thread");

        Ok(AgentWsServerHandle {
            addr,
            url,
            running,
            join: Some(join),
        })
    }

    /// Spawn a background WebSocket bridge on the default address.
    pub fn spawn_default() -> AgentWsServerResult<AgentWsServerHandle> {
        Self::spawn(DEFAULT_AGENT_WS_ADDR)
    }
}

fn handle_client(
    stream: TcpStream,
    running: Option<Arc<AtomicBool>>,
    client_slot: Option<ClientSlot>,
    bridge: Option<Arc<AgentBridge>>,
) -> AgentWsServerResult<()> {
    let _client_slot = client_slot;
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    let mut socket = accept(stream)?;
    let mut next_task_id = 1_u64;
    let mut update_rx = bridge.as_ref().map(|bridge| bridge.subscribe());

    loop {
        if let Some(running) = running.as_ref()
            && !running.load(Ordering::Relaxed)
        {
            break;
        }

        match socket.read() {
            Ok(message) => {
                let text = match message {
                    Message::Text(text) => text,
                    Message::Binary(payload) => String::from_utf8(payload)?,
                    Message::Close(_) => break,
                    _ => continue,
                };

                let payload = serde_json::from_str::<AgentEnvelope>(&text)?;
                if let AgentEnvelope::Command { command } = payload {
                    if let Some(bridge) = bridge.as_ref() {
                        if matches!(command, AgentCommand::Shutdown) {
                            send_update(
                                &mut socket,
                                AgentUpdate::Control {
                                    action: ControlAction::CloseWindow,
                                },
                            )?;
                            bridge.submit_command(command);
                            let _ = socket.close(None);
                            break;
                        }
                        bridge.submit_command(command);
                    } else if let Err(err) =
                        respond_to_command(&mut socket, &mut next_task_id, command)
                    {
                        if let Some(ws_err) = err.downcast_ref::<tungstenite::Error>()
                            && is_connection_closed_error(ws_err)
                        {
                            break;
                        }
                        return Err(err);
                    }
                }
            }
            Err(tungstenite::Error::Io(err))
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut => {}
            Err(err) => {
                if is_connection_closed_error(&err) {
                    break;
                }
                return Err(Box::new(err));
            }
        }

        if let Some(rx) = update_rx.as_mut()
            && !drain_updates(&mut socket, rx)?
        {
            break;
        }
    }

    Ok(())
}

fn is_connection_closed_error(err: &tungstenite::Error) -> bool {
    match err {
        tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => true,
        tungstenite::Error::Protocol(
            tungstenite::error::ProtocolError::ResetWithoutClosingHandshake,
        ) => true,
        tungstenite::Error::Io(err) => is_connection_closed_io(err),
        _ => false,
    }
}

fn is_connection_closed_io(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::NotConnected
    )
}

fn respond_to_command(
    socket: &mut tungstenite::WebSocket<TcpStream>,
    next_task_id: &mut u64,
    command: AgentCommand,
) -> AgentWsServerResult<()> {
    match command {
        AgentCommand::SubmitPrompt(prompt) => {
            let task_id = *next_task_id;
            *next_task_id += 1;
            let updates = [
                AgentUpdate::Message {
                    role: MessageRole::User,
                    text: prompt.clone(),
                },
                AgentUpdate::TaskStarted {
                    id: task_id,
                    label: "agent-ws-task".into(),
                },
                AgentUpdate::Message {
                    role: MessageRole::Agent,
                    text: format!("echo: {prompt}"),
                },
                AgentUpdate::TaskFinished {
                    id: task_id,
                    label: "agent-ws-task".into(),
                    success: true,
                },
            ];
            for update in updates {
                send_update(socket, update)?;
            }
        }
        AgentCommand::CancelActiveTask => {
            send_update(
                socket,
                AgentUpdate::Message {
                    role: MessageRole::System,
                    text: "Cancel requested (agent WS stub).".into(),
                },
            )?;
        }
        AgentCommand::ClearHistory => {
            send_update(socket, AgentUpdate::Reset)?;
        }
        AgentCommand::Shutdown => {
            send_update(
                socket,
                AgentUpdate::Control {
                    action: ControlAction::CloseWindow,
                },
            )?;
            let _ = socket.close(None);
        }
    }

    Ok(())
}

fn send_update(
    socket: &mut tungstenite::WebSocket<TcpStream>,
    update: AgentUpdate,
) -> AgentWsServerResult<()> {
    let payload = AgentEnvelope::Update { update };
    let text = serde_json::to_string(&payload)?;
    socket.send(Message::Text(text))?;
    Ok(())
}

fn drain_updates(
    socket: &mut tungstenite::WebSocket<TcpStream>,
    rx: &std::sync::mpsc::Receiver<AgentUpdate>,
) -> AgentWsServerResult<bool> {
    while let Ok(update) = rx.try_recv() {
        if let Err(err) = send_update(socket, update) {
            if let Some(ws_err) = err.downcast_ref::<tungstenite::Error>()
                && is_connection_closed_error(ws_err)
            {
                return Ok(false);
            }
            return Err(err);
        }
    }
    Ok(true)
}
