//! macOS-specific handling of native viewport windows.
//!
//! `winit` 0.30 does not expose `NSWindowCollectionBehavior`, so we set it here via
//! `objc2-app-kit`. Once egui is on a `winit` version with fullscreen-auxiliary support
//! (proposed upstream for 0.31), most of this module can be replaced by
//! `WindowAttributesMacOS::with_fullscreen_auxiliary` etc., keeping only the default
//! policy in [`should_be_fullscreen_auxiliary`].

use egui::ViewportBuilder;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{
    NSApplication, NSView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
use winit::window::Window;

/// Should the window created for this viewport be marked as a
/// "fullscreen auxiliary" window (`NSWindowCollectionBehaviorFullScreenAuxiliary`)?
///
/// See [`ViewportBuilder::with_fullscreen_auxiliary`].
pub(crate) fn should_be_fullscreen_auxiliary(builder: &ViewportBuilder) -> bool {
    if let Some(explicit) = builder.fullscreen_auxiliary {
        return explicit;
    }

    // Default: if the app currently has a native fullscreen window on the active Space,
    // showing a normal new window would make macOS renegotiate the Space,
    // flickering and potentially aborting the fullscreen state or triggering
    // a Split View (https://github.com/emilk/egui/issues/8259).
    // So mark the new window as an auxiliary window that can share the fullscreen Space —
    // unless it wants to become fullscreen itself,
    // which requires the (mutually exclusive) primary fullscreen behavior.
    let wants_fullscreen = builder.fullscreen.unwrap_or(false) || builder.monitor.is_some();
    !wants_fullscreen && app_has_fullscreen_window_on_active_space()
}

fn app_has_fullscreen_window_on_active_space() -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false; // AppKit windows can only be inspected on the main thread
    };
    let app = NSApplication::sharedApplication(mtm);
    app.windows().iter().any(|window| {
        window.styleMask().contains(NSWindowStyleMask::FullScreen) && window.isOnActiveSpace()
    })
}

/// Finish the initialization of a window that [`should_be_fullscreen_auxiliary`],
/// and was therefore created hidden (see `create_winit_window_attributes`):
/// mark it as a fullscreen-auxiliary window,
/// so that ordering it on screen won't disturb any active fullscreen Space.
pub(crate) fn apply_fullscreen_auxiliary(window: &Window, builder: &ViewportBuilder) {
    if should_be_fullscreen_auxiliary(builder) {
        let Some(ns_window) = ns_window_from_winit(window) else {
            log::warn!("Failed to get NSWindow to mark the window as fullscreen-auxiliary");
            return;
        };
        log::debug!(
            "Marking new window {:?} as fullscreen-auxiliary",
            builder.title
        );
        ns_window.setCollectionBehavior(
            ns_window.collectionBehavior() | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }

    // The window was created hidden so that the collection behavior above
    // takes effect before the window is first ordered on screen.
    // Show it now, if the builder asked for a visible window:
    if builder.visible.unwrap_or(true) && window.is_visible() == Some(false) {
        let Some(ns_window) = ns_window_from_winit(window) else {
            log::warn!("Failed to get NSWindow to show the window");
            return;
        };
        if builder.active.unwrap_or(true) {
            ns_window.makeKeyAndOrderFront(None);
        } else {
            ns_window.orderFront(None);
        }
    }
}

/// A window marked as fullscreen-auxiliary cannot enter native fullscreen,
/// so clear the flag before any attempt to make the window fullscreen.
///
/// See [`ViewportBuilder::with_fullscreen_auxiliary`].
pub(crate) fn clear_fullscreen_auxiliary(window: &Window) {
    let Some(ns_window) = ns_window_from_winit(window) else {
        log::warn!("Failed to get NSWindow to clear the fullscreen-auxiliary state");
        return;
    };
    let behavior = ns_window.collectionBehavior();
    if behavior.contains(NSWindowCollectionBehavior::FullScreenAuxiliary) {
        ns_window.setCollectionBehavior(behavior - NSWindowCollectionBehavior::FullScreenAuxiliary);
    }
}

fn ns_window_from_winit(window: &Window) -> Option<Retained<NSWindow>> {
    let handle = window.window_handle().ok()?.as_raw();
    let RawWindowHandle::AppKit(handle) = handle else {
        return None;
    };
    let ns_view = handle.ns_view.as_ptr().cast::<NSView>();

    // SAFETY: the pointer comes from winit, and is valid for as long as `window` is
    #[expect(unsafe_code)]
    let ns_view = unsafe { ns_view.as_ref() }?;

    ns_view.window()
}
