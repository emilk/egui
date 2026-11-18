//! Shared by the glow and wgpu backends: owned and modal dialog viewports.
//!
//! See [`egui::ViewportBuilder::with_owner`] and [`egui::ViewportBuilder::with_modal`].

use std::sync::Arc;

use egui::{OrderedViewportIdMap, ViewportBuilder, ViewportId, ViewportIdSet};
use winit::window::Window;

/// The owner of a viewport, if it has one: the builder's owner,
/// or else the parent viewport if the viewport is modal. Never the viewport itself.
pub(crate) fn resolve_owner(
    builder: &ViewportBuilder,
    this: ViewportId,
    parent: ViewportId,
) -> Option<ViewportId> {
    builder
        .owner
        .or_else(|| (builder.modal == Some(true)).then_some(parent))
        .filter(|owner| *owner != this)
}

/// Should the new window of a viewport be shown cloaked, and uncloaked after its first paint?
///
/// This is the case for owned windows on Windows, see [`egui_winit::create_window_with_owner`].
fn shows_cloaked(builder: &ViewportBuilder, owner: Option<&Window>) -> bool {
    cfg!(target_os = "windows") && owner.is_some() && builder.visible != Some(false)
}

/// Show a window that was just created, and return whether it is cloaked
/// (then call [`uncloak_after_first_paint`] after its first paint).
pub(crate) fn show_new_window(
    window: &Window,
    builder: &ViewportBuilder,
    owner: Option<&Window>,
) -> bool {
    let modal = builder.modal == Some(true);

    if modal {
        // Before the first show: Windows draws the title bar's icon, or not,
        // from the style the window has when it is first shown.
        egui_winit::set_dialog_chrome(window);
    }

    let cloaked = shows_cloaked(builder, owner);
    if cloaked {
        // Show it without a flash: cloaked now, uncloaked after the first paint.
        egui_winit::set_window_cloaked(window, true);
        window.set_visible(true);
        window.request_redraw();
    }

    if modal {
        // Again after showing it, while it is still cloaked: winit rewrites the
        // styles when it shows a window, after `ShowWindow`.
        egui_winit::set_dialog_chrome(window);
    }

    cloaked
}

/// Uncloak a window that [`show_new_window`] cloaked, once its first frame has been presented.
pub(crate) fn uncloak_after_first_paint(flag: &mut bool, window: &Window) {
    if core::mem::take(flag) {
        egui_winit::set_window_cloaked(window, false);
    }
}

/// Call after viewport commands changed the window: a command can make winit rewrite
/// the window's styles (e.g. show or hide it), which drops the dialog frame.
pub(crate) fn refresh_dialog_chrome(builder: &ViewportBuilder, window: &Window) {
    if builder.modal == Some(true) {
        egui_winit::set_dialog_chrome(window);
    }
}

/// Order the viewports whose windows are missing so that a viewport comes after its owner,
/// if the owner is also missing (an owned window needs its owner's window).
///
/// Viewports with a cyclic owner, or an owner that is not missing, keep their order.
pub(crate) fn in_creation_order(
    missing: Vec<ViewportId>,
    owner_of: impl Fn(ViewportId) -> Option<ViewportId>,
) -> Vec<ViewportId> {
    let mut ordered = Vec::with_capacity(missing.len());
    let mut rest = missing;
    while !rest.is_empty() {
        let (ready, waiting): (Vec<_>, Vec<_>) = rest
            .iter()
            .copied()
            .partition(|id| owner_of(*id).is_none_or(|owner| !rest.contains(&owner)));
        if ready.is_empty() {
            ordered.extend(waiting); // a cycle
            break;
        }
        ordered.extend(ready);
        rest = waiting;
    }
    ordered
}

/// What [`sync_modal_owners`] needs to know about a viewport.
pub(crate) struct ModalView<'a> {
    /// The viewport itself.
    pub id: ViewportId,

    /// The viewport that shows it.
    pub parent: ViewportId,

    /// Its builder.
    pub builder: &'a ViewportBuilder,

    /// Its window, if it has been created.
    pub window: Option<&'a Window>,

    /// The viewport whose window this window was created as an owned window of, if any.
    pub owned_by: Option<ViewportId>,
}

