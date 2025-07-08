use egui::{Align, AtomExt as _, Button, Layout, TextWrapMode, Ui, Vec2};
use egui_kittest::{HarnessBuilder, SnapshotResult, SnapshotResults};

#[test]
fn test_atoms() {
    let mut results = SnapshotResults::new();

    results.add(single_test("max_width", |ui| {
        ui.add(Button::new((
            "max width not grow".atom_max_width(30.0),
            "other text",
        )));
    }));
    results.add(single_test("max_width_and_grow", |ui| {
        ui.add(Button::new((
            "max width and grow".atom_max_width(30.0).atom_grow(true),
            "other text",
        )));
    }));
    results.add(single_test("shrink_first_text", |ui| {
        ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
        ui.add(Button::new(("this should shrink", "this shouldn't")));
    }));
    results.add(single_test("shrink_last_text", |ui| {
        ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
        ui.add(Button::new((
            "this shouldn't shrink",
            "this should".atom_shrink(true),
        )));
    }));
    results.add(single_test("grow_all", |ui| {
        ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
        ui.add(Button::new((
            "I grow".atom_grow(true),
            "I also grow".atom_grow(true),
            "I grow as well".atom_grow(true),
        )));
    }));
    results.add(single_test("size_max_size", |ui| {
        ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
        ui.add(Button::new((
            "size and max size"
                .atom_size(Vec2::new(80.0, 80.0))
                .atom_max_size(Vec2::new(20.0, 20.0)),
            "other text".atom_grow(true),
        )));
    }));
}

fn single_test(name: &str, mut f: impl FnMut(&mut Ui)) -> SnapshotResult {
    let mut harness = HarnessBuilder::default()
        .with_size(Vec2::new(400.0, 200.0))
        .build_ui(move |ui| {
            ui.label("Normal");
            let normal_width = ui.horizontal(&mut f).response.rect.width();

            ui.label("Justified");
            ui.with_layout(
                Layout::left_to_right(Align::Min).with_main_justify(true),
                &mut f,
            );

            ui.label("Shrunk");
            ui.scope(|ui| {
                ui.set_max_width(normal_width / 2.0);
                f(ui);
            });
        });

    harness.try_snapshot(name)
}

#[test]
fn test_intrinsic_size() {
    let mut intrinsic_size = None;
    for wrapping in [
        TextWrapMode::Extend,
        TextWrapMode::Wrap,
        TextWrapMode::Truncate,
    ] {
        _ = HarnessBuilder::default()
            .with_size(Vec2::new(100.0, 100.0))
            .build_ui(|ui| {
                ui.style_mut().wrap_mode = Some(wrapping);
                let response = ui.add(Button::new(
                    "Hello world this is a long text that should be wrapped.",
                ));
                if let Some(current_intrinsic_size) = intrinsic_size {
                    assert_eq!(
                        Some(current_intrinsic_size),
                        response.intrinsic_size,
                        "For wrapping: {wrapping:?}"
                    );
                }
                intrinsic_size = response.intrinsic_size;
            });
    }
    assert_eq!(intrinsic_size.unwrap().round(), Vec2::new(305.0, 18.0));
}
