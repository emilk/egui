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
            egui_extras::loaders::install(&cc.egui_ctx);
            Box::<MyApp>::default()
        }),
    )
}

struct MyApp {
    svg_image: egui_extras::RetainedImage,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            svg_image: egui_extras::RetainedImage::from_svg_bytes_with_size(
                "rustacean-flat-happy.svg",
                include_bytes!("rustacean-flat-happy.svg"),
                egui_extras::image::FitTo::Original,
            )
            .unwrap(),
        }
    }
}

const URI: &str = concat!(
    "file://",
    env!("CARGO_MANIFEST_DIR"),
    "/src/rustacean-flat-happy.svg"
);

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SVG example");
            ui.label("The SVG is rasterized and displayed as a texture.");

            ui.separator();

            let max_size = ui.available_size();
            ui.add(egui::Image2::from_uri(URI).size_hint(max_size));
            // self.svg_image.show_size(ui, max_size);
        });
    }
}
