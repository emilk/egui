use egui_kittest::Harness;
use kittest::{Key, Queryable};

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
    let mut harness = Harness::new_ui_state(
        |ui, cmd_clicked| {
            if ui.button("Click me").clicked() && ui.input(|i| i.modifiers.command) {
                *cmd_clicked = true;
            }
        },
        false,
    );

    harness.get_by_label("Click me").key_down(Key::Command);
    harness.get_by_label("Click me").click();
    // TODO(lucasmerlin): Right now the key_up needs to happen on a separate frame or it won't register.
    // This should be more intuitive
    harness.run();
    harness.get_by_label("Click me").key_up(Key::Command);

    harness.run();

    assert!(harness.state(), "The button wasn't command-clicked");
}
