#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Scene circle placer",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

struct MyApp {
    scene_rect: egui::Rect,
    circles_in_scene: Vec<egui::Pos2>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            scene_rect: egui::Rect::from_min_size(
                egui::Pos2 { x: 0.0, y: 0.0 },
                egui::Vec2 { x: 400.0, y: 400.0 },
            ),
            circles_in_scene: Vec::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut mouse_position_in_scene: Option<egui::Pos2> = Option::None;

            egui::Scene::new().show(
                ui,
                &mut self.scene_rect,
                &mut mouse_position_in_scene,
                |ui| {
                    for circle_pos in &self.circles_in_scene {
                        ui.painter().circle(
                            *circle_pos,
                            10.0,
                            egui::Color32::YELLOW,
                            egui::Stroke::NONE,
                        );
                    }
                },
            );

            let clicked_mouse = ui.input(|i| i.pointer.primary_clicked());

            if mouse_position_in_scene.is_some() && clicked_mouse {
                let new_circle_position = mouse_position_in_scene.unwrap();
                self.circles_in_scene.push(new_circle_position);
            }
        });
    }
}
