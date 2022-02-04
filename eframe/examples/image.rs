#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{egui, epi};

#[derive(Default)]
struct MyApp {
    texture: Option<egui::TextureHandle>,
}

impl epi::App for MyApp {
    fn name(&self) -> &str {
        "Show an image with eframe/egui"
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        let texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
            let image = load_image(include_bytes!("rust-logo-256x256.png")).unwrap();
            ctx.load_texture("rust-logo", image)
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("This is an image:");
            ui.image(texture, texture.size_vec2());

            ui.heading("This is an image you can click:");
            ui.add(egui::ImageButton::new(texture, texture.size_vec2()));
        });
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(MyApp::default()), options);
}

fn load_image(image_data: &[u8]) -> Result<egui::ColorImage, image::ImageError> {
    let image = image::load_from_memory(image_data)?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}
