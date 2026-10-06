use core::{
    cell::{Cell, RefCell},
    ptr::NonNull,
};
use std::{collections::HashMap, rc::Rc};

use block2::RcBlock;
use egui::{Rangef, Vec2};
use objc2::{
    MainThreadMarker,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_app_kit::{
    NSView, NSWindow, NSWindowButton, NSWindowDidChangeBackingPropertiesNotification,
    NSWindowDidEndLiveResizeNotification, NSWindowDidExitFullScreenNotification,
    NSWindowDidResizeNotification, NSWindowStyleMask, NSWindowWillCloseNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectProtocol};
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

    /// Position the traffic lights in a custom title bar.
    ///
    /// The buttons are centered vertically in `title_bar_y`
    /// (the y-range of your title bar, measured from the top of the window),
    /// with the close button `left_margin` from the left edge of the window.
    /// The native spacing between the buttons is preserved.
    ///
    /// Both arguments are in "native scale", just like [`Self::traffic_lights_size`],
    /// so multiply egui points by [`egui::Context::zoom_factor`] before passing them in,
    /// and call this again whenever the zoom factor changes.
    ///
    /// `AppKit` resets the position of the traffic lights whenever the window is laid out again
    /// (e.g. when resized or exiting fullscreen), so eframe remembers the placement and re-applies
    /// it after each frame (and on window resize notifications) until the window closes.
    /// Calling this again with new values replaces the previous placement.
    ///
    /// This is meant to be used together with [`egui::ViewportBuilder::with_fullsize_content_view`].
    ///
    /// Must be called on the main thread.
    /// Returns the updated window chrome metrics, or `None` on failure.
    pub fn position_traffic_lights(
        window_handle: &RawWindowHandle,
        title_bar_y: Rangef,
        left_margin: f32,
    ) -> Option<Self> {
        let RawWindowHandle::AppKit(appkit_handle) = window_handle else {
            return None;
        };
        MainThreadMarker::new()?;

        let ns_view = ns_view_from_handle(appkit_handle)?;
        let ns_window = ns_view.window()?;
        let placement = TrafficLightsPlacement {
            title_bar_top: title_bar_y.min as f64,
            title_bar_bottom: title_bar_y.max as f64,
            left_margin: left_margin as f64,
        };
        remember_traffic_lights_placement(&ns_window, placement);
        position_traffic_lights_in_title_bar(&ns_window, placement)?;

        Some(Self {
            traffic_lights_size: traffic_lights_metrics(&ns_window)?,
        })
    }
}

/// Re-apply the traffic lights placement set with
/// [`WindowChromeMetrics::position_traffic_lights`], if any.
///
/// `AppKit` lays out the title bar again during its display pass (e.g. while resizing),
/// which can happen after any notification we observe, so we also re-apply after each frame.
pub(crate) fn maintain_traffic_lights(window_handle: &RawWindowHandle) {
    let RawWindowHandle::AppKit(appkit_handle) = window_handle else {
        return;
    };
    if MainThreadMarker::new().is_none() {
        return;
    }
    let Some(ns_window) = ns_view_from_handle(appkit_handle).and_then(|view| view.window()) else {
        return;
    };
    let key = core::ptr::from_ref(&*ns_window) as usize;
    let placement = PLACED_WINDOWS
        .with_borrow(|windows| windows.get(&key).map(|placed| placed.placement.get()));
    if let Some(placement) = placement {
        position_traffic_lights_in_title_bar(&ns_window, placement);
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

#[derive(Clone, Copy, Debug)]
struct TrafficLightsPlacement {
    title_bar_top: f64,
    title_bar_bottom: f64,
    left_margin: f64,
}

/// A window whose traffic lights we keep re-positioning.
struct PlacedWindow {
    placement: Rc<Cell<TrafficLightsPlacement>>,
    observers: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
}

thread_local! {
    /// Keyed by `NSWindow` pointer. Entries are removed when the window closes.
    static PLACED_WINDOWS: RefCell<HashMap<usize, PlacedWindow>> = RefCell::default();
}

/// Remember the placement, and re-apply it whenever `AppKit` lays out the title bar again.
fn remember_traffic_lights_placement(ns_window: &NSWindow, placement: TrafficLightsPlacement) {
    let key = core::ptr::from_ref(ns_window) as usize;

    let existing = PLACED_WINDOWS.with_borrow(|windows| {
        windows
            .get(&key)
            .map(|placed| placed.placement.set(placement))
    });
    if existing.is_some() {
        return;
    }

    let shared_placement = Rc::new(Cell::new(placement));
    let center = NSNotificationCenter::defaultCenter();
    let mut observers = Vec::new();

    // SAFETY: the notification names are valid static strings provided by AppKit.
    #[expect(unsafe_code)]
    let reposition_on = unsafe {
        [
            NSWindowDidResizeNotification,
            NSWindowDidEndLiveResizeNotification,
            NSWindowDidExitFullScreenNotification,
            NSWindowDidChangeBackingPropertiesNotification,
        ]
    };
    for name in reposition_on {
        let shared_placement = Rc::clone(&shared_placement);
        let block = RcBlock::new(move |notification: NonNull<NSNotification>| {
            // SAFETY: AppKit hands us a valid notification for the duration of the callback.
            #[expect(unsafe_code)]
            let notification = unsafe { notification.as_ref() };
            if let Some(ns_window) = notification
                .object()
                .and_then(|object| object.downcast::<NSWindow>().ok())
            {
                position_traffic_lights_in_title_bar(&ns_window, shared_placement.get());
            }
        });
        // SAFETY: the block only runs on the posting (main) thread, and the observer is
        // removed again when the window closes.
        #[expect(unsafe_code)]
        let observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(name),
                Some(ns_window),
                None,
                &block,
            )
        };
        observers.push(observer);
    }

    let block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
        let placed = PLACED_WINDOWS.with_borrow_mut(|windows| windows.remove(&key));
        if let Some(placed) = placed {
            let center = NSNotificationCenter::defaultCenter();
            for observer in &placed.observers {
                // SAFETY: `observer` was returned by `addObserverForName_object_queue_usingBlock`.
                #[expect(unsafe_code)]
                unsafe {
                    center.removeObserver(AsRef::<AnyObject>::as_ref(&**observer));
                };
            }
        }
    });
    // SAFETY: the notification name is a valid static string provided by AppKit, and the block
    // only runs on the posting (main) thread.
    #[expect(unsafe_code)]
    let observer = unsafe {
        center.addObserverForName_object_queue_usingBlock(
            Some(NSWindowWillCloseNotification),
            Some(ns_window),
            None,
            &block,
        )
    };
    observers.push(observer);

    PLACED_WINDOWS.with_borrow_mut(|windows| {
        windows.insert(
            key,
            PlacedWindow {
                placement: shared_placement,
                observers,
            },
        );
    });
}

fn position_traffic_lights_in_title_bar(
    ns_window: &NSWindow,
    placement: TrafficLightsPlacement,
) -> Option<()> {
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
    let center_y =
        (0.5 * (placement.title_bar_top + placement.title_bar_bottom)).max(half_button_height);
    let title_bar_height = placement
        .title_bar_bottom
        .max(center_y + half_button_height);

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
    title_bar_container.setFrame(container_frame);
    title_bar_view.setFrame(title_bar_container.bounds());

    let x_offset = placement.left_margin - close_button_frame.origin.x;
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
        button.setFrameOrigin(origin);
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
