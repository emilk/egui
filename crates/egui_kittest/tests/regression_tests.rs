use egui::{Button, Image, Vec2, Widget};
use egui_kittest::{kittest::Queryable, Harness};

#[test]
pub fn focus_should_skip_over_disabled_buttons() {
    let mut harness = Harness::new_ui(|ui| {
        ui.add(Button::new("Button 1"));
        ui.add_enabled(false, Button::new("Button Disabled"));
        ui.add(Button::new("Button 3"));
    });

    harness.press_key(egui::Key::Tab);
    harness.run();

    let button_1 = harness.get_by_label("Button 1");
    assert!(button_1.is_focused());

    harness.press_key(egui::Key::Tab);
    harness.run();

    let button_3 = harness.get_by_label("Button 3");
    assert!(button_3.is_focused());

    harness.press_key(egui::Key::Tab);
    harness.run();

    let button_1 = harness.get_by_label("Button 1");
    assert!(button_1.is_focused());
}

#[test]
fn image_failed() {
    let mut harness = Harness::new_ui(|ui| {
        Image::new("file://invalid/path")
            .alt_text("I have an alt text")
            .max_size(Vec2::new(100.0, 100.0))
            .ui(ui);
    });

    harness.run();
    harness.fit_contents();

    #[cfg(all(feature = "wgpu", feature = "snapshot"))]
    harness.snapshot("image_snapshots");
}
