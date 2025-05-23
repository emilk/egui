use egui::{Align, AtomicExt, Button, Layout, Ui, Vec2, Widget};
use egui_kittest::HarnessBuilder;

#[test]
fn test_atomics() {
    single_test("max_width_and_grow", |ui| {
        _ = Button::new((
            "hello my name is".atom_max_width(10.0).atom_grow(true),
            "world",
        ))
        .ui(ui);
    });
}

fn single_test(name: &str, mut f: impl FnMut(&mut Ui)) {
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

    // harness.fit_contents();

    harness.snapshot(name);
}
