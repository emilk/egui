#![cfg(feature = "wgpu")]

use std::sync::Arc;

use egui::{Color32, ColorImage, Vec2};
use egui_kittest::Harness;

/// Requesting a screenshot via [`egui::ViewportCommand::Screenshot`] from within the app should
/// be fulfilled by the harness (when rendering is enabled) and delivered back via
/// [`egui::Event::Screenshot`].
#[test]
#[cfg(any(feature = "wgpu", feature = "snapshot"))]
fn screenshot_viewport_command() {
    #[derive(Default)]
    struct State {
        requested: bool,
        screenshot: Option<Arc<ColorImage>>,
    }

    let mut harness = Harness::builder()
        .with_size(Vec2::new(100.0, 80.0))
        .build_ui_state(
            |ui, state: &mut State| {
                // Paint the whole content area with a known color so we can verify the capture.
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, Color32::RED);

                // Request a screenshot once.
                if !state.requested {
                    state.requested = true;
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                }

                // Capture the screenshot once it's delivered.
                ui.input(|i| {
                    for event in &i.raw.events {
                        if let egui::Event::Screenshot { image, .. } = event {
                            state.screenshot = Some(Arc::clone(image));
                        }
                    }
                });
            },
            State::default(),
        );

    harness.run();

    let screenshot = harness
        .state()
        .screenshot
        .clone()
        .expect("Expected a screenshot to be delivered via Event::Screenshot");

    // The frame was filled with red, so the center pixel should be red.
    let center = screenshot.pixels[screenshot.pixels.len() / 2];
    assert_eq!(center, Color32::RED, "center pixel should be red");

    // The screenshot should match the rendered frame size.
    let rendered = harness.render().unwrap();
    assert_eq!(screenshot.width() as u32, rendered.width());
    assert_eq!(screenshot.height() as u32, rendered.height());
}
