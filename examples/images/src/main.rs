#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 880.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::default())
        }),
    )
}

#[derive(Default)]
struct MyApp {}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.image(egui::include_image!("cat.webp"))
                    .on_hover_text_at_pointer("WebP");
                ui.image(egui::include_image!("ferris.gif"))
                    .on_hover_text_at_pointer("Gif");
                ui.image(egui::include_image!("ferris.svg"))
                    .on_hover_text_at_pointer("Svg");

                let url = "https://picsum.photos/seed/1.759706314/1024";
                ui.add(egui::Image::new(url).corner_radius(10))
                    .on_hover_text_at_pointer(url);
            });
        });
    }
}
