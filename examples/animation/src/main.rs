#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::{color, Color32, Id, Rect, Rounding};
use std::ops::Range;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            let app = MyApp::default();
            // Set the initial opacity of the rectangle to write the current value to memory in
            // the animation manager.
            cc.egui_ctx
                .animate_value_with_time(app.rectangle_id, app.current_opacity, 0.0);
            Box::new(app)
        }),
    );
}

#[derive(PartialEq)]
enum AnimateDirection {
    FadeIn,
    FadeOut,
}

struct MyApp {
    rectangle_id: Id,
    opacity_range: Range<f32>,
    animate_direction: AnimateDirection,
    fade_in_time: f32,
    fade_out_time: f32,
    current_opacity: f32,
}

impl Default for MyApp {
    fn default() -> Self {
        let opacity_range = Range {
            start: 0.0,
            end: 1.0,
        };
        let animate_direction = AnimateDirection::FadeOut;
        let current_opacity = match animate_direction {
            AnimateDirection::FadeOut => opacity_range.start,
            AnimateDirection::FadeIn => opacity_range.end,
        };
        Self {
            rectangle_id: Id::new("rectangle"),
            opacity_range,
            animate_direction,
            fade_in_time: 0.5,
            fade_out_time: 1.0,
            current_opacity,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Draw the filled rectangle with the current curved opacity value.  You may use
            // self.current_opacity directly if you want a linear fade.
            let curved_opacity = curve(self.current_opacity, 50.0);
            ui.painter().rect_filled(
                Rect::from_min_max(ui.max_rect().min, ui.max_rect().max),
                Rounding::same(15.0),
                Color32::from_white_alpha(color::linear_u8_from_linear_f32(curved_opacity)),
            );

            if ui.button("Animate").clicked() {
                // Switch animation direction.
                self.animate_direction = match self.animate_direction {
                    AnimateDirection::FadeIn => AnimateDirection::FadeOut,
                    AnimateDirection::FadeOut => AnimateDirection::FadeIn,
                };

                // To ensure smooth transitions with different times, we need to clear the
                // animation manager and then provide it the current value so it knows where to
                // animate from.  This is not necessary if you're using the same times for both
                // fade in and fade out.
                if self.fade_in_time != self.fade_out_time {
                    ctx.clear_animations();
                    ctx.animate_value_with_time(self.rectangle_id, self.current_opacity, 0.0);
                }

                // Request a repaint to start the animation if necessary.
                ctx.request_repaint();
            }

            // Perform the fade in or fade out if required.
            if self.animate_direction == AnimateDirection::FadeIn
                && self.current_opacity != self.opacity_range.end
            {
                self.current_opacity = ctx.animate_value_with_time(
                    self.rectangle_id,
                    self.opacity_range.end,
                    self.fade_in_time,
                );
            } else if self.animate_direction == AnimateDirection::FadeOut
                && self.current_opacity != self.opacity_range.start
            {
                self.current_opacity = ctx.animate_value_with_time(
                    self.rectangle_id,
                    self.opacity_range.start,
                    self.fade_out_time,
                );
            }
        });
    }
}

// Creates an exponential curve with a given steepness (a) for x values between 0.0 and 1.0.
fn curve(x: f32, a: f32) -> f32 {
    // Curve algorithm pulled from the following post:
    // https://math.stackexchange.com/questions/384613/exponential-function-with-values-between-0-and-1-for-x-values-between-0-and-1

    // Values must be greater than 1.0 to work in this algorithm so we assume a linear curve for
    // 1.0 or below.
    if a <= 1.0 {
        return x;
    }

    (a.powf(x) - 1.0) / (a - 1.0)
}
