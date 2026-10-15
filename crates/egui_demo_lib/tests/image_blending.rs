use egui::{hex_color, include_image};
use egui_kittest::Harness;

#[test]
fn test_image_blending() {
    let mut results = egui_kittest::SnapshotResults::new();
    for pixels_per_point in [1.0, 2.0] {
        let mut harness = Harness::builder()
            .with_pixels_per_point(pixels_per_point)
            .build_ui(|ui| {
                egui_extras::install_image_loaders(ui.ctx());
                egui::Frame::new()
                    .fill(hex_color!("#5981FF"))
                    .show(ui, |ui| {
                        ui.add(
                            egui::Image::new(include_image!("../data/ring.png"))
                                .max_height(18.0)
                                .tint(egui::Color32::GRAY),
                        );
                    });
            });
        harness.run();
        harness.fit_contents();
        harness.snapshot(format!("image_blending/image_x{pixels_per_point}"));
        results.extend_harness(&mut harness);
    }
}
