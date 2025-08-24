#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eframe::egui::{
    Color32, ColorImage, FontFamily, FontId, FontSelection, RichText, TextEdit,
    global_theme_preference_switch, include_image,
};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<MyApp>::default())
        }),
    )
}

struct MyApp {
    name: String,
    age: u32,
}

fn font_id() -> FontId {
    FontId::new(30.0, FontFamily::Proportional)
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Ferris :crab:".to_owned(),
            age: 42,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            global_theme_preference_switch(ui);

            ctx.fonts(|f| {
                let mut fonts = f.lock();
                let font = fonts.fonts.font(&font_id());
                if !font.has_glyph('🦀') {
                    let image = include_bytes!("../../../crates/egui/assets/ferris.png");

                    let image = egui_extras::image::load_image_bytes(image).unwrap();

                    font.allocate_custom_glyph('🦀', image);
                }
            });

            TextEdit::singleline(&mut self.name)
                .font(FontSelection::FontId(font_id()))
                .text_color(Color32::WHITE)
                .show(ui);

            self.name = self.name.replace(":crab:", "🦀");

            ui.label(RichText::new(&self.name).font(font_id()).strong());
        });
    }
}
