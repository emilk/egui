use ahash::HashMap;

use crate::{Context, Id, Plugin, Ui};

type CleanupClosure = Box<dyn FnOnce(&Context) + Send + Sync + 'static>;

/// Cleans up state in [`crate::Memory`] once the widget owning it is no longer shown.
///
/// egui doesn't know when a widget goes away, so any state a widget stores in [`crate::Memory`]
/// (e.g. the open state of a collapsing header) is kept until the app exits.
/// That is usually what you want, but not always.
///
/// With this plugin a widget can register a cleanup closure, keyed by its [`Id`], on each pass it is shown.
/// The closure is called at the end of the first pass where it is *not* registered again,
/// i.e. when the widget stops being shown.
///
/// This is a built-in plugin, registered by default when creating a [`Context`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// let id = ui.make_persistent_id("temporary_state");
/// let is_open: bool = ui.data_mut(|d| *d.get_temp_mut_or_default(id));
///
/// // Forget the state once this widget stops being shown:
/// ui.ctx()
///     .plugin::<egui::MemoryGarbageCollector>()
///     .lock()
///     .add(id, move |ctx| {
///         ctx.data_mut(|d| d.remove::<bool>(id));
///     });
/// # });
/// ```
#[derive(Default)]
pub struct MemoryGarbageCollector {
    /// Everything in here is moved to `pending_cleanup` at the end of the pass.
    seen_this_pass: HashMap<Id, CleanupClosure>,

    /// Everything still in here at the end of a pass was not registered again during that pass,
    /// so its cleanup closure is called.
    pending_cleanup: HashMap<Id, CleanupClosure>,
}

impl MemoryGarbageCollector {
    /// Register a cleanup closure for `id`.
    ///
    /// Call this on every pass the widget is shown.
    /// The closure is called at the end of the first pass where this is *not* called for `id`.
    pub fn add(&mut self, id: Id, cleanup: impl FnOnce(&Context) + Send + Sync + 'static) {
        self.seen_this_pass.insert(id, Box::new(cleanup));
        self.pending_cleanup.remove(&id);
    }
}

impl Plugin for MemoryGarbageCollector {
    fn debug_name(&self) -> &'static str {
        "MemoryGarbageCollector"
    }

    fn on_end_pass(&mut self, ui: &mut Ui) {
        let cleanup = core::mem::take(&mut self.pending_cleanup);
        core::mem::swap(&mut self.seen_this_pass, &mut self.pending_cleanup);

        // The cleanups are independent, so order does not matter.
        #[expect(clippy::iter_over_hash_type)]
        for (_, cleanup) in cleanup {
            cleanup(ui.ctx());
        }
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    #[test]
    fn cleanup_runs_when_id_is_no_longer_added() {
        let ctx = Context::default();
        ctx.set_fonts(crate::FontDefinitions::empty());
        let id = Id::new("test");
        let num_cleanups = Arc::new(AtomicUsize::new(0));

        let run_pass = |add: bool| {
            let output = ctx.run_ui(Default::default(), |ui| {
                if add {
                    let num_cleanups = Arc::clone(&num_cleanups);
                    ui.ctx()
                        .plugin::<MemoryGarbageCollector>()
                        .lock()
                        .add(id, move |_ctx| {
                            num_cleanups.fetch_add(1, Ordering::SeqCst);
                        });
                }
            });
            output.drop_without_applying_deltas();
        };

        run_pass(true);
        run_pass(true);
        assert_eq!(num_cleanups.load(Ordering::SeqCst), 0);

        run_pass(false);
        assert_eq!(num_cleanups.load(Ordering::SeqCst), 1);

        run_pass(false);
        assert_eq!(num_cleanups.load(Ordering::SeqCst), 1);
    }
}
