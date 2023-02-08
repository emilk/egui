// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui::{self, ColorImage};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Take screenshots and display with eframe/egui",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    )
}

#[derive(Default)]
struct MyApp {
    continuously_take_screenshots: bool,
    texture: Option<egui::TextureHandle>,
    screenshot: Option<ColorImage>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
                        frame.request_pixels();
                    } else if ui.button("take screenshot!").clicked() {
                        frame.request_pixels();
                    }
                });
            });

            if let Some(texture) = self.texture.as_ref() {
                ui.image(texture, ui.available_size());
            } else {
                ui.spinner();
            }

            ctx.request_repaint();
        });
    }

    #[allow(unsafe_code)]
    fn post_rendering(&mut self, _screen_size_px: [u32; 2], frame: &eframe::Frame) {
        if let Some(pixels) = frame.frame_pixels() {
            self.screenshot = Some(pixels)
        }
    }
}
