use egui::{Image, include_image};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

#[test]
fn image_button_should_have_alt_text() {
    let harness = Harness::new_ui(|ui| {
        _ = ui.button(
            Image::new(include_image!("../../../crates/eframe/data/icon.png")).alt_text("Egui"),
        );
    });

    harness.get_by_label("Egui");
}
