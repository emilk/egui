//! Programmatic automation channel for a running eframe application.
//!
//! [`AutomationHandle`] lets code running outside the winit event loop
//! (typically a test thread) feed synthetic [`egui::Event`]s into the next
//! frame and observe the AccessKit [`accesskit::TreeUpdate`]s produced by
//! egui. This is the building block used by `egui_kittest`'s remote harness
//! to drive a real eframe app the same way it drives the in-process test
//! harness.
//!
//! Enable it by attaching an [`AutomationHandle`] to
//! [`crate::NativeOptions::automation`] before calling [`crate::run_native`].

use std::sync::{Arc, Condvar, Mutex, OnceLock};

use egui::accesskit;

/// Shared automation channel between a running eframe app and an external
/// controller (e.g. a test thread).
///
/// Construct one with [`AutomationHandle::new`], hand a `clone` of the `Arc`
/// to [`crate::NativeOptions::automation`], and keep another `Arc` in the
/// controller for [`AutomationHandle::push_event`] /
/// [`AutomationHandle::wait_for_tree_update`].
pub struct AutomationHandle {
    /// Events pushed by the controller. Drained into [`egui::RawInput`] each
    /// frame by `egui_winit::State::take_egui_input`.
    pub(crate) events: Arc<Mutex<Vec<egui::Event>>>,

    /// Queue of AccessKit updates emitted by egui this frame and earlier,
    /// in arrival order. Tree updates are *incremental* — consumers must
    /// apply them in order to a `kittest::State` (or equivalent) to track
    /// the current accessibility tree.
    tree_updates: Mutex<Vec<accesskit::TreeUpdate>>,

    /// Signalled whenever a new tree update lands in `tree_updates`.
    tree_update_signal: Condvar,

    /// The egui context of the running app. Populated by eframe on the first
    /// frame. The controller uses this to request repaints between input
    /// pushes.
    ctx: OnceLock<egui::Context>,
}

impl AutomationHandle {
    /// Create a fresh automation handle. Wrap in an `Arc` and pass via
    /// [`crate::NativeOptions::automation`].
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            tree_updates: Mutex::new(Vec::new()),
            tree_update_signal: Condvar::new(),
            ctx: OnceLock::new(),
        }
    }

    /// Queue a synthetic event to be delivered on the next frame, then
    /// request a repaint so the running app picks it up.
    pub fn push_event(&self, event: egui::Event) {
        self.events
            .lock()
            .expect("events lock poisoned")
            .push(event);
        if let Some(ctx) = self.ctx.get() {
            ctx.request_repaint();
        }
    }

    /// Queue multiple events in a single batch (cheaper than calling
    /// [`Self::push_event`] in a loop).
    pub fn push_events(&self, events: impl IntoIterator<Item = egui::Event>) {
        let mut queue = self.events.lock().expect("events lock poisoned");
        queue.extend(events);
        drop(queue);
        if let Some(ctx) = self.ctx.get() {
            ctx.request_repaint();
        }
    }

    /// Drain and return all AccessKit tree updates that have been produced
    /// since the last drain, in arrival order. Returns an empty `Vec` if
    /// none are pending.
    pub fn drain_tree_updates(&self) -> Vec<accesskit::TreeUpdate> {
        std::mem::take(&mut *self.tree_updates.lock().expect("tree lock poisoned"))
    }

    /// Block the calling thread until at least one tree update is available,
    /// then drain and return all queued updates in order.
    ///
    /// Returns `None` if `timeout` elapses with no update.
    pub fn wait_for_tree_update(
        &self,
        timeout: std::time::Duration,
    ) -> Option<Vec<accesskit::TreeUpdate>> {
        let mut updates = self.tree_updates.lock().expect("tree lock poisoned");
        if !updates.is_empty() {
            return Some(std::mem::take(&mut *updates));
        }
        let (mut updates, wait) = self
            .tree_update_signal
            .wait_timeout(updates, timeout)
            .expect("tree lock poisoned");
        if wait.timed_out() && updates.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut *updates))
        }
    }

    /// Returns the running app's [`egui::Context`], if the first frame has
    /// already run. Use this to call [`egui::Context::request_repaint`] or
    /// inspect input/output state directly.
    pub fn ctx(&self) -> Option<egui::Context> {
        self.ctx.get().cloned()
    }

    /// Block until the eframe app has rendered its first frame and the
    /// [`egui::Context`] is available. Returns `None` on timeout.
    pub fn wait_for_ctx(&self, timeout: std::time::Duration) -> Option<egui::Context> {
        let deadline = std::time::Instant::now() + timeout;
        // The context is populated alongside the first tree update, so we
        // can piggyback on the same condvar.
        let mut updates = self.tree_updates.lock().expect("tree lock poisoned");
        while self.ctx.get().is_none() {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return None;
            }
            let (next, wait) = self
                .tree_update_signal
                .wait_timeout(updates, remaining)
                .expect("tree lock poisoned");
            updates = next;
            if wait.timed_out() && self.ctx.get().is_none() {
                return None;
            }
        }
        self.ctx.get().cloned()
    }

    /// Wire this handle to an `egui_winit::State`: share the event queue so
    /// pushed events reach the next frame's [`egui::RawInput`], install an
    /// AccessKit observer that forwards every tree update back here, and
    /// attach the running [`egui::Context`] so the controller can request
    /// repaints.
    pub(crate) fn install(
        self: &Arc<Self>,
        egui_winit: &mut egui_winit::State,
        egui_ctx: &egui::Context,
    ) {
        egui_winit.set_external_event_sink(Arc::clone(&self.events));
        let observer = Arc::clone(self);
        egui_winit.set_accesskit_observer(Some(Box::new(move |update| {
            observer._push_tree_update(update.clone());
        })));
        let _ = self.ctx.set(egui_ctx.clone());
        // Wake any thread blocked in `wait_for_ctx`.
        self.tree_update_signal.notify_all();
    }

    /// Called by eframe internals on every frame that produces an AccessKit
    /// update. Not part of the public API.
    #[doc(hidden)]
    pub fn _push_tree_update(&self, update: accesskit::TreeUpdate) {
        self.tree_updates
            .lock()
            .expect("tree lock poisoned")
            .push(update);
        self.tree_update_signal.notify_all();
    }
}

impl Default for AutomationHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AutomationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutomationHandle")
            .field(
                "queued_events",
                &self.events.lock().map(|q| q.len()).unwrap_or(0),
            )
            .field(
                "queued_tree_updates",
                &self.tree_updates.lock().map(|q| q.len()).unwrap_or(0),
            )
            .field("ctx_attached", &self.ctx.get().is_some())
            .finish()
    }
}
