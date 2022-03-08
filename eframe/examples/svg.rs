//! A good way of displaying an SVG image in egui.
//!
//! Requires the dependency `egui_extras` with the `svg` feature.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{egui, epi};

struct MyApp {
    svg_image: egui_extras::RetainedImage,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            svg_image: egui_extras::RetainedImage::from_svg_bytes(
                "rustacean-flat-happy.svg",
                include_bytes!("rustacean-flat-happy.svg"),
            )
            .unwrap(),
        }
    }
}

impl epi::App for MyApp {
    fn name(&self) -> &str {
        "svg example"
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SVG example");
            ui.label("The SVG is rasterized and displayed as a texture.");

            ui.separator();

            let max_size = ui.available_size();
            self.svg_image.show_max_size(ui, max_size);
        });
    }
}

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1000.0, 700.0)),
        ..Default::default()
    };
    eframe::run_native(Box::new(MyApp::default()), options);
}
