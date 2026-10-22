//! Detect the OS-driven momentum ("kinetic") phase of trackpad scrolling on macOS.
//!
//! `AppKit` reports it via `NSEvent::momentumPhase`, but winit collapses that into the
//! regular `TouchPhase`, so a momentum scroll looks just like a second finger scroll.
//! We observe the raw `NSEvent`s ourselves with a local event monitor and remember
//! whether the latest scroll event was part of the momentum phase.
//!
//! TODO(emilk): remove this module once we are on a winit version that includes
//! <https://github.com/rust-windowing/winit/pull/4732>, which adds `ScrollSource`
//! to `WindowEvent::MouseWheel`. Then read `source == ScrollSource::Momentum` instead.
//!
//! The monitor runs before the event is dispatched to the window, so when winit hands us
//! the corresponding `MouseWheel` event, the flag describes it. If several scroll events
//! are delivered before winit dispatches them, the flag describes the last one; only the
//! `Start` of a gesture reads the flag, so the worst case is a very short momentum
//! phase being taken for a finger scroll.

#![allow(unsafe_code)] // Talking to AppKit

use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSEvent, NSEventMask, NSEventPhase};

/// Observes scroll events for as long as it lives.
pub struct ScrollMomentumMonitor {
    /// The `NSEvent` local monitor. Removed on drop.
    monitor: Retained<AnyObject>,

    /// Keeps the handler alive for as long as the monitor is installed.
    _handler: block2::RcBlock<dyn Fn(NonNull<NSEvent>) -> *mut NSEvent>,

    /// Was the latest scroll event part of the OS-driven momentum phase?
    latest_is_momentum: Arc<AtomicBool>,
}

impl ScrollMomentumMonitor {
    /// Start observing scroll events.
    ///
    /// Returns `None` when not on the main thread, since `AppKit` requires it.
    pub fn install() -> Option<Self> {
        if MainThreadMarker::new().is_none() {
            log::debug!("Not on the main thread; cannot detect momentum scrolling");
            return None;
        }

        let latest_is_momentum = Arc::new(AtomicBool::new(false));

        let handler = {
            let latest_is_momentum = Arc::clone(&latest_is_momentum);
            block2::RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
                // SAFETY: AppKit hands us a valid event for the duration of the call.
                let is_momentum = unsafe { event.as_ref() }.momentumPhase() != NSEventPhase::None;
                latest_is_momentum.store(is_momentum, Ordering::Relaxed);
                event.as_ptr() // Pass the event on unchanged
            })
        };

        // SAFETY: we are on the main thread, and the handler returns a valid event pointer.
        let monitor = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                NSEventMask::ScrollWheel,
                &handler,
            )
        }?;

        Some(Self {
            monitor,
            _handler: handler,
            latest_is_momentum,
        })
    }

    /// Was the latest scroll event part of the OS-driven momentum phase?
    pub fn latest_scroll_event_is_momentum(&self) -> bool {
        self.latest_is_momentum.load(Ordering::Relaxed)
    }
}

impl Drop for ScrollMomentumMonitor {
    fn drop(&mut self) {
        // SAFETY: `monitor` came from `addLocalMonitorForEventsMatchingMask_handler`.
        unsafe { NSEvent::removeMonitor(&self.monitor) };
    }
}
