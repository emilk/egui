#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::sync::Arc;

use eframe::egui::{self, ColorImage};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Take screenshots and display with eframe/egui",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

#[derive(Default)]
struct MyApp {
    continuously_take_screenshots: bool,
    texture: Option<egui::TextureHandle>,
    screenshot: Option<Arc<ColorImage>>,
    save_to_file: bool,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(screenshot) = self.screenshot.take() {
                self.texture = Some(ui.ctx().load_texture(
                    "screenshot",
                    screenshot,
                    Default::default(),
                ));
            }

            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.continuously_take_screenshots,
                    "continuously take screenshots",
                );

                if ui.button("save to 'top_left.png'").clicked() {
                    self.save_to_file = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
                }

                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                    if self.continuously_take_screenshots {
                        if ui
                            .add(egui::Label::new("hover me!").sense(egui::Sense::hover()))
                            .hovered()
                        {
                            ctx.set_visuals(egui::Visuals::dark());
                        } else {
                            ctx.set_visuals(egui::Visuals::light());
                        };
                        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
                    } else if ui.button("take screenshot!").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
                    }
                });
            });

            if let Some(texture) = self.texture.as_ref() {
                ui.image((texture.id(), ui.available_size()));
            } else {
                ui.spinner();
            }

            // Check for returned screenshot:
            ui.input(|i| {
                for event in &i.raw.events {
                    if let egui::Event::Screenshot { image, .. } = event {
                        if self.save_to_file {
                            let pixels_per_point = i.pixels_per_point();
                            let region = egui::Rect::from_two_pos(
                                egui::Pos2::ZERO,
                                egui::Pos2 { x: 100., y: 100. },
                            );
                            let top_left_corner = image.region(&region, Some(pixels_per_point));
                            image::save_buffer(
                                "top_left.png",
                                top_left_corner.as_raw(),
                                top_left_corner.width() as u32,
                                top_left_corner.height() as u32,
                                image::ColorType::Rgba8,
                            )
                            .unwrap();
                            self.save_to_file = false;
                        }
                        self.screenshot = Some(image.clone());
                    }
                }
            });

            ctx.request_repaint();
        });
    }
}
