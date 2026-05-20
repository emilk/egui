//! Drive a real eframe app through [`eframe::AutomationHandle`].
//!
//! Where [`crate::Harness`] owns its own [`egui::Context`] and runs the
//! frame loop itself, [`AutomationHarness`] talks to an eframe app running on
//! another thread (typically the main thread) via an automation channel.
//! The query API is the same — `harness.get_by_label("Save").click()` etc.
//! — but every event is sent to the live app and every observation comes
//! from real AccessKit tree updates the app produced.
//!
//! See `examples/remote_kittest/` for an end-to-end usage.

use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::AutomationHandle;
use egui::mutex::Mutex;
use kittest::Queryable;

use crate::node::{EventQueue, EventType, Node};

/// Default deadline used to wait for tree updates produced by the remote
/// app. Frames are typically delivered within a few milliseconds; the
/// generous deadline tolerates the OS scheduler and the first-frame setup.
const DEFAULT_FRAME_TIMEOUT: Duration = Duration::from_secs(2);

/// Idle interval used by [`AutomationHarness::run`] to decide that the remote
/// app has settled. If no new tree updates arrive within this window after
/// the last one, the run loop returns.
const DEFAULT_SETTLE_TIMEOUT: Duration = Duration::from_millis(100);

/// Errors returned by [`AutomationHarness::attach`] and friends.
#[derive(Debug)]
pub enum AutomationHarnessError {
    /// The remote app never produced its first AccessKit tree update within
    /// the timeout, so the harness has nothing to query.
    TimedOut,
}

impl std::fmt::Display for AutomationHarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TimedOut => write!(
                f,
                "Timed out waiting for the remote eframe app to produce its first AccessKit tree"
            ),
        }
    }
}

impl std::error::Error for AutomationHarnessError {}

/// Harness that drives a running eframe app through an
/// [`eframe::AutomationHandle`].
///
/// Build one by handing the same `Arc<AutomationHandle>` to both
/// [`eframe::NativeOptions::automation`] and [`AutomationHarness::attach`]:
///
/// ```no_run
/// # use std::sync::Arc;
/// # use eframe::AutomationHandle;
/// # use egui_kittest::{AutomationHarness, kittest::Queryable as _};
/// let automation = Arc::new(AutomationHandle::new());
/// let controller = Arc::clone(&automation);
///
/// // Spawn the app on another thread (or run it on the main thread and
/// // drive it from a worker — eframe must be on the main thread on macOS).
/// std::thread::spawn(move || {
///     let opts = eframe::NativeOptions {
///         automation: Some(controller),
///         ..Default::default()
///     };
///     eframe::run_native(
///         "my app",
///         opts,
///         Box::new(|_cc| Ok(Box::<MyApp>::default())),
///     ).unwrap();
/// });
///
/// let mut harness = AutomationHarness::attach(automation).unwrap();
/// harness.get_by_label("Click me").click();
/// harness.run();
/// assert!(harness.query_by_label("Clicked!").is_some());
/// # #[derive(Default)] struct MyApp;
/// # impl eframe::App for MyApp {
/// #     fn ui(&mut self, _: &mut eframe::egui::Ui, _: &mut eframe::Frame) {}
/// # }
/// ```
pub struct AutomationHarness {
    handle: Arc<AutomationHandle>,
    ctx: egui::Context,
    kittest: kittest::State,
    queue: EventQueue,
    frame_timeout: Duration,
    settle_timeout: Duration,
    max_steps: u32,
}

impl std::fmt::Debug for AutomationHarness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kittest.fmt(f)
    }
}

impl AutomationHarness {
    /// Attach to a running eframe app via the shared automation handle.
    ///
    /// Blocks until the app has produced its first AccessKit tree update or
    /// [`AutomationHarness::frame_timeout`] elapses.
    ///
    /// # Errors
    /// Returns [`AutomationHarnessError::TimedOut`] if the remote app does
    /// not produce its first AccessKit tree update within the default
    /// timeout.
    pub fn attach(handle: Arc<AutomationHandle>) -> Result<Self, AutomationHarnessError> {
        Self::attach_with_timeout(handle, DEFAULT_FRAME_TIMEOUT)
    }

    /// Like [`Self::attach`], but with a caller-supplied timeout for the
    /// initial wait.
    ///
    /// # Errors
    /// Returns [`AutomationHarnessError::TimedOut`] if the remote app does
    /// not produce its first AccessKit tree update within `timeout`.
    pub fn attach_with_timeout(
        handle: Arc<AutomationHandle>,
        timeout: Duration,
    ) -> Result<Self, AutomationHarnessError> {
        let deadline = Instant::now() + timeout;
        let ctx = handle
            .wait_for_ctx(timeout)
            .ok_or(AutomationHarnessError::TimedOut)?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        let updates = handle
            .wait_for_tree_update(remaining)
            .ok_or(AutomationHarnessError::TimedOut)?;
        let mut iter = updates.into_iter();
        let first = iter.next().ok_or(AutomationHarnessError::TimedOut)?;
        let mut kittest = kittest::State::new(first);
        for update in iter {
            kittest.update(update);
        }
        Ok(Self {
            handle,
            ctx,
            kittest,
            queue: Mutex::new(Vec::new()),
            frame_timeout: DEFAULT_FRAME_TIMEOUT,
            settle_timeout: DEFAULT_SETTLE_TIMEOUT,
            max_steps: 32,
        })
    }

