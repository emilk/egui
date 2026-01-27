#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::{self, Button, Frame, Grid, Margin, Panel, widget_style::HasModifiers as _};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    let mut style_code = "// future style code live editor".to_owned();

    eframe::run_ui_native("My egui App", options, move |ui, _frame| {
        // Add a modifier to this ui
        ui.add_modifier("body");
        ui.label("body");
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Add a modifier to this ui
            ui.add_modifier("central panel");
            ui.label("central panel");

            Panel::left("style_code").show_inside(ui, |ui| {
                ui.add_modifier("style code editor");
                ui.label("style code editor");

                ui.text_edit_multiline(&mut style_code);
            });

            Grid::new("grid").show(ui, |ui| {
                ui.add_modifier("grid");

                Frame::new().inner_margin(Margin::same(10)).show(ui, |ui| {
                    ui.add_modifier("frame1");

                    ui.add(Button::new("button1").with_modifier("button1"));
                    ui.add(Button::new("button2").with_modifier("button2"));
                })
            });
        });
    })
}
