#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{
    egui::{self, ColorImage},
    glow::{self, HasContext},
};
use itertools::Itertools as _;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Take screenshots and display with eframe/egui",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    )
}

#[derive(Default)]
struct MyApp {
    continuously_take_screenshots: bool,
    take_screenshot: bool,
    texture: Option<egui::TextureHandle>,
    screenshot: Option<ColorImage>,
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
                    } else if ui.button("take screenshot!").clicked() {
                        self.take_screenshot = true;
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
    fn post_rendering(&mut self, screen_size_px: [u32; 2], frame: &eframe::Frame) {
        if !self.take_screenshot && !self.continuously_take_screenshots {
            return;
        }

        self.take_screenshot = false;
        if let Some(gl) = frame.gl() {
            let [w, h] = screen_size_px;
            let mut buf = vec![0u8; w as usize * h as usize * 4];
            let pixels = glow::PixelPackData::Slice(&mut buf[..]);
            unsafe {
                gl.read_pixels(
                    0,
                    0,
                    w as i32,
                    h as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    pixels,
                );
            }

            // Flip vertically:
            let mut rows: Vec<Vec<u8>> = buf
                .into_iter()
                .chunks(w as usize * 4)
                .into_iter()
                .map(|chunk| chunk.collect())
                .collect();
            rows.reverse();
            let buf: Vec<u8> = rows.into_iter().flatten().collect();

            self.screenshot = Some(ColorImage::from_rgba_unmultiplied(
                [screen_size_px[0] as usize, screen_size_px[1] as usize],
                &buf[..],
            ));
        }
    }
}
