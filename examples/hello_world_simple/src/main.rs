#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    // Our application state:
    let mut name = "Arthur".to_owned();
    let mut age = 42;

    let mut window1_embedded = true;
    let mut window2_embedded = true;

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!(
                "Current window: {}, Current rendering window: {}",
                ctx.current_viewport(),
                ctx.current_rendering_viewport()
            ));
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
            if ui.button("Click each year").clicked() {
                age += 1;
            }
            ui.label(format!("Hello '{name}', age {age}"));
            egui::CollapsingHeader::new("Show Test1").show(ui, |ui| {
                egui::Window::new("Test1")
                    .embedded(window1_embedded)
                    .show(ctx, |ui| {
                        ui.checkbox(&mut window1_embedded, "Should embedd?");
                        ui.label(format!(
                            "Current window: {}, Current rendering window: {}",
                            ctx.current_viewport(),
                            ctx.current_rendering_viewport()
                        ));
                    });
            });
            egui::CollapsingHeader::new("Shout Test2").show(ui, |ui| {
                egui::Window::new("Test2")
                    .embedded(window2_embedded)
                    .show(ctx, |ui| {
                        ui.checkbox(&mut window2_embedded, "Should embedd?");
                        ui.label(format!(
                            "Current window: {}, Current rendering window: {}",
                            ctx.current_viewport(),
                            ctx.current_rendering_viewport()
                        ));
                    });
            });
        });
    })
}
