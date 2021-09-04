use eframe::{
    egui::{self, CtxRef, TextureId, Vec2},
    epi,
    epi::{Frame, Storage},
};
use image::{DynamicImage, GenericImageView};

pub struct DisplayImage {
    texture_id: TextureId,
    size: Vec2,
}
impl DisplayImage {
    pub fn new(texture_id: TextureId, size: Vec2) -> DisplayImage {
        DisplayImage { texture_id, size }
    }
}
pub struct App {
    name: String,
    dynamic_image: DynamicImage,
    display_image: DisplayImage,
}
impl App {
    pub fn new(dynamic_image: DynamicImage) -> Self {
        App {
            name: String::from("EGUI image example"),
            dynamic_image,
            display_image: DisplayImage::new(egui::TextureId::Egui, Vec2::default()),
        }
    }
}
impl epi::App for App {
    fn name(&self) -> &str {
        &self.name
    }
    fn setup(&mut self, _ctx: &CtxRef, frame: &mut Frame<'_>, _storage: Option<&dyn Storage>) {
        let Self { dynamic_image, .. } = self;
        let size = Vec2::from([dynamic_image.width() as f32, dynamic_image.height() as f32]);
        let pixels = {
            let image_bytes = dynamic_image.as_bytes();
            let mut pixels = Vec::with_capacity(image_bytes.len() / 4);
            for rgba in image_bytes.chunks_exact(4) {
                match rgba {
                    &[r, g, b, a] => pixels.push(egui::Color32::from_rgba_unmultiplied(r, g, b, a)),
                    _ => (),
                }
            }
            pixels
        };
        let image_texture_id = frame.tex_allocator().alloc_srgba_premultiplied(
            (
                dynamic_image.width() as usize,
                dynamic_image.height() as usize,
            ),
            &*pixels,
        );
        self.display_image = DisplayImage::new(image_texture_id, size);
    }
    fn update(&mut self, ctx: &CtxRef, _frame: &mut Frame<'_>) {
        let Self { display_image, .. } = self;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.image(display_image.texture_id, display_image.size);
        });
    }
}
fn main() {
    let dynamic_image = image::open("media/egui-0.10-plot.gif").expect("failed to open image file");
    let window_size = Vec2::from(&[
        (dynamic_image.width() + 15) as f32,
        (dynamic_image.height() + 15) as f32,
    ]);
    let app = App::new(dynamic_image);
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Option::from(window_size);

    eframe::run_native(Box::new(app), native_options);
}
