use std::collections::hash_map::DefaultHasher;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::{Hash, Hasher},
    sync::Arc,
    time::{Duration, Instant},
};

use egui::accesskit::{Action, ActionRequest, Node, NodeId, TreeUpdate};
use parking_lot::{Mutex, RwLock};
use serde_json::Value;
use std::sync::mpsc;

use egui::plugin::Plugin;
use egui::{Event, FullOutput, Key, Modifiers, RawInput};

use crate::{
    jsonl::{JsonlDriver, KeyModifier, Target, TargetBy},
    state::AgentState,
};

const SNAPSHOT_WAIT_MS: u64 = 500;
const SNAPSHOT_POLL_MS: u64 = 10;

/// Live automation bridge for the running egui UI.
#[derive(Clone)]
pub struct AutomationBridge {
    inner: Arc<AutomationInner>,
}

struct AutomationInner {
    action_tx: mpsc::Sender<AutomationAction>,
    action_rx: Mutex<mpsc::Receiver<AutomationAction>>,
    screenshot_requests: Mutex<VecDeque<u64>>,
    screenshot_waiters: Mutex<HashMap<u64, mpsc::Sender<AutomationScreenshot>>>,
    next_screenshot_id: AtomicU64,
    snapshot: RwLock<AutomationSnapshot>,
}

#[derive(Debug, Clone)]
enum AutomationAction {
    Click { target: NodeId },
    Focus { target: NodeId },
    TypeText { target: NodeId, text: String },
    PressKey { key: Key, modifiers: Modifiers },
}

#[derive(Default)]
struct AutomationSnapshot {
    root: Option<NodeId>,
    focus: Option<NodeId>,
    nodes: HashMap<NodeId, Node>,
    last_update: Option<Instant>,
    state: Option<AgentState>,
}

struct AutomationOutputPlugin {
    automation: AutomationBridge,
}

impl AutomationOutputPlugin {
    fn new(automation: AutomationBridge) -> Self {
        Self { automation }
    }
}

impl Plugin for AutomationOutputPlugin {
    fn debug_name(&self) -> &'static str {
        "automation_output"
    }

    fn output_hook(&mut self, output: &mut FullOutput) {
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            self.automation.apply_accesskit_update(update);
        }
    }
}

/// Errors returned by the automation bridge/driver.
#[derive(Debug)]
pub enum AutomationError {
    /// No AccessKit snapshot is available yet.
    NoSnapshot,

    /// The requested target could not be found.
    TargetNotFound {
        /// The missing target selector.
        target: String,
    },

    /// A key name could not be parsed.
    InvalidKey {
        /// The invalid key name.
        key: String,
    },

    /// The action queue was closed.
    SendFailed,

    /// Failed to serialize a state snapshot.
    StateSnapshot(String),

    /// Timed out waiting for a screenshot.
    ScreenshotTimeout {
        /// Timeout in milliseconds.
        timeout_ms: u64,
    },

    /// The screenshot response channel closed unexpectedly.
    ScreenshotChannelClosed,
}

impl std::fmt::Display for AutomationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutomationError::NoSnapshot => write!(f, "no accesskit snapshot available"),
            AutomationError::TargetNotFound { target } => {
                write!(f, "target not found: {target}")
            }
            AutomationError::InvalidKey { key } => write!(f, "invalid key: {key}"),
            AutomationError::SendFailed => write!(f, "automation action queue is closed"),
            AutomationError::StateSnapshot(message) => write!(f, "state snapshot error: {message}"),
            AutomationError::ScreenshotTimeout { timeout_ms } => {
                write!(f, "screenshot not received within {timeout_ms}ms")
            }
            AutomationError::ScreenshotChannelClosed => {
                write!(f, "screenshot channel closed")
            }
        }
    }
}

impl std::error::Error for AutomationError {}

/// Metadata returned for a captured screenshot.
#[derive(Clone, Debug)]
pub struct AutomationScreenshot {
    /// Screenshot width in pixels.
    pub width: usize,

    /// Screenshot height in pixels.
    pub height: usize,

    /// Hash of the RGBA pixel data for quick verification.
    pub hash: u64,
}

impl AutomationSnapshot {
    fn apply_update(&mut self, update: TreeUpdate) {
        if let Some(tree) = update.tree {
            self.root = Some(tree.root);
        }
        self.focus = Some(update.focus);
        for (id, node) in update.nodes {
            self.nodes.insert(id, node);
        }
        self.last_update = Some(Instant::now());
    }

