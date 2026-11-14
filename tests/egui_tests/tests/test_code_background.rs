//! Snapshot test for the background of inline code, see [`egui::CodeVisuals`].

use egui::{CodeVisuals, Color32, CornerRadius, Margin, Visuals, vec2};
use egui_kittest::{Harness, SnapshotResults};

#[test]
fn code_background() {
    let mut results = SnapshotResults::new();

    let custom = Visuals {
        code: CodeVisuals {
            corner_radius: CornerRadius::ZERO,
            text_color: Some(Color32::LIGHT_BLUE),
            margin: Margin::same(1),
            ..CodeVisuals::dark()
        },
        ..Visuals::dark()
    };

    for (name, visuals) in [
        ("dark", Visuals::dark()),
        ("light", Visuals::light()),
        ("custom", custom),
    ] {
        let mut harness = Harness::builder()
            .with_size(vec2(240.0, 120.0))
            .build_ui(|ui| {
                *ui.visuals_mut() = visuals.clone();
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("Call ");
                        ui.code("foo()");
                        ui.label(" or ");
                        ui.code("bar");
                        ui.label(", then wrap some very long text with ");
                        ui.code("more_code_at_the_end");
                        ui.code("code_on_the_row_below");
                    });
                });
            });
        harness.fit_contents();
        harness.snapshot(format!("code_background_{name}"));
        results.extend_harness(&mut harness);
    }
}
