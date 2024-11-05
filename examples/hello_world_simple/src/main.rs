#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eframe::egui::modal::Modal;
use eframe::egui::{Align, ComboBox, Id, Layout, ProgressBar, Widget};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let mut save_modal_open = false;
    let mut user_modal_open = false;
    let mut save_progress = None;

    let roles = ["user", "admin"];
    let mut role = roles[0];

    let mut name = "John Doe".to_string();

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Open Modal A").clicked() {
                save_modal_open = true;
            }

            if ui.button("Open Modal B").clicked() {
                user_modal_open = true;
            }

            if save_modal_open {
                let modal = Modal::new(Id::new("Modal A")).show(ui.ctx(), |ui| {
                    ui.set_width(250.0);

                    ui.heading("Edit User");

                    ui.label("Name:");
                    ui.text_edit_singleline(&mut name);

                    ComboBox::new("role", "Role")
                        .selected_text(role)
                        .show_ui(ui, |ui| {
                            for r in &roles {
                                ui.selectable_value(&mut role, r, *r);
                            }
                        });

                    ui.separator();

                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if ui.button("Save").clicked() {
                            user_modal_open = true;
                        }
                        if ui.button("Cancel").clicked() {
                            save_modal_open = false;
                        }
                    });
                });

                if modal.backdrop_response.clicked() {
                    save_modal_open = false;
                }
            }

            if user_modal_open {
                let modal = Modal::new(Id::new("Modal B")).show(ui.ctx(), |ui| {
                    ui.set_width(200.0);
                    ui.heading("Save? Are you sure?");

                    ui.add_space(32.0);

                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if ui.button("Yes Please").clicked() {
                            save_progress = Some(0.0);
                        }

                        if ui.button("No Thanks").clicked() {
                            user_modal_open = false;
                        }
                    });
                });

                if modal.backdrop_response.clicked() {
                    user_modal_open = false;
                }
            }

            if let Some(progress) = save_progress {
                let modal = Modal::new(Id::new("Modal C")).show(ui.ctx(), |ui| {
                    ui.set_width(70.0);
                    ui.heading("Saving...");

                    ProgressBar::new(progress).ui(ui);

                    if progress >= 1.0 {
                        save_progress = None;
                        user_modal_open = false;
                        save_modal_open = false;
                    } else {
                        save_progress = Some(progress + 0.003);
                        ui.ctx().request_repaint();
                    }
                });
            }
        });

        egui::Window::new("My Window").show(ctx, |ui| {
            if ui.button("show modal").clicked() {
                user_modal_open = true;
            }
        });
    })
}
