#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use eframe::egui::panel::TopBottomSide;

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
            // The following call is needed to load images when using `ui.image`:
            egui_extras::loaders::install(&cc.egui_ctx);
            Box::<MyApp>::default()
        }),
    )
}

struct MyApp {
    current_uri: String,
    uri_edit_text: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            current_uri: "https://picsum.photos/seed/1.759706314/1024".to_owned(),
            uri_edit_text: "https://picsum.photos/seed/1.759706314/1024".to_owned(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::new(TopBottomSide::Top, "controls").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label("URI:");
                ui.text_edit_singleline(&mut self.uri_edit_text);
                if ui.small_button("✔").clicked() {
                    ctx.forget_image(&self.current_uri);
                    self.uri_edit_text = self.uri_edit_text.trim().to_owned();
                    self.current_uri = self.uri_edit_text.clone();
                };
                if ui.button("file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.uri_edit_text = format!("file://{}", path.display());
                        self.current_uri = self.uri_edit_text.clone();
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_sized(
                ui.available_size(),
                egui::Image::from_uri(&self.current_uri).shrink_to_fit(),
            );
        });
    }
}