    fn for_each_reachable<F: FnMut(NodeId, &Node) -> bool>(&self, mut f: F) -> Option<NodeId> {
        let Some(root) = self.root else {
            for (id, node) in &self.nodes {
                if f(*id, node) {
                    return Some(*id);
                }
            }
            return None;
        };

        let mut stack = vec![root];
        let mut visited = HashSet::new();
        while let Some(id) = stack.pop() {
            if !visited.insert(id) {
                continue;
            }
            let Some(node) = self.nodes.get(&id) else {
                continue;
            };
            if f(id, node) {
                return Some(id);
            }
            for child in node.children() {
                stack.push(*child);
            }
        }
        None
    }

    fn find_by_author_id(&self, value: &str) -> Option<NodeId> {
        self.for_each_reachable(|_id, node| !node.is_hidden() && node.author_id() == Some(value))
    }

    fn find_by_label(&self, value: &str) -> Option<NodeId> {
        self.for_each_reachable(|_id, node| !node.is_hidden() && node.label() == Some(value))
    }

    fn find_by_text(&self, value: &str) -> Option<NodeId> {
        self.for_each_reachable(|_id, node| {
            if node.is_hidden() {
                return false;
            }
            node.label() == Some(value) || node.value() == Some(value)
        })
    }

    fn find_by_role(&self, value: &str) -> Option<NodeId> {
        self.for_each_reachable(|_id, node| {
            if node.is_hidden() {
                return false;
            }
            let role_name = format!("{:?}", node.role());
            role_name.eq_ignore_ascii_case(value)
        })
    }

    fn any_text_contains(&self, value: &str) -> bool {
        let needle = value;
        let mut found = false;
        self.for_each_reachable(|_, node| {
            if node.is_hidden() {
                return false;
            }
            let label_match = node
                .label()
                .map(|text| text.contains(needle))
                .unwrap_or(false);
            let value_match = node
                .value()
                .map(|text| text.contains(needle))
                .unwrap_or(false);
            found = label_match || value_match;
            found
        });
        found
    }
}

impl AutomationBridge {
    /// Create a new automation bridge.
    pub fn new() -> Self {
        let (action_tx, action_rx) = mpsc::channel();
        Self {
            inner: Arc::new(AutomationInner {
                action_tx,
                action_rx: Mutex::new(action_rx),
                screenshot_requests: Mutex::new(VecDeque::new()),
                screenshot_waiters: Mutex::new(HashMap::new()),
                next_screenshot_id: AtomicU64::new(1),
                snapshot: RwLock::new(AutomationSnapshot::default()),
            }),
        }
    }

    pub(crate) fn register_accesskit_plugin(&self, ctx: &egui::Context) {
        ctx.add_plugin(AutomationOutputPlugin::new(self.clone()));
    }

    /// Queue a click action for the given target.
    pub fn click(&self, target: &Target) -> Result<(), AutomationError> {
        let target = self.resolve_target(target)?;
        self.enqueue_action(AutomationAction::Click { target })
    }

    /// Queue a focus action for the given target.
    pub fn focus(&self, target: &Target) -> Result<(), AutomationError> {
        let target = self.resolve_target(target)?;
        self.enqueue_action(AutomationAction::Focus { target })
    }

    /// Queue text input for the given target.
    pub fn type_text(&self, target: &Target, text: &str) -> Result<(), AutomationError> {
        let target = self.resolve_target(target)?;
        self.enqueue_action(AutomationAction::TypeText {
            target,
            text: text.to_string(),
        })
    }

    /// Queue a key press with modifiers.
    pub fn press_key(&self, key: &str, modifiers: &[KeyModifier]) -> Result<(), AutomationError> {
        let Some(parsed) = parse_key_name(key) else {
            return Err(AutomationError::InvalidKey { key: key.into() });
        };
        let modifiers = modifiers_from_jsonl(modifiers);
        self.enqueue_action(AutomationAction::PressKey {
            key: parsed,
            modifiers,
        })
    }

    /// Check if a target exists in the latest UI snapshot.
    pub fn ui_exists(&self, target: &Target) -> Result<bool, AutomationError> {
        let snapshot = self.inner.snapshot.read();
        if snapshot.nodes.is_empty() {
            return Ok(false);
        }
        Ok(match target.by {
            TargetBy::Id => snapshot.find_by_author_id(&target.value).is_some(),
            TargetBy::Label => snapshot.find_by_label(&target.value).is_some(),
            TargetBy::Text => snapshot.find_by_text(&target.value).is_some(),
            TargetBy::Role => snapshot.find_by_role(&target.value).is_some(),
        })
    }

