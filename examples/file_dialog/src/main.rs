#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
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
    dropped_files: Vec<egui::DroppedFile>,
    picker: Picker,
    picked_path: Option<String>,
}

#[derive(Default)]
enum PickerState {
    #[default]
    Pending,
    // use a boolean to indicate of picking has completed
    Picking(Arc<Mutex<(bool, Option<PathBuf>)>>),
}

#[derive(Default)]
struct Picker {
    state: PickerState,
}

impl Picker {
    pub fn is_picking(&self) -> bool {
        matches!(self.state, PickerState::Picking(_))
    }

    pub fn pick(&mut self) {
        let picker = Arc::new(Mutex::new((false, None)));
        self.state = PickerState::Picking(picker.clone());
        std::thread::spawn(move || {
            let mut guard = picker.lock().unwrap();
            *guard = (true, rfd::FileDialog::new()
                .set_directory("/")
                .pick_file()
                .map(std::path::PathBuf::from));
        });
    }

    /// when picked, returns true, and the result of the pick, which may be None
    /// otherwise returns false
    pub fn picked(&mut self) -> (bool, Option<PathBuf>) {

        let mut was_picked = false;

        let return_value = match &mut self.state {
            PickerState::Picking(arc) => {
                if let Ok(mut guard) = arc.try_lock() {
                    match &mut *guard {
                        (true, picked) => {
                            was_picked = true;
                            let result = picked.take();
                            (true, result)
                        },
                        (false, _) => (false, None),
                    }
                } else {
                    (false, None)
                }
            }
            _ => (false, None),
        };

        if was_picked {
            self.state = PickerState::Pending;
        }

        return_value
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Drag-and-drop files onto the window!");

            let picking = self.picker.is_picking();

            if ui.add_enabled(!picking, egui::Button::new("Open file…")).clicked() {
                self.picker.pick();
            }

            if let (true, Some(picked_path)) = self.picker.picked() {
                self.picked_path = Some(picked_path.display().to_string());
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
                        let mut info = if let Some(path) = &file.path {
                            path.display().to_string()
                        } else if !file.name.is_empty() {
                            file.name.clone()
                        } else {
                            "???".to_owned()
                        };

                        let mut additional_info = vec![];
                        if !file.mime.is_empty() {
                            additional_info.push(format!("type: {}", file.mime));
                        }
                        if let Some(bytes) = &file.bytes {
                            additional_info.push(format!("{} bytes", bytes.len()));
                        }
                        if !additional_info.is_empty() {
                            info += &format!(" ({})", additional_info.join(", "));
                        }

                        ui.label(info);
                    }
                });
            }
        });

        preview_files_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
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
                } else if !file.mime.is_empty() {
                    write!(text, "\n{}", file.mime).ok();
                } else {
                    text += "\n???";
                }
            }
            text
        });

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}
