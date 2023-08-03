#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::sync::{Arc, RwLock};

use eframe::egui::{self, ViewportRender};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Confirm exit",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

#[derive(Default)]
struct MyAppData {
    allowed_to_close: bool,
    show_confirmation_dialog: bool,
}

#[derive(Default)]
struct MyApp {
    data: Arc<RwLock<MyAppData>>,
}

impl eframe::App for MyApp {
    fn on_close_event(&mut self) -> bool {
        self.data.write().unwrap().show_confirmation_dialog = true;
        self.data.read().unwrap().allowed_to_close
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        render: Option<&ViewportRender>,
    ) {
        if let Some(render) = render {
            render(ctx, frame.viewport_id(), frame.parent_viewport_id());
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Try to close the window");
        });

        let show_confirmation_dialog = self.data.read().unwrap().show_confirmation_dialog;
        if show_confirmation_dialog {
            let data = self.data.clone();
            // Show confirmation dialog:
            egui::Window::new("Do you want to quit?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, move |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            data.write().unwrap().show_confirmation_dialog = false;
                        }

                        if ui.button("Yes!").clicked() {
                            data.write().unwrap().allowed_to_close = true;
                        }
                    });
                });
            if self.data.read().unwrap().allowed_to_close {
                frame.close()
            }
        }
    }
}
