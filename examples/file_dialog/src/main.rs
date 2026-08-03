#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 240.0]) // wide enough for the drag-drop overlay text
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Native file dialogs and drag-and-drop files",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

#[derive(Default)]
struct MyApp {
    dropped_files: Vec<egui::DroppedFileHandle>,
    picked_path: Option<String>,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Drag-and-drop files onto the window!");

            if ui.button("Open file…").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_file()
            {
                self.picked_path = Some(path.display().to_string());
            }

            if let Some(picked_path) = &self.picked_path {
                ui.horizontal(|ui| {
                    ui.label("Picked file:");
                    ui.monospace(picked_path);
                });
            }

            // Show dropped files (if any):
            if !self.dropped_files.is_empty() {
                ui.group(|ui| {
                    ui.label("Dropped files:");

                    for file in &self.dropped_files {
                        #[cfg(not(target_arch = "wasm32"))]
                        ui.label(file.path().display().to_string());

                        #[cfg(target_arch = "wasm32")]
                        {
                            let Some(web_file) = file.web_file() else {
                                continue;
                            };
                            let name = web_file.name();
                            let mime = web_file.type_();
                            let size = web_file.size();
                            if mime.is_empty() {
                                ui.label(format!("{name} ({size} bytes)"));
                            } else {
                                ui.label(format!("{name} (type: {mime}, {size} bytes)"));
                            }
                        }
                    }
                });
            }
        });

        preview_files_being_dropped(ui.ctx());

        // Collect dropped files:
        ui.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.dropped_files.clone_from(&i.raw.dropped_files);
            }
        });
    }
}

/// Preview hovering files:
fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "Dropping files:\n".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(text, "\n{}", path.display()).ok();
                } else if file.mime.is_empty() {
                    text += "\n???";
                } else {
                    write!(text, "\n{}", file.mime).ok();
                }
            }
            text
        });

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let content_rect = ctx.content_rect();
        painter.rect_filled(content_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            content_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.global_style()),
            Color32::WHITE,
        );
    }
}
