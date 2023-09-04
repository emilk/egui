//! A good way of displaying an SVG image in egui.
//!
//! Requires the dependency `egui_extras` with the `svg` feature.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1000.0, 700.0)),
        ..Default::default()
    };
    eframe::run_native(
        "svg example",
        options,
        Box::new(|cc| {
            // Without the following call, the `Image2` created below
            // will simply output a `not supported` error message.
            egui_extras::loaders::install(&cc.egui_ctx);
            Box::new(MyApp)
        }),
    )
}

struct MyApp;

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SVG example");
            ui.label("The SVG is rasterized and displayed as a texture.");

            ui.separator();

            let max_size = ui.available_size();
            ui.add(
                egui::Image2::from_static_bytes(
                    "ferris.svg",
                    include_bytes!("rustacean-flat-happy.svg"),
                )
                .size_hint(max_size),
            );
        });
    }
}
