use eframe::{egui, epi};

#[derive(Default)]
struct MyApp {
    texture: Option<(egui::Vec2, egui::TextureId)>,
}

impl epi::App for MyApp {
    fn name(&self) -> &str {
        "Show an image with eframe/egui"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        if self.texture.is_none() {
            // Load the image:
            let image_data = include_bytes!("rust-logo-256x256.png");
            use image::GenericImageView;
            let image = image::load_from_memory(image_data).expect("Failed to load image");
            let image_buffer = image.to_rgba8();
            let size = (image.width() as usize, image.height() as usize);
            let pixels = image_buffer.into_vec();
            assert_eq!(size.0 * size.1 * 4, pixels.len());
            let pixels: Vec<_> = pixels
                .chunks_exact(4)
                .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                .collect();

            // Allocate a texture:
            let texture = frame.tex_allocator().alloc(size, &pixels);
            let size = egui::Vec2::new(size.0 as f32, size.1 as f32);
            self.texture = Some((size, texture));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some((size, texture)) = self.texture {
                ui.heading("This is an image:");
                ui.image(texture, size);

                ui.heading("This is an image you can click:");
                ui.add(egui::ImageButton::new(texture, size));
            }
        });
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(MyApp::default()), options);
}
