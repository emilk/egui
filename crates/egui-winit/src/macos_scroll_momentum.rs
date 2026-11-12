//! Detect what drives a scroll on macOS: a wheel, fingers on a trackpad, or OS momentum.
//!
//! `AppKit` tells us with `NSEvent::phase` and `NSEvent::momentumPhase`, but winit collapses
//! both into the regular `TouchPhase` (so a momentum scroll looks just like a second finger
//! scroll), and reports a phase-less event as `TouchPhase::Moved` (so a wheel looks like a
//! finger mid-gesture). We observe the raw `NSEvent`s ourselves with a local event monitor
//! and remember what drove the latest scroll event:
//!
//! * a momentum phase: the OS continuing a finger scroll ([`egui::MouseWheelSource::Momentum`]),
//! * a gesture phase: fingers on a trackpad or a Magic Mouse ([`egui::MouseWheelSource::Trackpad`]),
//! * no phase at all: a wheel ([`egui::MouseWheelSource::Wheel`]).
//!
//! Precise (pixel) deltas don't mean a trackpad: mice with smooth scrolling in their driver
//! (e.g. Logitech Options+) send precise deltas too, but never with a phase.
//!
//! TODO(emilk): remove this module once we are on a winit version that includes
//! <https://github.com/rust-windowing/winit/pull/4732>, which adds `ScrollSource`
//! to `WindowEvent::MouseWheel`.
//!
//! The monitor runs before the event is dispatched to the window, so when winit hands us
//! the corresponding `MouseWheel` event, the source describes it. If several scroll events
//! are delivered before winit dispatches them, it describes the last one; only the
//! `Start` of a gesture reads it, so the worst case is a very short momentum
//! phase being taken for a finger scroll.

#![allow(unsafe_code)] // Talking to AppKit

use core::ptr::NonNull;
use core::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use egui::MouseWheelSource;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSEvent, NSEventMask, NSEventPhase};

/// What drives a scroll event, from its `AppKit` phases.
fn source_of(phase: NSEventPhase, momentum_phase: NSEventPhase) -> MouseWheelSource {
    if momentum_phase != NSEventPhase::None {
        MouseWheelSource::Momentum
    } else if phase != NSEventPhase::None {
        MouseWheelSource::Trackpad
    } else {
        MouseWheelSource::Wheel
    }
}

fn encode(source: MouseWheelSource) -> u8 {
    match source {
        MouseWheelSource::Unknown => 0,
        MouseWheelSource::Wheel => 1,
        MouseWheelSource::Trackpad => 2,
        MouseWheelSource::Momentum => 3,
    }
}

fn decode(source: u8) -> MouseWheelSource {
    match source {
        1 => MouseWheelSource::Wheel,
        2 => MouseWheelSource::Trackpad,
        3 => MouseWheelSource::Momentum,
        _ => MouseWheelSource::Unknown,
    }
}

/// Observes scroll events for as long as it lives.
pub struct ScrollSourceMonitor {
    /// The `NSEvent` local monitor. Removed on drop.
    monitor: Retained<AnyObject>,

    /// Keeps the handler alive for as long as the monitor is installed.
    _handler: block2::RcBlock<dyn Fn(NonNull<NSEvent>) -> *mut NSEvent>,

    /// What drove the latest scroll event, see [`encode`].
    latest_source: Arc<AtomicU8>,
}

impl ScrollSourceMonitor {
    /// Start observing scroll events.
    ///
    /// Returns `None` when not on the main thread, since `AppKit` requires it.
    pub fn install() -> Option<Self> {
        if MainThreadMarker::new().is_none() {
            log::debug!("Not on the main thread; cannot detect what drives scrolling");
            return None;
        }

        let latest_source = Arc::new(AtomicU8::new(encode(MouseWheelSource::Unknown)));

        let handler = {
            let latest_source = Arc::clone(&latest_source);
            block2::RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
                // SAFETY: AppKit hands us a valid event for the duration of the call.
                let event_ref = unsafe { event.as_ref() };
                let source = source_of(event_ref.phase(), event_ref.momentumPhase());
                latest_source.store(encode(source), Ordering::Relaxed);
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
            latest_source,
        })
    }

    /// What drove the latest scroll event?
    ///
    /// [`MouseWheelSource::Unknown`] before the first one.
    pub fn latest_scroll_source(&self) -> MouseWheelSource {
        decode(self.latest_source.load(Ordering::Relaxed))
    }
}

impl Drop for ScrollSourceMonitor {
    fn drop(&mut self) {
        // SAFETY: `monitor` came from `addLocalMonitorForEventsMatchingMask_handler`.
        unsafe { NSEvent::removeMonitor(&self.monitor) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_phase_less_event_is_a_wheel() {
        // A notched wheel, and also a mouse with smooth scrolling in its driver,
        // which sends precise deltas without any phase.
        assert_eq!(
            source_of(NSEventPhase::None, NSEventPhase::None),
            MouseWheelSource::Wheel
        );
    }

    #[test]
    fn a_gesture_phase_is_a_trackpad() {
        for phase in [
            NSEventPhase::MayBegin,
            NSEventPhase::Began,
            NSEventPhase::Changed,
            NSEventPhase::Stationary,
            NSEventPhase::Ended,
            NSEventPhase::Cancelled,
        ] {
            assert_eq!(
                source_of(phase, NSEventPhase::None),
                MouseWheelSource::Trackpad,
                "{phase:?}"
            );
        }
    }

    #[test]
    fn a_momentum_phase_is_momentum() {
        for momentum in [
            NSEventPhase::Began,
            NSEventPhase::Changed,
            NSEventPhase::Ended,
        ] {
            assert_eq!(
                source_of(NSEventPhase::None, momentum),
                MouseWheelSource::Momentum,
                "{momentum:?}"
            );
        }
    }

    #[test]
    fn the_source_survives_the_atomic() {
        for source in [
            MouseWheelSource::Unknown,
            MouseWheelSource::Wheel,
            MouseWheelSource::Trackpad,
            MouseWheelSource::Momentum,
        ] {
            assert_eq!(decode(encode(source)), source);
        }
    }
}
