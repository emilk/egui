use egui::Color32;
use egui::color_picker::{Alpha, color_picker_color32};
use egui_kittest::Harness;

#[test]
fn color_picker() {
    let mut color = Color32::from_rgba_unmultiplied(130, 130, 130, 45);
    let mut harness = Harness::builder()
        .with_pixels_per_point(2.0)
        .build_ui(move |ui| {
            ui.spacing_mut().slider_width = 275.0;
            color_picker_color32(ui, &mut color, Alpha::OnlyBlend);
        });
    harness.run();
    harness.fit_contents();
    harness.snapshot("color_picker");
}
