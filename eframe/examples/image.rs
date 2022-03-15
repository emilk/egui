#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui_extras::RetainedImage;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Show an image with eframe/egui", options, |_cc| {
        Box::new(MyApp::default())
    });
}

struct MyApp {
    image: RetainedImage,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            image: RetainedImage::from_image_bytes(
                "rust-logo-256x256.png",
                include_bytes!("rust-logo-256x256.png"),
            )
            .unwrap(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("This is an image:");
            self.image.show(ui);

            ui.heading("This is an image you can click:");
            ui.add(egui::ImageButton::new(
                self.image.texture_id(ctx),
                self.image.size_vec2(),
            ));
        });
    }
}
