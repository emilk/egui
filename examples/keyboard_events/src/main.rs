#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::*;
fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Keyboard events",
        options,
        Box::new(|_cc| Box::new(Content::default())),
    );
}

struct Content {
    text: String,
}

impl Default for Content {
    fn default() -> Self {
        Self {
            text: "".to_owned(),
        }
    }
}

impl eframe::App for Content {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Press/Hold/Release example");
            let text_style = TextStyle::Body;
            let row_height = ui.text_style_height(&text_style);
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show_rows(ui, row_height, self.text.len(), |ui, _row_range| {
                    //for row in row_range {
                    for line in self.text.lines() {
                        ui.label(line);
                    }
                });

            if ctx.input().key_released(Key::A) {
                self.text.push_str("\nReleased");
            }
            if ctx.input().key_pressed(Key::A) {
                self.text.push_str("\npressed");
            }
            if ctx.input().key_down(Key::A) {
                self.text.push_str("\nheld");
            }
        });
    }
}
