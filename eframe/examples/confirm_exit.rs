#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{egui, epi};

#[derive(Default)]
struct MyApp {
    can_exit: bool,
    is_exiting: bool,
}

impl epi::App for MyApp {
    fn name(&self) -> &str {
        "Confirm exit"
    }

    fn on_exit_event(&mut self) -> bool {
        self.is_exiting = true;
        self.can_exit
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Try to close the window");
        });

        if self.is_exiting {
            egui::Window::new("Do you want to quit?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Yes!").clicked() {
                            self.can_exit = true;
                            frame.quit();
                        }

                        if ui.button("Not yet").clicked() {
                            self.is_exiting = false;
                        }
                    });
                });
        }
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(MyApp::default()), options);
}
