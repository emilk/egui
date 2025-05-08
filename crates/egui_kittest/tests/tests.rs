use egui::{include_image, Modifiers, Vec2};
use egui_kittest::Harness;
use kittest::{Key, Queryable as _};

#[test]
fn test_shrink() {
    let mut harness = Harness::new_ui(|ui| {
        ui.label("Hello, world!");
        ui.separator();
        ui.label("This is a test");
    });

    harness.fit_contents();

    #[cfg(all(feature = "snapshot", feature = "wgpu"))]
    harness.snapshot("test_shrink");
}

#[test]
fn test_modifiers() {
    #[derive(Default)]
    struct State {
        cmd_clicked: bool,
        cmd_z_pressed: bool,
        cmd_y_pressed: bool,
    }
    let mut harness = Harness::new_ui_state(
        |ui, state| {
            if ui.button("Click me").clicked() && ui.input(|i| i.modifiers.command) {
                state.cmd_clicked = true;
            }
            if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
                state.cmd_z_pressed = true;
            }
            if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Y)) {
                state.cmd_y_pressed = true;
            }
        },
        State::default(),
    );

    harness.get_by_label("Click me").key_down(Key::Command);
    // This run isn't necessary, but allows us to test whether modifiers are remembered between frames
    harness.run();
    harness.get_by_label("Click me").click();
    harness.get_by_label("Click me").key_up(Key::Command);
    harness.run();

    harness.press_key_modifiers(Modifiers::COMMAND, egui::Key::Z);
    harness.run();

    harness.node().key_combination(&[Key::Command, Key::Y]);
    harness.run();

    let state = harness.state();
    assert!(state.cmd_clicked, "The button wasn't command-clicked");
    assert!(state.cmd_z_pressed, "Cmd+Z wasn't pressed");
    assert!(state.cmd_y_pressed, "Cmd+Y wasn't pressed");
}

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
                egui::Image::new(include_image!("../../eframe/data/icon.png")),
            );
        });

    harness.snapshot("should_wait_for_images");
}