    /// Check if any visible text contains the provided value.
    pub fn ui_text_contains(&self, value: &str) -> Result<bool, AutomationError> {
        let snapshot = self.inner.snapshot.read();
        if snapshot.nodes.is_empty() {
            return Ok(false);
        }
        Ok(snapshot.any_text_contains(value))
    }

    /// Return the latest state snapshot as JSON, if available.
    pub fn state_snapshot(&self) -> Result<Option<Value>, AutomationError> {
        let state = self.inner.snapshot.read().state.clone();
        let Some(state) = state else {
            return Ok(None);
        };
        serde_json::to_value(&state)
            .map(Some)
            .map_err(|err| AutomationError::StateSnapshot(err.to_string()))
    }

    /// Create a JSONL driver backed by this automation bridge.
    pub fn driver(&self) -> AutomationDriver {
        AutomationDriver {
            bridge: self.clone(),
        }
    }

    pub(crate) fn inject_raw_input(&self, raw_input: &mut RawInput) -> usize {
        let actions = self.drain_actions();
        if actions.is_empty() {
            return 0;
        }

        let mut injected = 0;
        for action in actions {
            match action {
                AutomationAction::Click { target } => {
                    raw_input
                        .events
                        .push(Event::AccessKitActionRequest(ActionRequest {
                            action: Action::Click,
                            target,
                            data: None,
                        }));
                    injected += 1;
                }
                AutomationAction::Focus { target } => {
                    raw_input
                        .events
                        .push(Event::AccessKitActionRequest(ActionRequest {
                            action: Action::Focus,
                            target,
                            data: None,
                        }));
                    injected += 1;
                }
                AutomationAction::TypeText { target, text } => {
                    raw_input
                        .events
                        .push(Event::AccessKitActionRequest(ActionRequest {
                            action: Action::Focus,
                            target,
                            data: None,
                        }));
                    raw_input.events.push(Event::Text(text));
                    injected += 2;
                }
                AutomationAction::PressKey { key, modifiers } => {
                    raw_input.events.push(Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        modifiers,
                        repeat: false,
                    });
                    raw_input.events.push(Event::Key {
                        key,
                        physical_key: None,
                        pressed: false,
                        modifiers,
                        repeat: false,
                    });
                    injected += 2;
                }
            }
        }
        injected
    }

    pub(crate) fn capture_screenshot_events(&self, raw_input: &RawInput) {
        for event in &raw_input.events {
            let Event::Screenshot {
                user_data, image, ..
            } = event
            else {
                continue;
            };
            let id = user_data
                .data
                .as_ref()
                .and_then(|data| data.as_ref().downcast_ref::<u64>().copied());
            let Some(id) = id else { continue };
            let screenshot = AutomationScreenshot {
                width: image.size[0],
                height: image.size[1],
                hash: hash_color_image(image),
            };
            if let Some(sender) = self.inner.screenshot_waiters.lock().remove(&id) {
                let _ = sender.send(screenshot);
            }
        }
    }

    pub(crate) fn drain_screenshot_requests(&self) -> Vec<u64> {
        let mut pending = self.inner.screenshot_requests.lock();
        pending.drain(..).collect()
    }

    /// Request a screenshot and wait for it to be returned.
    pub fn request_screenshot(
        &self,
        timeout: Duration,
    ) -> Result<AutomationScreenshot, AutomationError> {
        let (tx, rx) = mpsc::channel();
        let id = self
            .inner
            .next_screenshot_id
            .fetch_add(1, Ordering::Relaxed);
        self.inner.screenshot_waiters.lock().insert(id, tx);
        self.inner.screenshot_requests.lock().push_back(id);
        match rx.recv_timeout(timeout) {
            Ok(screenshot) => Ok(screenshot),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.inner.screenshot_waiters.lock().remove(&id);
                Err(AutomationError::ScreenshotTimeout {
                    timeout_ms: timeout.as_millis() as u64,
                })
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(AutomationError::ScreenshotChannelClosed)
            }
        }
    }

    pub(crate) fn update_from_ui(&self, ui: &egui::Ui, state: &AgentState) {
        let update = ui.output(|output| output.accesskit_update.clone());
        let mut snapshot = self.inner.snapshot.write();
        if let Some(update) = update {
            snapshot.apply_update(update);
        }
        snapshot.state = Some(state.clone());
    }

    fn apply_accesskit_update(&self, update: TreeUpdate) {
        let mut snapshot = self.inner.snapshot.write();
        snapshot.apply_update(update);
    }

    fn enqueue_action(&self, action: AutomationAction) -> Result<(), AutomationError> {
        self.inner
            .action_tx
            .send(action)
            .map_err(|_| AutomationError::SendFailed)
    }

    fn drain_actions(&self) -> Vec<AutomationAction> {
        let receiver = self.inner.action_rx.lock();
        let mut actions = Vec::new();
        while let Ok(action) = receiver.try_recv() {
            actions.push(action);
        }
        actions
    }

    fn wait_for_snapshot(&self) -> bool {
        let deadline = Instant::now() + Duration::from_millis(SNAPSHOT_WAIT_MS);
        loop {
            let has_snapshot = {
                let snapshot = self.inner.snapshot.read();
                !snapshot.nodes.is_empty()
            };
            if has_snapshot {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(SNAPSHOT_POLL_MS));
        }
    }

    fn resolve_target(&self, target: &Target) -> Result<NodeId, AutomationError> {
        if !self.wait_for_snapshot() {
            return Err(AutomationError::NoSnapshot);
        }
        let snapshot = self.inner.snapshot.read();

        let resolved = match target.by {
            TargetBy::Id => snapshot.find_by_author_id(&target.value),
            TargetBy::Label => snapshot.find_by_label(&target.value),
            TargetBy::Text => snapshot.find_by_text(&target.value),
            TargetBy::Role => snapshot.find_by_role(&target.value),
        };

        resolved.ok_or_else(|| AutomationError::TargetNotFound {
            target: format!("{target:?}"),
        })
    }
}

