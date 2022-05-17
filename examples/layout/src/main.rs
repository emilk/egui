#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(480., 480.)),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "My egui Layout example",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

struct MyApp {}

impl Default for MyApp {
    fn default() -> Self {
        Self {}
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[derive(Hash)]
        enum MyId {
            ScrollArea1,
            ScrollArea2,
            ScrollArea3,
            ScrollAreaN(usize),
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Layout example");

            ui.add_space(16.);
            egui::ScrollArea::horizontal()
                .auto_shrink([true, true])
                .max_height(16.)
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::left_to_right(), |ui| {
                        for i in 0..8 {
                            ui.label(format!("Horizontal label {}", i));
                        }
                    });
                });

            ui.add_space(16.);
            // Multiple scrollareas require an explicit id with `push_id`
            ui.push_id(MyId::ScrollArea1, |ui| {
                egui::ScrollArea::horizontal()
                    .auto_shrink([true, true])
                    .max_height(16.)
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(), |ui| {
                            for i in 0..8 {
                                ui.label(format!("Horizontal label {}", i));
                            }
                        });
                    });
            });

            ui.add_space(16.);
            ui.push_id(MyId::ScrollArea2, |ui| {
                ui.with_layout(egui::Layout::left_to_right(), |ui| {
                    for i in 0..4 {
                        ui.push_id(MyId::ScrollAreaN(i), |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink([true, false])
                                .max_height(64.)
                                .show(ui, |ui| {
                                    // Layouts can be easily nested
                                    ui.with_layout(
                                        egui::Layout::top_down(egui::Align::LEFT),
                                        |ui| {
                                            for i in 0..8 {
                                                ui.label(format!("Vertical label {}", i));
                                            }
                                        },
                                    );
                                });
                        });
                    }
                });
            });

            ui.push_id(MyId::ScrollArea3, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(128.)
                    .show(ui, |ui| {
                        // This vertical layout has centered and shrunk objects
                        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                            for i in 0..4 {
                                if ui.button(format!("Centered button {}", i)).clicked() {
                                    println!("Clicked {}!", i)
                                };
                            }
                        });

                        ui.with_layout(
                            egui::Layout::top_down_justified(egui::Align::Center),
                            |ui| {
                                for i in 0..4 {
                                    if ui
                                        .button(format!("Centered&justified button {}", i))
                                        .clicked()
                                    {
                                        println!("Clicked {}!", i)
                                    };
                                }
                            },
                        );

                        ui.with_layout(
                            egui::Layout::top_down_justified(egui::Align::RIGHT),
                            |ui| {
                                for i in 0..4 {
                                    if ui.button(format!("Right&justified button {}", i)).clicked()
                                    {
                                        println!("Clicked {}!", i)
                                    };
                                }
                            },
                        );

                        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            for i in 0..4 {
                                if ui.button(format!("Left button {}", i)).clicked() {
                                    println!("Clicked {}!", i)
                                };
                            }
                        });
                    });
            });
        });
    }
}
