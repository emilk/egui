#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use egui::{Key, ScrollArea};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Keyboard events",
        options,
        Box::new(|_cc| Ok(Box::<Content>::default())),
    )
}

#[derive(Default)]
struct Content {
    text: String,
}

impl eframe::App for Content {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Press/Hold/Release example. Press A to test.");
            if ui.button("Clear").clicked() {
                self.text.clear();
            }
            ScrollArea::vertical()
                .auto_shrink(false)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.label(&self.text);
                });

            if ui.input(|i| i.key_pressed(Key::A)) {
                self.text.push_str("\nPressed");
            }
            if ui.input(|i| i.key_down(Key::A)) {
                self.text.push_str("\nHeld");
                ui.request_repaint(); // make sure we note the holding.
            }
            if ui.input(|i| i.key_released(Key::A)) {
                self.text.push_str("\nReleased");
            }
        });
    }
}
