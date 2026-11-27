use std::sync::{Arc, Weak};

use egui::{ViewportId, ViewportIdMap};
use winit::window::Window;

/// The native window of each viewport, kept in the [`egui::Context`] so that
/// [`viewport_winit_window`] works from any viewport callback.
///
/// Holds weak references: it never keeps a closed window alive.
#[derive(Clone, Default)]
struct ViewportWindows(ViewportIdMap<Weak<Window>>);

fn id() -> egui::Id {
    egui::Id::unique("eframe::ViewportWindows")
}

/// The [`winit::window::Window`] of the viewport with the given id.
///
/// Unlike [`crate::Frame::winit_window`], this works for every viewport (root, deferred and immediate),
/// e.g. to parent a native dialog to the window it is opened from.
/// The window implements [`raw_window_handle::HasWindowHandle`] and [`raw_window_handle::HasDisplayHandle`].
///
/// `None` if eframe has no window for that viewport (not created yet, closed, or headless).
///
/// ```no_run
/// # fn example(ui: &mut egui::Ui) {
/// if let Some(window) = eframe::viewport_winit_window(ui.ctx(), ui.ctx().viewport_id()) {
///     // e.g. `rfd::FileDialog::new().set_parent(&*window)`
/// }
/// # }
/// ```
pub fn viewport_winit_window(ctx: &egui::Context, viewport_id: ViewportId) -> Option<Arc<Window>> {
    ctx.data(|d| {
        d.get_temp::<ViewportWindows>(id())?
            .0
            .get(&viewport_id)?
            .upgrade()
    })
}

/// Record the window eframe created for a viewport.
pub(crate) fn register(ctx: &egui::Context, viewport_id: ViewportId, window: &Arc<Window>) {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<ViewportWindows>(id())
            .0
            .insert(viewport_id, Arc::downgrade(window));
    });
}

/// Forget the windows of viewports that are gone.
pub(crate) fn retain(ctx: &egui::Context, mut keep: impl FnMut(&ViewportId) -> bool) {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<ViewportWindows>(id())
            .0
            .retain(|id, _| keep(id));
    });
}
