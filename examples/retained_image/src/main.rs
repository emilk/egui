#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui_extras::RetainedImage;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(400.0, 1000.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Show an image with eframe/egui",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    image: RetainedImage,
    rounding: f32,
    tint: egui::Color32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            // crab image is CC0, found on https://stocksnap.io/search/crab
            image: RetainedImage::from_image_bytes("crab.png", include_bytes!("crab.png")).unwrap(),
            rounding: 32.0,
            tint: egui::Color32::from_rgb(100, 200, 200),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            image,
            rounding,
            tint,
        } = self;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("This is an image:");
            image.show(ui);

            ui.add_space(32.0);

            ui.heading("This is a tinted image with rounded corners:");
            ui.add(
                egui::Image::new(image.texture_id(ctx), image.size_vec2())
                    .tint(*tint)
                    .rounding(*rounding),
            );

            ui.horizontal(|ui| {
                ui.label("Tint:");
                egui::color_picker::color_edit_button_srgba(
                    ui,
                    tint,
                    egui::color_picker::Alpha::BlendOrAdditive,
                );

                ui.add_space(16.0);

                ui.label("Rounding:");
                ui.add(
                    egui::DragValue::new(rounding)
                        .speed(1.0)
                        .clamp_range(0.0..=0.5 * image.size_vec2().min_elem()),
                );
            });

            ui.add_space(32.0);

            ui.heading("This is an image you can click:");
            ui.add(egui::ImageButton::new(
                image.texture_id(ctx),
                image.size_vec2(),
            ));
        });
    }
}
