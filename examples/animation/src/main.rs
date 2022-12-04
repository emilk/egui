#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod scaled_color;

use eframe::egui;
use egui::{Color32, Id, Rect, Rounding};
use scaled_color::ScaledColor32;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

struct MyApp {
    rectangle_id: Id,
    animate_direction: bool,
    fade_in_time: f32,
    fade_out_time: f32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            rectangle_id: Id::new("rectangle"),
            animate_direction: false,
            fade_in_time: 0.5,
            fade_out_time: 1.0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Perform the fade in or fade out if required.
            let (required_opacity, time) = if self.animate_direction {
                (1.0, self.fade_in_time)
            } else {
                (0.0, self.fade_out_time)
            };

            let current_opacity =
                ctx.animate_value_with_time(self.rectangle_id, required_opacity, time);

            // Draw a filled rectangle with the current opacity.
            ui.painter().rect_filled(
                Rect::from_min_max(ui.max_rect().min, ui.max_rect().max),
                Rounding::same(15.0),
                Color32::WHITE.scale(current_opacity),
            );

            if ui.button("Animate").clicked() {
                // Switch animation direction.
                self.animate_direction = !self.animate_direction;

                // To ensure smooth transitions with different times, we need to clear the
                // animation manager and then provide it the current value so it knows where to
                // animate from.  This is not necessary if you're using the same times for both
                // fade in and fade out.
                if self.fade_in_time != self.fade_out_time {
                    ctx.clear_animations();
                    ctx.animate_value_with_time(self.rectangle_id, current_opacity, 0.0);
                }

                // Request a repaint to start the animation.
                ctx.request_repaint();
            }
        });
    }
}
