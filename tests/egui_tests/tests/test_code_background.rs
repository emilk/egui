//! Snapshot test for the background of inline code, see [`egui::Visuals::code_bg_color`].

use egui::{Color32, CornerRadius, vec2};
use egui_kittest::{Harness, SnapshotResults};

#[test]
fn code_background() {
    let mut results = SnapshotResults::new();

    for (name, corner_radius, padding_x, text_color) in [
        ("default", CornerRadius::ZERO, 0.0, None),
        (
            "styled",
            CornerRadius::same(3),
            2.0,
            Some(Color32::LIGHT_BLUE),
        ),
    ] {
        let mut harness = Harness::builder()
            .with_size(vec2(240.0, 80.0))
            .build_ui(|ui| {
                ui.visuals_mut().code_bg_corner_radius = corner_radius;
                ui.visuals_mut().code_bg_padding_x = padding_x;
                ui.visuals_mut().code_text_color = text_color;
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("Call ");
                        ui.code("foo()");
                        ui.label(" or ");
                        ui.code("bar");
                        ui.label(", then wrap some very long text with ");
                        ui.code("more_code_at_the_end");
                    });
                });
            });
        harness.fit_contents();
        harness.snapshot(format!("code_background_{name}"));
        results.extend_harness(&mut harness);
    }
}
