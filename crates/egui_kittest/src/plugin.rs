//! Plugin system for observing and extending the [`crate::Harness`] test lifecycle.
//!
//! Implement [`Plugin`] to hook into harness events: frame steps, run loops, events,
//! renders, snapshots, and final pass/fail. Register plugins via
//! [`crate::HarnessBuilder::with_plugin`] or [`crate::Harness::add_plugin`].

use std::any::Any;

use crate::{ExceededMaxStepsError, Harness};

/// A plugin observes the test-harness lifecycle and can drive additional frames.
///
/// All methods default to no-ops; implement only the ones you need.
///
/// State-agnostic plugins should impl for all `State` so they're reusable across harnesses:
/// ```
/// use egui_kittest::{Harness, Plugin};
///
/// struct MyPlugin;
///
/// impl<S> Plugin<S> for MyPlugin {
///     fn after_step(&mut self, _harness: &mut Harness<'_, S>) {
///         // ...
///     }
/// }
/// ```
///
/// # Downcasting
///
/// [`Any`] is a supertrait, so [`Harness::plugin`] / [`Harness::plugin_mut`] /
/// [`Harness::take_plugin`] downcast registered plugins back to their concrete type via
/// trait upcasting. No boilerplate needed on your end.
///
/// # Re-entrancy
///
/// Plugin hooks receive `&mut Harness`. Calling [`Harness::step`] / [`Harness::run`] /
/// etc. from inside a hook will recurse infinitely through your own `after_step`. If
/// a plugin needs to advance the harness from inside a hook — e.g. an inspector that
/// blocks on user input — use [`Harness::step_no_side_effects`] instead.
#[expect(unused_variables, reason = "default no-op impls")]
pub trait Plugin<State = ()>: Send + Any {
    /// Called once at the start of every `run()` / `try_run()` / `try_run_realtime()` /
    /// `run_ok()` invocation, before the first step.
    fn before_run(&mut self, harness: &mut Harness<'_, State>) {}

    /// Called once after the outer run loop exits (successful completion or
    /// [`ExceededMaxStepsError`]).
    fn after_run(
        &mut self,
        harness: &mut Harness<'_, State>,
        result: Result<u64, &ExceededMaxStepsError>,
    ) {
    }

    /// Called immediately before each single-frame step (per-frame, not per public call).
    fn before_step(&mut self, harness: &mut Harness<'_, State>) {}

    /// Called immediately after each single-frame step.
    fn after_step(&mut self, harness: &mut Harness<'_, State>) {}

    /// Called after each single-frame step with the AccessKit tree update egui produced
    /// for that frame, before it's applied to the internal kittest state.
    ///
    /// Plugins that need the tree (e.g. to stream it to an external debugger) should
    /// clone it here — the harness no longer retains it after this hook returns.
    fn on_accesskit_update(
        &mut self,
        harness: &mut Harness<'_, State>,
        tree: &egui::accesskit::TreeUpdate,
    ) {
    }

    /// Called after a queued event has been pushed into the harness input, before the
    /// frame runs that consumes it.
    fn on_event(&mut self, harness: &mut Harness<'_, State>, event: &egui::Event) {}

    /// Called from inside [`Harness::render`] after the image is produced. Lets a plugin
    /// observe every rendered frame without triggering a second render pass.
    #[cfg(any(feature = "wgpu", feature = "snapshot"))]
    fn on_render(&mut self, harness: &mut Harness<'_, State>, image: &image::RgbaImage) {}

    /// Called from [`Harness::try_snapshot`] / [`Harness::try_snapshot_options`] after
    /// the comparison has run, before the result is handed back to the caller. The
    /// `image` is the frame that was compared against the stored snapshot.
    #[cfg(feature = "snapshot")]
    fn on_snapshot(
        &mut self,
        harness: &mut Harness<'_, State>,
        name: &str,
        image: &image::RgbaImage,
        result: &crate::SnapshotResult,
    ) {
    }

    /// Called exactly once, from [`Harness::drop`], after the harness has finalized its
    /// snapshot results. `result` is [`TestResult::Pass`] unless a panic is in progress
    /// on this thread, in which case it's [`TestResult::Fail`].
    ///
    /// The `message` and `location` fields of `Fail` are only populated if the user has
    /// called [`install_panic_hook`]. Without the hook, the variant still flips to
    /// `Fail` but both fields are `None`.
    fn on_test_result(&mut self, harness: &mut Harness<'_, State>, result: TestResult<'_>) {}
}

/// Location of a panic — a `std::panic::Location` stripped of its borrow so it can be
/// stored in a thread-local and handed to plugins.
#[derive(Debug, Clone)]
pub struct PanicLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// Outcome of a test, as seen by [`Plugin::on_test_result`].
#[derive(Debug)]
pub enum TestResult<'a> {
    /// No panic in progress on this thread when `on_test_result` fired.
    Pass,

    /// A panic is in progress on this thread.
    ///
    /// `message` and `location` are populated only if [`install_panic_hook`] has been
    /// called (once, process-wide) before the panic occurred.
    Fail {
        message: Option<&'a str>,
        location: Option<&'a PanicLocation>,
    },
}

// ------------------------------------------------------------------------------------------------
// Opt-in panic hook for capturing the panic message + location so plugins can report them.
//
// Installing a `std::panic::set_hook` from library code is a process-wide side effect, so we
// do NOT install it automatically. Users opt in once (e.g. from a test main or `#[ctor]`).

use std::cell::RefCell;
use std::sync::OnceLock;

thread_local! {
    static LAST_PANIC: RefCell<Option<PanicRecord>> = const { RefCell::new(None) };
}

struct PanicRecord {
    message: Option<String>,
    location: Option<PanicLocation>,
}

static INSTALLED: OnceLock<()> = OnceLock::new();

/// Install a `std::panic::set_hook` that captures each panic's message and location into
/// a thread-local, which [`Plugin::on_test_result`] then reads into its `Fail` variant.
///
/// Process-wide and idempotent (subsequent calls are no-ops). Chains to whatever hook was
/// previously installed, so existing output is preserved.
pub fn install_panic_hook() {
    INSTALLED.get_or_init(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let message = info
                .payload()
                .downcast_ref::<&'static str>()
                .map(|s| (*s).to_owned())
                .or_else(|| info.payload().downcast_ref::<String>().cloned());
            let location = info.location().map(|loc| PanicLocation {
                file: loc.file().to_owned(),
                line: loc.line(),
                column: loc.column(),
            });
            LAST_PANIC.with(|slot| {
                *slot.borrow_mut() = Some(PanicRecord { message, location });
            });
            prev(info);
        }));
    });
}

/// Called from [`Harness::drop`] when `std::thread::panicking()` is true. Builds a
/// [`TestResult::Fail`] borrowing from the thread-local panic record, invokes `f` with
/// it, then restores the record.
///
/// We have to invoke via callback (rather than returning the `Fail`) because the borrows
/// live inside the thread-local's `RefCell`.
pub(crate) fn with_fail_test_result<R>(f: impl FnOnce(TestResult<'_>) -> R) -> R {
    LAST_PANIC.with(|slot| {
        let borrow = slot.borrow();
        let (message, location) = match borrow.as_ref() {
            Some(rec) => (rec.message.as_deref(), rec.location.as_ref()),
            None => (None, None),
        };
        f(TestResult::Fail { message, location })
    })
}
