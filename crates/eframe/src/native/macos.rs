use egui::Vec2;
use objc2_app_kit::{NSView, NSWindow, NSWindowButton};
use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};

/// Size of the "traffic lights" (red/yellow/green close/minimize/maximize buttons)
/// on the native macOS window.
///
/// This is very useful together with [`egui::ViewportBuilder::with_fullsize_content_view`].
#[derive(Debug)]
pub struct WindowChromeMetrics {
    /// Size of the "traffic lights" (red/yellow/green close/minimize/maximize buttons),
    /// including margins.
    pub traffic_lights_size: Vec2,
}

impl WindowChromeMetrics {
    /// Get the window chrome metrics for a given window handle.
    pub fn from_window_handle(window_handle: &RawWindowHandle) -> Option<Self> {
        window_chrome_metrics(window_handle)
    }
}

fn window_chrome_metrics(window_handle: &RawWindowHandle) -> Option<WindowChromeMetrics> {
    let RawWindowHandle::AppKit(appkit_handle) = window_handle else {
        return None;
    };

    let ns_view = ns_view_from_handle(appkit_handle)?;
    let ns_window = ns_view.window()?;

    Some(WindowChromeMetrics {
        traffic_lights_size: traffic_lights_metrics(&ns_window)?,
    })
}

fn traffic_lights_metrics(ns_window: &NSWindow) -> Option<Vec2> {
    // Button order is CloseButton, MiniaturizeButton, ZoomButton:
    let close_button = ns_window
        .standardWindowButton(NSWindowButton::CloseButton)?
        .frame();
    let zoom_button = ns_window
        .standardWindowButton(NSWindowButton::ZoomButton)?
        .frame();

    let left_margin = close_button.origin.x;
    let right_margin = left_margin; // for symmetry

    let total_width = zoom_button.origin.x + zoom_button.size.width + right_margin;

    let top_margin = close_button.origin.y;
    let bottom_margin = top_margin; // Usually symmetric
    let total_height = top_margin + close_button.size.height + bottom_margin;

    Some(Vec2::new(total_width as f32, total_height as f32))
}

fn ns_view_from_handle(handle: &AppKitWindowHandle) -> Option<&NSView> {
    let ns_view_ptr = handle.ns_view.as_ptr().cast::<NSView>();

    // Validate the pointer is non-null
    if ns_view_ptr.is_null() {
        None
    } else {
        // SAFETY:
        // - We've verified the pointer is non-null
        // - The pointer comes from the windowing system, so it should be valid
        // - NSView pointers from AppKit are expected to remain valid for the window lifetime
        #[expect(unsafe_code)]
        unsafe {
            ns_view_ptr.as_ref()
        }
    }
}
