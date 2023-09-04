#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Download and show an image with eframe/egui",
        options,
        Box::new(|cc| {
            // Without the following call, the `Image2` created below
            // will simply output `not supported` error messages.
            egui_extras::loaders::install(&cc.egui_ctx);
            Box::new(MyApp)
        }),
    )
}

#[derive(Default)]
struct MyApp;

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let width = ui.available_width();
            let half_height = ui.available_height() / 2.0;

            ui.allocate_ui(egui::Vec2::new(width, half_height), |ui| {
                ui.add(egui::Image2::from_uri(
                    "https://picsum.photos/seed/1.759706314/1024",
                ))
            });
            ui.allocate_ui(egui::Vec2::new(width, half_height), |ui| {
                ui.add(egui::Image2::from_uri(
                    "https://this-is-hopefully-not-a-real-website.rs/image.png",
                ))
            });
        });
    }
}
