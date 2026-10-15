use egui::{Vec2, include_image};
use egui_kittest::Harness;

#[test]
fn should_wait_for_images() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(60.0, 120.0))
        .build_ui(|ui| {
            egui_extras::install_image_loaders(ui.ctx());
            let size = Vec2::splat(30.0);
            ui.label("Url:");
            ui.add_sized(
                size,
                egui::Image::new(
                    "https://raw.githubusercontent.com\
                    /emilk/egui/refs/heads/main/crates/eframe/data/icon.png",
                ),
            );

            ui.label("Include:");
            ui.add_sized(
                size,
                egui::Image::new(include_image!("../../../crates/eframe/data/icon.png")),
            );
        });

    harness.snapshot("should_wait_for_images");
}
