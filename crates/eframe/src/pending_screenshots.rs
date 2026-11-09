use core::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use egui::ScreenshotCallback;

/// Tracks readbacks until their callback is either invoked or dropped on an error.
#[derive(Default)]
pub(super) struct PendingScreenshots {
    count: Arc<AtomicUsize>,
}

impl PendingScreenshots {
    /// Whether any readback still owns callbacks that need to be driven by polling.
    pub fn is_pending(&self) -> bool {
        self.count.load(Ordering::Relaxed) != 0
    }

    /// Track a readback until it delivers its image or drops its callbacks on failure.
    pub fn track(&self, callbacks: Vec<ScreenshotCallback>) -> ScreenshotCallback {
        self.count.fetch_add(1, Ordering::Relaxed);
        let pending = PendingScreenshot(Arc::clone(&self.count));
        ScreenshotCallback::new(move |image| {
            let _pending = pending;
            for callback in callbacks {
                callback.complete(Arc::clone(&image));
            }
        })
    }
}

struct PendingScreenshot(Arc<AtomicUsize>);

impl Drop for PendingScreenshot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completing_a_readback_delivers_all_callbacks_and_stops_polling() {
        let pending = PendingScreenshots::default();
        let received = Arc::new(AtomicUsize::new(0));
        let callbacks = (0..2)
            .map(|_| {
                let received = Arc::clone(&received);
                ScreenshotCallback::new(move |image| {
                    assert_eq!([image.width(), image.height()], [1, 1]);
                    received.fetch_add(1, Ordering::Relaxed);
                })
            })
            .collect();
        let callback = pending.track(callbacks);
        let duplicate = callback.clone();
        assert!(pending.is_pending());
        let image = Arc::new(egui::ColorImage::filled([1, 1], egui::Color32::WHITE));
        callback.complete(Arc::clone(&image));
        duplicate.complete(image);
        assert_eq!(received.load(Ordering::Relaxed), 2);
        assert!(!pending.is_pending());
    }

    #[test]
    fn failed_readback_drops_callbacks_without_leaving_a_repaint_loop() {
        let pending = PendingScreenshots::default();
        let callback = pending.track(vec![ScreenshotCallback::new(|_| {
            panic!("A failed readback must not deliver an image");
        })]);
        assert!(pending.is_pending());
        // CaptureState drops its callbacks when mapping or reading the buffer fails.
        drop(callback);
        assert!(!pending.is_pending());
    }

    #[test]
    fn completing_one_readback_does_not_abandon_another_pending_readback() {
        let pending = PendingScreenshots::default();
        let first = pending.track(vec![]);
        let second = pending.track(vec![]);
        first.complete(Arc::new(egui::ColorImage::filled(
            [1, 1],
            egui::Color32::WHITE,
        )));
        assert!(pending.is_pending());
        drop(second);
        assert!(!pending.is_pending());
    }
}
