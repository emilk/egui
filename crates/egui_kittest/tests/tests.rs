use egui::{Modifiers, ScrollArea, Vec2, include_image};
use egui_kittest::{Harness, SnapshotResults};
use kittest::Queryable as _;

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

    harness
        .get_by_label("Click me")
        .click_modifiers(Modifiers::COMMAND);
    harness.run();

    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::Z);
    harness.run();

    harness.key_combination_modifiers(Modifiers::COMMAND, &[egui::Key::Y]);
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

fn test_scroll_harness() -> Harness<'static, bool> {
    Harness::builder()
        .with_size(Vec2::new(100.0, 200.0))
        .build_ui_state(
            |ui, state| {
                ScrollArea::vertical().show(ui, |ui| {
                    for i in 0..20 {
                        ui.label(format!("Item {}", i));
                    }
                    if ui.button("Hidden Button").clicked() {
                        *state = true;
                    };
                });
            },
            false,
        )
}

#[test]
fn test_scroll_to_me() {
    let mut harness = test_scroll_harness();
    let mut results = SnapshotResults::new();

    results.add(harness.try_snapshot("test_scroll_initial"));

    harness.get_by_label("Hidden Button").scroll_to_me();

    harness.run();
    results.add(harness.try_snapshot("test_scroll_scrolled"));

    harness.get_by_label("Hidden Button").click();
    harness.run();

    assert!(
        harness.state(),
        "The button was not clicked after scrolling."
    );
}

#[test]
fn test_scroll_down() {
    let mut harness = test_scroll_harness();

    harness.get_by_label("Hidden Button").scroll_up();
    harness.run();
    harness.get_by_label("Hidden Button").scroll_up();
    harness.run();
    harness.get_by_label("Hidden Button").scroll_up();
    harness.run();
    harness.get_by_label("Hidden Button").scroll_up();
    harness.run();
    harness.get_by_label("Hidden Button").scroll_up();
    harness.run();

    harness.get_by_label("Hidden Button").click();
    harness.run();

    assert!(
        harness.state(),
        "The button was not clicked after scrolling down. (Probably not scrolled enough / at all)"
    );
}
