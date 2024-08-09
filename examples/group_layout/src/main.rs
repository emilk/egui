#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                egui::Group::new("my_group")
                    .frame(egui::Frame::group(ui.style()))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Hello");
                            ui.code("world!");
                        });

                        if ui.button("Reset egui").clicked() {
                            ui.memory_mut(|mem| *mem = Default::default());
                        }
                    });
            });
        });
    })
}
