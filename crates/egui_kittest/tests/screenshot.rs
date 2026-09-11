#![cfg(feature = "wgpu")]

use std::sync::Arc;

use egui::{Color32, Vec2, mutex::Mutex};
use egui_kittest::Harness;

/// Requesting a screenshot from within the app should be fulfilled by the harness (when
/// rendering is enabled) and delivered to the callback.
#[test]
fn screenshot_callback() {
    let screenshot = Arc::new(Mutex::new(None));
    let screenshot_from_callback = Arc::clone(&screenshot);
    let mut requested = false;

    let mut harness = Harness::builder()
        .with_size(Vec2::new(100.0, 80.0))
        .build_ui(move |ui| {
            // Paint the whole content area with a known color so we can verify the capture.
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, Color32::RED);

            if !requested {
                requested = true;
                let screenshot = Arc::clone(&screenshot_from_callback);
                ui.ctx().request_screenshot(move |image| {
                    *screenshot.lock() = Some(image);
                });
            }
        });

    let screenshot = screenshot
        .lock()
        .clone()
        .expect("Expected the screenshot callback to be invoked");

    // The frame was filled with red, so the center pixel should be red.
    let center = screenshot.pixels[screenshot.pixels.len() / 2];
    assert_eq!(center, Color32::RED, "center pixel should be red");

    // The screenshot should match the rendered frame size.
    let rendered = harness.render().unwrap();
    assert_eq!(screenshot.width() as u32, rendered.width());
    assert_eq!(screenshot.height() as u32, rendered.height());
}
