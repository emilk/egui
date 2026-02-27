//! WebSocket runtime for the egui agent UI using `AgentEnvelope` messages.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::runtime::{AgentRuntime, AgentUpdate, MessageRole};

/// Default WebSocket bridge URL for AgentEnvelope traffic.
const DEFAULT_AGENT_WS_URL: &str = "ws://127.0.0.1:9001";

fn push_system_message(updates: &Arc<Mutex<Vec<AgentUpdate>>>, text: impl Into<String>) {
    updates.lock().push(AgentUpdate::Message {
        role: MessageRole::System,
        text: text.into(),
    });
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        collections::VecDeque,
        env,
        net::TcpStream,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        thread,
        time::{Duration, Instant},
    };

    use log::warn;
    use parking_lot::Mutex;
    use tungstenite::stream::MaybeTlsStream;
    use tungstenite::{Message, connect};
    use url::Url;

    use super::{DEFAULT_AGENT_WS_URL, push_system_message};
    use crate::runtime::{AgentCommand, AgentEnvelope, AgentRuntime, AgentUpdate};

    const READ_TIMEOUT: Duration = Duration::from_millis(50);
    const RECONNECT_DELAY: Duration = Duration::from_secs(1);

    type Socket = tungstenite::WebSocket<MaybeTlsStream<TcpStream>>;
    type SocketResult<T> = Result<T, Box<tungstenite::Error>>;

    /// AgentEnvelope WebSocket runtime for native platforms (not MCP spec).
    pub struct AgentWsRuntime {
        updates: Arc<Mutex<Vec<AgentUpdate>>>,
        command_tx: mpsc::Sender<AgentCommand>,
        is_running: Arc<AtomicBool>,
        worker: Mutex<Option<thread::JoinHandle<()>>>,
    }

    impl AgentWsRuntime {
        /// Connect to the AgentEnvelope WebSocket bridge at `url`.
        pub fn connect(url: &str) -> Self {
            let updates = Arc::new(Mutex::new(Vec::new()));
            let (command_tx, command_rx) = mpsc::channel::<AgentCommand>();
            let is_running = Arc::new(AtomicBool::new(true));

            let Ok(url) = Url::parse(url) else {
                push_system_message(&updates, format!("Invalid agent WS url: {url}"));
                return Self {
                    updates,
                    command_tx,
                    is_running,
                    worker: Mutex::new(None),
                };
            };

            let updates_worker = Arc::clone(&updates);
            let running_worker = Arc::clone(&is_running);

            let worker = thread::Builder::new()
                .name("eframe_agent_ws_runtime".to_string())
                .spawn(move || {
                    let mut pending = VecDeque::new();
                    let mut socket: Option<Socket> = None;
                    let mut next_retry = Instant::now();
                    let mut reported_error = false;

                    while running_worker.load(Ordering::Relaxed) {
                        while let Ok(command) = command_rx.try_recv() {
                            pending.push_back(command);
                        }

                        if socket.is_none() && Instant::now() >= next_retry {
                            match connect_socket(&url) {
                                Ok(sock) => {
                                    socket = Some(sock);
                                    reported_error = false;
                                }
                                Err(err) => {
                                    if !reported_error {
                                        push_system_message(
                                            &updates_worker,
                                            format!("Agent WS connect failed: {err}"),
                                        );
                                        reported_error = true;
                                    }
                                    next_retry = Instant::now() + RECONNECT_DELAY;
                                }
                            }
                        }

                        if let Some(sock) = socket.as_mut() {
                            if let Err(err) = flush_pending(sock, &mut pending) {
                                warn!("Agent WS send error: {err}");
                                socket = None;
                                reported_error = false;
                                next_retry = Instant::now() + RECONNECT_DELAY;
                                continue;
                            }

                            match sock.read() {
                                Ok(message) => match message {
                                    Message::Close(_) => {
                                        if !reported_error {
                                            push_system_message(
                                                &updates_worker,
                                                "Agent WS connection closed",
                                            );
                                            reported_error = true;
                                        }
                                        socket = None;
                                        next_retry = Instant::now() + RECONNECT_DELAY;
                                    }
                                    other => {
                                        if let Some(text) = message_to_text(other) {
                                            match serde_json::from_str::<AgentEnvelope>(&text) {
                                                Ok(AgentEnvelope::Update { update }) => {
                                                    updates_worker.lock().push(update);
                                                }
                                                Ok(_) => {}
                                                Err(err) => {
                                                    warn!("Agent WS decode error: {err}");
                                                }
                                            }
                                        }
                                    }
                                },
                                Err(tungstenite::Error::Io(err))
                                    if err.kind() == std::io::ErrorKind::WouldBlock
                                        || err.kind() == std::io::ErrorKind::TimedOut => {}
                                Err(err) => {
                                    warn!("Agent WS socket error: {err}");
                                    if !reported_error {
                                        push_system_message(
                                            &updates_worker,
                                            format!("Agent WS disconnected: {err}"),
                                        );
                                        reported_error = true;
                                    }
                                    socket = None;
                                    next_retry = Instant::now() + RECONNECT_DELAY;
                                }
                            }
                        } else {
                            thread::sleep(READ_TIMEOUT);
                        }
                    }
                })
                .expect("failed to spawn agent ws runtime thread");

            Self {
                updates,
                command_tx,
                is_running,
                worker: Mutex::new(Some(worker)),
            }
        }

        /// Connect using `AGENT_WS_URL` or fall back to the default bridge URL.
        pub fn connect_from_env() -> Self {
            let url = env::var("AGENT_WS_URL").unwrap_or_else(|_| DEFAULT_AGENT_WS_URL.into());
            Self::connect(&url)
        }
    }

    impl AgentRuntime for AgentWsRuntime {
        fn submit_command(&self, command: AgentCommand) {
            let _ = self.command_tx.send(command);
        }

        fn poll_updates(&self, out: &mut Vec<AgentUpdate>) {
            let mut updates = self.updates.lock();
            out.extend(updates.drain(..));
        }

        fn shutdown(&self) {
            if self
                .is_running
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
                && let Some(handle) = self.worker.lock().take()
            {
                handle.join().ok();
            }
        }
    }

    impl Drop for AgentWsRuntime {
        fn drop(&mut self) {
            self.shutdown();
        }
    }

    fn connect_socket(url: &Url) -> SocketResult<Socket> {
        let (mut socket, _response) = connect(url.as_str()).map_err(Box::new)?;
        if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
            let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
        }
        Ok(socket)
    }

    fn flush_pending(
        socket: &mut Socket,
        pending: &mut VecDeque<AgentCommand>,
    ) -> SocketResult<()> {
        while let Some(command) = pending.pop_front() {
            let payload = AgentEnvelope::Command {
                command: command.clone(),
            };
            let text = match serde_json::to_string(&payload) {
                Ok(text) => text,
                Err(err) => {
                    warn!("Agent WS encode error: {err}");
                    continue;
                }
            };
            if let Err(err) = socket.send(Message::Text(text)) {
                pending.push_front(command);
                return Err(Box::new(err));
            }
        }
        Ok(())
    }

    fn message_to_text(message: Message) -> Option<String> {
        match message {
            Message::Text(text) => Some(text),
            Message::Binary(payload) => String::from_utf8(payload).ok(),
            _ => None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        sync::Arc,
    };

    use parking_lot::Mutex;
    use wasm_bindgen::{JsCast, closure::Closure};
    use web_sys::{CloseEvent, Event, MessageEvent, WebSocket};

    use super::{DEFAULT_AGENT_WS_URL, push_system_message};
    use crate::runtime::{AgentCommand, AgentEnvelope, AgentRuntime, AgentUpdate};

    thread_local! {
        static WS_STATE: RefCell<Option<WsState>> = const { RefCell::new(None) };
    }

    pub struct AgentWsRuntime {
        updates: Arc<Mutex<Vec<AgentUpdate>>>,
    }

    impl AgentWsRuntime {
        pub fn connect(url: &str) -> Self {
            let updates = Arc::new(Mutex::new(Vec::new()));
            match WsState::new(url, Arc::clone(&updates)) {
                Ok(state) => {
                    WS_STATE.with(|cell| cell.replace(Some(state)));
                }
                Err(err) => {
                    WS_STATE.with(|cell| cell.replace(None));
                    push_system_message(&updates, format!("Agent WS error: {err:?}"));
                }
            }
            Self { updates }
        }

        pub fn connect_from_query() -> Self {
            let url = agent_ws_url_from_query().unwrap_or_else(|| DEFAULT_AGENT_WS_URL.into());
            Self::connect(&url)
        }
    }

    impl AgentRuntime for AgentWsRuntime {
        fn submit_command(&self, command: AgentCommand) {
            let payload = AgentEnvelope::Command { command };
            let Ok(text) = serde_json::to_string(&payload) else {
                return;
            };
            WS_STATE.with(|cell| {
                if let Some(state) = cell.borrow().as_ref() {
                    if state.socket.ready_state() == WebSocket::OPEN {
                        let _ = state.socket.send_with_str(&text);
                    } else {
                        state.pending.borrow_mut().push(text);
                    }
                }
            });
        }

        fn poll_updates(&self, out: &mut Vec<AgentUpdate>) {
            let mut updates = self.updates.lock();
            out.extend(updates.drain(..));
        }

        fn shutdown(&self) {
            WS_STATE.with(|cell| {
                if let Some(state) = cell.borrow_mut().take() {
                    let _ = state.socket.close();
                }
            });
        }
    }

    impl Drop for AgentWsRuntime {
        fn drop(&mut self) {
            self.shutdown();
        }
    }

    struct WsState {
        socket: WebSocket,
        pending: Rc<RefCell<Vec<String>>>,
        reported_error: Rc<Cell<bool>>,
        _on_open: Closure<dyn FnMut(Event)>,
        _on_message: Closure<dyn FnMut(MessageEvent)>,
        _on_error: Closure<dyn FnMut(Event)>,
        _on_close: Closure<dyn FnMut(CloseEvent)>,
    }

    impl WsState {
        fn new(
            url: &str,
            updates: Arc<Mutex<Vec<AgentUpdate>>>,
        ) -> Result<Self, wasm_bindgen::JsValue> {
            let socket = WebSocket::new(url)?;
            let pending = Rc::new(RefCell::new(Vec::new()));
            let reported_error = Rc::new(Cell::new(false));

            let on_open = {
                let pending = Rc::clone(&pending);
                let socket = socket.clone();
                let reported_error = Rc::clone(&reported_error);
                Closure::wrap(Box::new(move |_event: Event| {
                    reported_error.set(false);
                    let mut queue = pending.borrow_mut();
                    for message in queue.drain(..) {
                        let _ = socket.send_with_str(&message);
                    }
                }) as Box<dyn FnMut(Event)>)
            };

            let on_message = {
                let updates = Arc::clone(&updates);
                Closure::wrap(Box::new(move |event: MessageEvent| {
                    if let Ok(text) = event.data().dyn_into::<js_sys::JsString>() {
                        if let Ok(payload) = serde_json::from_str::<AgentEnvelope>(&text.into()) {
                            if let AgentEnvelope::Update { update } = payload {
                                updates.lock().push(update);
                            }
                        }
                    }
                }) as Box<dyn FnMut(MessageEvent)>)
            };

            let on_error = {
                let updates = Arc::clone(&updates);
                let reported_error = Rc::clone(&reported_error);
                Closure::wrap(Box::new(move |_event: Event| {
                    if !reported_error.replace(true) {
                        push_system_message(&updates, "Agent WS error");
                    }
                }) as Box<dyn FnMut(Event)>)
            };

            let on_close = {
                let updates = Arc::clone(&updates);
                let reported_error = Rc::clone(&reported_error);
                Closure::wrap(Box::new(move |_event: CloseEvent| {
                    if !reported_error.replace(true) {
                        push_system_message(&updates, "Agent WS closed");
                    }
                }) as Box<dyn FnMut(CloseEvent)>)
            };

            socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
            socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));

            Ok(Self {
                socket,
                pending,
                reported_error,
                _on_open: on_open,
                _on_message: on_message,
                _on_error: on_error,
                _on_close: on_close,
            })
        }
    }

    fn agent_ws_url_from_query() -> Option<String> {
        let window = web_sys::window()?;
        let location = window.location();
        let search = location.search().ok()?;
        for pair in search.trim_start_matches('?').split('&') {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");
            if key == "agent_ws" && !value.is_empty() {
                return Some(value.to_string());
            }
        }
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::AgentWsRuntime;
#[cfg(target_arch = "wasm32")]
pub use web::AgentWsRuntime;

/// Build an AgentEnvelope WebSocket runtime using the platform defaults.
pub fn build_runtime() -> Arc<dyn AgentRuntime> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Arc::new(AgentWsRuntime::connect_from_env())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Arc::new(AgentWsRuntime::connect_from_query())
    }
}

/// Build a WebSocket runtime and start a local AgentEnvelope bridge on native platforms.
///
/// For a spec-compliant MCP server, start an SSE server separately via
/// `mcp_sse_server::McpSseServer` (feature `mcp_sse`).
#[cfg(not(target_arch = "wasm32"))]
pub fn build_runtime_with_local_server() -> (
    Arc<dyn AgentRuntime>,
    Option<crate::agent_ws_server::AgentWsServerHandle>,
) {
    let server = if std::env::var("AGENT_WS_URL").is_ok() {
        None
    } else {
        match crate::agent_ws_server::AgentWsServer::spawn_default() {
            Ok(handle) => Some(handle),
            Err(err) => {
                log::warn!("failed to start local Agent WS server: {err}");
                None
            }
        }
    };

    let runtime = Arc::new(AgentWsRuntime::connect_from_env());
    (runtime, server)
}
