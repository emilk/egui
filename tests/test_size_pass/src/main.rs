#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's a test

use eframe::egui;

fn main() -> eframe::Result<()> {
    env_logger::init(); // Use `RUST_LOG=debug` to see logs.

    let options = eframe::NativeOptions::default();
    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Reset egui memory").clicked() {
                ctx.memory_mut(|mem| *mem = Default::default());
            }

            ui.separator();

            ui.label("The menu should be as wide as the widest button");
            ui.menu_button("Click for menu", |ui| {
                let _ = ui.button("Narrow").clicked();
                let _ = ui.button("Very wide text").clicked();
                let _ = ui.button("Narrow").clicked();
            });

            ui.label("Hover for tooltip").on_hover_ui(|ui| {
                ui.label("A separator:");
                ui.separator();
            });

            ui.separator();

            let alternatives = [
                "Short",
                "Min",
                "Very very long text that will extend",
                "Short",
            ];
            let mut selected = 1;

            egui::ComboBox::from_label("ComboBox").show_index(
                ui,
                &mut selected,
                alternatives.len(),
                |i| alternatives[i],
            );
        });
    })
}
