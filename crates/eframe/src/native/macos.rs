use egui::{Rangef, Vec2};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSView, NSWindow, NSWindowButton, NSWindowStyleMask};
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

/// Center the traffic lights vertically in `title_bar_y` (measured from the top of the window),
/// with the close button `left_margin` from the left edge of the window.
///
/// Both arguments are in native points.
///
/// `AppKit` resets the position whenever it lays out the title bar again (e.g. on resize),
/// so this needs to be called every frame.
pub(crate) fn position_traffic_lights(
    window_handle: &RawWindowHandle,
    title_bar_y: Rangef,
    left_margin: f32,
) -> Option<()> {
    let RawWindowHandle::AppKit(appkit_handle) = window_handle else {
        return None;
    };
    MainThreadMarker::new()?;

    let ns_window = ns_view_from_handle(appkit_handle)?.window()?;

    if ns_window
        .styleMask()
        .contains(NSWindowStyleMask::FullScreen)
    {
        // The traffic lights live in a separate window in fullscreen; leave them alone.
        return Some(());
    }

    let close_button = ns_window.standardWindowButton(NSWindowButton::CloseButton)?;
    let close_button_frame = close_button.frame();

    // The buttons live in an `NSTitlebarView`, inside an `NSTitlebarContainerView`,
    // inside the window's theme frame.
    // SAFETY: we are on the main thread, and every view stays retained while we access its superview.
    #[expect(unsafe_code)]
    let (title_bar_view, title_bar_container, theme_frame) = unsafe {
        let title_bar_view = close_button.superview()?;
        let title_bar_container = title_bar_view.superview()?;
        let theme_frame = title_bar_container.superview()?;
        (title_bar_view, title_bar_container, theme_frame)
    };

    // Distance from the top of the window to the center of the buttons.
    // Clamped so the buttons never go above the top of the window.
    let half_button_height = close_button_frame.size.height / 2.0;
    let center_y = (title_bar_y.center() as f64).max(half_button_height);
    let title_bar_height = (title_bar_y.max as f64).max(center_y + half_button_height);

    // Resize the native title bar so that it covers the buttons.
    // Otherwise taller title bars would push the buttons outside their superview,
    // where they no longer receive clicks.
    let theme_bounds = theme_frame.bounds();
    let mut container_frame = title_bar_container.frame();
    container_frame.size.height = title_bar_height;
    container_frame.origin.y = if theme_frame.isFlipped() {
        theme_bounds.origin.y
    } else {
        theme_bounds.origin.y + theme_bounds.size.height - title_bar_height
    };
    // Only touch the frames when they change, to avoid needless re-layouts every frame:
    if title_bar_container.frame() != container_frame {
        title_bar_container.setFrame(container_frame);
    }
    if title_bar_view.frame() != title_bar_container.bounds() {
        title_bar_view.setFrame(title_bar_container.bounds());
    }

    let x_offset = left_margin as f64 - close_button_frame.origin.x;
    let bounds = title_bar_view.bounds();
    let flipped = title_bar_view.isFlipped();

    for button_kind in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        let button = ns_window.standardWindowButton(button_kind)?;
        let frame = button.frame();
        let top_margin = center_y - frame.size.height / 2.0;
        let mut origin = frame.origin;
        origin.x += x_offset;
        origin.y = if flipped {
            bounds.origin.y + top_margin
        } else {
            bounds.origin.y + bounds.size.height - top_margin - frame.size.height
        };
        if origin != frame.origin {
            button.setFrameOrigin(origin);
        }
    }

    Some(())
}

fn distance_from_top(view: &NSView) -> Option<f64> {
    let frame = view.frame();
    // SAFETY: we are on the main thread, and the caller retains `view`
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
