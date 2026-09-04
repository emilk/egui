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
    ///
    /// The unit here is in "native scale", which means it needs to be divided by [`egui::Context::zoom_factor`]
    /// to get the size in egui points.
    pub traffic_lights_size: Vec2,
}

impl WindowChromeMetrics {
    /// Get the window chrome metrics for a given window handle.
    pub fn from_window_handle(window_handle: &RawWindowHandle) -> Option<Self> {
        window_chrome_metrics(window_handle)
    }

    /// Position the traffic lights in a title bar of the given height.
    ///
    /// The buttons are centered vertically and inset by `left_margin`.
    /// Both arguments use the same native scale as [`Self::traffic_lights_size`].
    /// Returns the updated window chrome metrics.
    pub fn position_traffic_lights(
        window_handle: &RawWindowHandle,
        title_bar_height: f32,
        left_margin: f32,
    ) -> Option<Self> {
        let RawWindowHandle::AppKit(appkit_handle) = window_handle else {
            return None;
        };

        let ns_view = ns_view_from_handle(appkit_handle)?;
        let ns_window = ns_view.window()?;
        position_traffic_lights_in_title_bar(&ns_window, title_bar_height, left_margin)?;

        Some(Self {
            traffic_lights_size: traffic_lights_metrics(&ns_window)?,
        })
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
    let close_button = ns_window.standardWindowButton(NSWindowButton::CloseButton)?;
    let close_button_frame = close_button.frame();
    let zoom_button = ns_window
        .standardWindowButton(NSWindowButton::ZoomButton)?
        .frame();

    let left_margin = close_button_frame.origin.x;
    let right_margin = left_margin; // for symmetry

    let total_width = zoom_button.origin.x + zoom_button.size.width + right_margin;

    let top_margin = distance_from_top(&close_button)?;
    let bottom_margin = top_margin; // Usually symmetric
    let total_height = top_margin + close_button_frame.size.height + bottom_margin;

    Some(Vec2::new(total_width as f32, total_height as f32))
}

fn position_traffic_lights_in_title_bar(
    ns_window: &NSWindow,
    title_bar_height: f32,
    left_margin: f32,
) -> Option<()> {
    let close_button_x = ns_window
        .standardWindowButton(NSWindowButton::CloseButton)?
        .frame()
        .origin
        .x;
    let x_offset = left_margin as f64 - close_button_x;

    for button_kind in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        let button = ns_window.standardWindowButton(button_kind)?;
        let frame = button.frame();
        // SAFETY: native eframe window updates run on the main thread, and `button` stays retained
        // while its superview is accessed.
        #[expect(unsafe_code)]
        let superview = unsafe { button.superview()? };
        let bounds = superview.bounds();
        let top_margin = ((title_bar_height as f64 - frame.size.height) / 2.0).max(0.0);
        let mut origin = frame.origin;
        origin.x += x_offset;
        origin.y = if superview.isFlipped() {
            bounds.origin.y + top_margin
        } else {
            bounds.origin.y + bounds.size.height - top_margin - frame.size.height
        };
        button.setFrameOrigin(origin);
    }

    Some(())
}

fn distance_from_top(view: &NSView) -> Option<f64> {
    let frame = view.frame();
    // SAFETY: native eframe window updates run on the main thread, and the caller retains `view`
    // while its superview is accessed.
    #[expect(unsafe_code)]
    let superview = unsafe { view.superview()? };
    let bounds = superview.bounds();

    if superview.isFlipped() {
        Some(frame.origin.y - bounds.origin.y)
    } else {
        Some(bounds.origin.y + bounds.size.height - frame.origin.y - frame.size.height)
    }
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