impl Default for AutomationBridge {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_color_image(image: &egui::ColorImage) -> u64 {
    let mut hasher = DefaultHasher::new();
    image.size.hash(&mut hasher);
    for pixel in &image.pixels {
        pixel.to_array().hash(&mut hasher);
    }
    hasher.finish()
}

/// JSONL driver backed by the live automation bridge.
pub struct AutomationDriver {
    bridge: AutomationBridge,
}

impl JsonlDriver for AutomationDriver {
    type Error = AutomationError;

    fn click(&mut self, target: &Target) -> Result<(), Self::Error> {
        self.bridge.click(target)
    }

    fn focus(&mut self, target: &Target) -> Result<(), Self::Error> {
        self.bridge.focus(target)
    }

    fn type_text(&mut self, target: &Target, text: &str) -> Result<(), Self::Error> {
        self.bridge.type_text(target, text)
    }

    fn press_key(&mut self, key: &str, modifiers: &[KeyModifier]) -> Result<(), Self::Error> {
        self.bridge.press_key(key, modifiers)
    }

    fn run_steps(&mut self, steps: u32, dt: Option<f32>) -> Result<(), Self::Error> {
        let dt = dt.unwrap_or(0.016);
        let total_ms = dt * steps as f32 * 1_000.0;
        if total_ms > 0.0 {
            std::thread::sleep(Duration::from_millis(total_ms as u64));
        }
        Ok(())
    }

    fn sleep_ms(&mut self, ms: u64) -> Result<(), Self::Error> {
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms));
        }
        Ok(())
    }

    fn ui_exists(&mut self, target: &Target) -> Result<bool, Self::Error> {
        self.bridge.ui_exists(target)
    }

    fn ui_text_contains(&mut self, value: &str) -> Result<bool, Self::Error> {
        self.bridge.ui_text_contains(value)
    }

    fn state_snapshot(&mut self) -> Result<Option<Value>, Self::Error> {
        self.bridge.state_snapshot()
    }
}

fn parse_key_name(name: &str) -> Option<Key> {
    Key::from_name(name)
        .or_else(|| Key::from_name(&name.to_ascii_uppercase()))
        .or_else(|| Key::from_name(&name.to_ascii_lowercase()))
        .or_else(|| {
            let mut chars = name.chars();
            let first = chars.next()?;
            let mut normalized = String::new();
            normalized.push(first.to_ascii_uppercase());
            normalized.push_str(chars.as_str());
            Key::from_name(&normalized)
        })
}

fn modifiers_from_jsonl(modifiers: &[KeyModifier]) -> Modifiers {
    let mut output = Modifiers::default();
    for modifier in modifiers {
        match modifier {
            KeyModifier::Ctrl => output.ctrl = true,
            KeyModifier::Shift => output.shift = true,
            KeyModifier::Alt => output.alt = true,
            KeyModifier::Command | KeyModifier::MacCmd => {
                output.mac_cmd = true;
            }
        }
    }
    output.command = output.ctrl || output.mac_cmd;
    output
}