/// Take the windows that Windows destroys together with the windows of `owners`: the windows
/// owned by them, directly or through other owned windows.
/// Does nothing on other platforms, where a window outlives its owner.
///
/// `take_window` takes a viewport's window and drops the state that belongs to it,
/// so that a new window is created for it on the next window-creation pass.
///
/// Returns the taken windows in breadth-first order, so each one comes after its owner's.
/// Call [`sync_modal_owners`] before dropping any of them, then drop them in reverse order
/// (deepest first), before the windows of `owners`.
pub(crate) fn take_owned_windows<V>(
    viewports: &mut OrderedViewportIdMap<V>,
    owners: &[ViewportId],
    view: impl Fn(&V) -> ModalView<'_>,
    mut take_window: impl FnMut(&mut V) -> Option<Arc<Window>>,
) -> Vec<Arc<Window>> {
    if !cfg!(target_os = "windows") {
        return Vec::new();
    }

    // Breadth first from `owners`:
    let mut ids = owners.to_vec();
    let mut next = 0;
    while let Some(&owner) = ids.get(next) {
        next += 1;
        for viewport in viewports.values() {
            let ModalView { id, owned_by, .. } = view(viewport);
            if owned_by == Some(owner) && !ids.contains(&id) {
                ids.push(id);
            }
        }
    }

    ids.iter()
        .skip(owners.len())
        .filter_map(|id| viewports.get_mut(id).and_then(&mut take_window))
        .collect()
}

/// Disable the owners of modal viewports, and enable them again when no modal viewport needs them.
///
/// `disabled` is the set of owners this function has disabled so far.
/// Only viewports in `kept` (if given) count as existing.
///
/// Call this before windows are destroyed, so that an owner is enabled again
/// before its dialog window is destroyed (or Windows activates a window of another app).
///
/// Does nothing but a few lookups when nothing changes.
pub(crate) fn sync_modal_owners<'a, V: 'a>(
    viewports: &'a OrderedViewportIdMap<V>,
    view: impl Fn(&'a V) -> ModalView<'a>,
    kept: Option<&dyn Fn(ViewportId) -> bool>,
    disabled: &mut ViewportIdSet,
) {
    let modal_owner = |v: &'a V| {
        let view = view(v);
        if view.builder.modal != Some(true)
            || view.window.is_none()
            || !kept.is_none_or(|kept| kept(view.id))
        {
            return None;
        }
        // Only a window that was created owned is a dialog of its owner:
        resolve_owner(view.builder, view.id, view.parent)
            .filter(|owner| view.owned_by == Some(*owner))
    };
    let is_needed = |owner: ViewportId| viewports.values().any(|v| modal_owner(v) == Some(owner));

    // Enable first, so that an owner is never left disabled:
    disabled.retain(|owner| {
        // An owner without a window (recreated or gone) is forgotten: it is disabled again
        // below once it has a window and is needed.
        let Some(window) = viewports.get(owner).and_then(|v| view(v).window) else {
            return false;
        };
        if is_needed(*owner) {
            true
        } else {
            egui_winit::set_window_enabled(window, true);
            false
        }
    });

    for v in viewports.values() {
        if let Some(owner) = modal_owner(v)
            && !disabled.contains(&owner)
            && let Some(window) = viewports.get(&owner).and_then(|v| view(v).window)
        {
            egui_winit::set_window_enabled(window, false);
            disabled.insert(owner);
        }
    }
}

/// Enable every owner that [`sync_modal_owners`] disabled (on shutdown).
pub(crate) fn enable_all_modal_owners<'a, V: 'a>(
    viewports: &'a OrderedViewportIdMap<V>,
    view: impl Fn(&'a V) -> ModalView<'a>,
    disabled: &mut ViewportIdSet,
) {
    disabled.retain(|owner| {
        if let Some(window) = viewports.get(owner).and_then(|v| view(v).window) {
            egui_winit::set_window_enabled(window, true);
        }
        false
    });
}
