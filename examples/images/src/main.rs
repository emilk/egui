#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use eframe::epaint::vec2;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        ..Default::default()
    };
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // The following call is needed to load images when using `ui.image` and `egui::Image`:
            egui_extras::loaders::install(&cc.egui_ctx);
            Box::<MyApp>::default()
        }),
    )
}

#[derive(Default)]
struct MyApp {}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::new([true, true]).show(ui, |ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("ferris.svg").into())
                        .fit_to_fraction(vec2(1.0, 0.5)),
                );
                ui.add(
                    egui::Image::new("https://picsum.photos/seed/1.759706314/1024".into())
                        .rounding(egui::Rounding::same(10.0)),
                );
            });
        });
    }
}