    /// How long each [`Self::step`] waits for the remote app to deliver at
    /// least one tree update after the injected events. Default: 2s.
    #[inline]
    pub fn frame_timeout(&self) -> Duration {
        self.frame_timeout
    }

    /// Configure the per-step frame timeout (see [`Self::frame_timeout`]).
    #[inline]
    pub fn set_frame_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.frame_timeout = timeout;
        self
    }

    /// How long [`Self::run`] waits for further tree updates after the last
    /// one before declaring the app settled. Default: 100ms.
    #[inline]
    pub fn settle_timeout(&self) -> Duration {
        self.settle_timeout
    }

    /// Configure the settle timeout (see [`Self::settle_timeout`]).
    #[inline]
    pub fn set_settle_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.settle_timeout = timeout;
        self
    }

    /// Maximum frames [`Self::run`] will pull before giving up. Default: 32.
    #[inline]
    pub fn max_steps(&self) -> u32 {
        self.max_steps
    }

    /// Configure the maximum number of frames [`Self::run`] will pull.
    #[inline]
    pub fn set_max_steps(&mut self, max_steps: u32) -> &mut Self {
        self.max_steps = max_steps;
        self
    }

    /// Returns the remote app's [`egui::Context`].
    #[inline]
    pub fn ctx(&self) -> &egui::Context {
        &self.ctx
    }

    /// Returns the automation handle this harness drives.
    #[inline]
    pub fn handle(&self) -> &Arc<AutomationHandle> {
        &self.handle
    }

    /// Returns the underlying [`kittest::State`].
    #[inline]
    pub fn kittest_state(&self) -> &kittest::State {
        &self.kittest
    }

    /// The root node of the remote AccessKit tree. Returned [`Node`]s push
    /// events into this harness's queue, the same way they do for
    /// [`crate::Harness`].
    pub fn root(&self) -> Node<'_> {
        Node {
            accesskit_node: self.kittest.root(),
            queue: &self.queue,
        }
    }

    /// Drain queued events, deliver them to the remote app, and pull the
    /// resulting tree updates back into [`Self::kittest_state`].
    ///
    /// Returns the number of tree updates that landed during this step.
    pub fn step(&mut self) -> usize {
        let pending = std::mem::take(&mut *self.queue.lock());
        let mut events: Vec<egui::Event> = Vec::with_capacity(pending.len());
        for entry in pending {
            match entry {
                EventType::Event(e) => events.push(e),
                // Standalone modifier updates are a Harness-only concept
                // (they mutate `RawInput.modifiers` directly). The remote
                // egui-winit derives modifier state from real key events,
                // and every modifier-flavored event we ship (e.g. an
                // `Event::Key` with `modifiers: COMMAND`) carries the
                // modifiers in its own field, so we can safely drop these.
                EventType::Modifiers(_) => {}
            }
        }
        if !events.is_empty() {
            self.handle.push_events(events);
        }
        // Make sure the remote loop wakes up — push_events also calls this,
        // but call it explicitly so a no-event step still forces a frame.
        self.ctx.request_repaint();

        let Some(updates) = self.handle.wait_for_tree_update(self.frame_timeout) else {
            return 0;
        };
        let count = updates.len();
        for update in updates {
            self.kittest.update(update);
        }
        count
    }

    /// Run [`Self::step`] until the remote app stops emitting tree updates
    /// for [`Self::settle_timeout`], or [`Self::max_steps`] is exhausted.
    ///
    /// Returns the number of steps that ran.
    // TODO: find a way to wait for the app to settle even if it renders continuously
    pub fn run(&mut self) -> u32 {
        let mut steps = 0;
        // First step: deliver any queued events.
        let first_count = self.step();
        steps += 1;
        if first_count == 0 {
            return steps;
        }
        // Subsequent steps: drain follow-up frames the app produces on its
        // own (animations, async load completions, etc.). Stop when the
        // app stops talking back.
        while steps < self.max_steps {
            let Some(updates) = self.handle.wait_for_tree_update(self.settle_timeout) else {
                break;
            };
            if updates.is_empty() {
                break;
            }
            for update in updates {
                self.kittest.update(update);
            }
            steps += 1;
        }
        steps
    }
}

impl<'tree, 'node> Queryable<'tree, 'node, Node<'tree>> for AutomationHarness
where
    'node: 'tree,
{
    fn queryable_node(&'node self) -> Node<'tree> {
        self.root()
    }
}
