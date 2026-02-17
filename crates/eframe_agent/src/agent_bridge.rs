use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use parking_lot::Mutex;

use crate::runtime::{AgentCommand, AgentRuntime, AgentUpdate};

const BRIDGE_TICK: Duration = Duration::from_millis(20);

/// Fan-out bridge that forwards commands into an [`AgentRuntime`] and broadcasts updates.
pub struct AgentBridge {
    command_tx: mpsc::Sender<AgentCommand>,
    subscribers: Arc<Mutex<Vec<mpsc::Sender<AgentUpdate>>>>,
    running: Arc<AtomicBool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}

impl AgentBridge {
    /// Create a new bridge for `runtime` and start the background pump.
    pub fn new(runtime: Arc<dyn AgentRuntime>) -> Self {
        let (command_tx, command_rx) = mpsc::channel::<AgentCommand>();
        let subscribers = Arc::new(Mutex::new(Vec::new()));
        let running = Arc::new(AtomicBool::new(true));
        let running_thread = Arc::clone(&running);
        let runtime_thread = Arc::clone(&runtime);
        let subscribers_thread = Arc::clone(&subscribers);

        let worker = thread::Builder::new()
            .name("eframe_agent_bridge".to_string())
            .spawn(move || {
                let mut updates = Vec::new();
                while running_thread.load(Ordering::Relaxed) {
                    match command_rx.recv_timeout(BRIDGE_TICK) {
                        Ok(command) => {
                            runtime_thread.submit_command(command);
                            while let Ok(command) = command_rx.try_recv() {
                                runtime_thread.submit_command(command);
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => {}
                    }

                    runtime_thread.poll_updates(&mut updates);
                    if updates.is_empty() {
                        continue;
                    }

                    let mut subs = subscribers_thread.lock();
                    subs.retain(|tx: &mpsc::Sender<AgentUpdate>| {
                        for update in &updates {
                            if tx.send(update.clone()).is_err() {
                                return false;
                            }
                        }
                        true
                    });
                    updates.clear();
                }
            })
            .expect("failed to spawn agent bridge thread");

        Self {
            command_tx,
            subscribers,
            running,
            worker: Mutex::new(Some(worker)),
        }
    }

    /// Subscribe to broadcast updates.
    pub fn subscribe(&self) -> mpsc::Receiver<AgentUpdate> {
        let (tx, rx) = mpsc::channel();
        self.subscribers.lock().push(tx);
        rx
    }

    /// Submit a command to the underlying runtime.
    pub fn submit_command(&self, command: AgentCommand) {
        let _ = self.command_tx.send(command);
    }

    /// Broadcast an update directly to subscribers.
    pub fn broadcast_update(&self, update: AgentUpdate) {
        let mut subs = self.subscribers.lock();
        subs.retain(|tx: &mpsc::Sender<AgentUpdate>| tx.send(update.clone()).is_ok());
    }

    /// Stop the bridge worker thread.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.worker.lock().take() {
            let _ = handle.join();
        }
    }
}

impl Drop for AgentBridge {
    fn drop(&mut self) {
        self.shutdown();
    }
}
