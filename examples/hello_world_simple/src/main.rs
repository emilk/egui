#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::process::exit;

use eframe::egui::{self, Color32, FontId, Label, RichText, Sense, Stroke, style::WidgetVisuals};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    // Our application state:
    let mut name = "Arthur".to_owned();
    let mut age = 42;

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.ctx().style_mut(|s| {
                s.visuals.widgets.inactive = WidgetVisuals {
                    fg_stroke: Stroke::new(1.0, Color32::LIGHT_GRAY),
                    ..s.visuals.widgets.inactive
                };
                s.visuals.widgets.active = WidgetVisuals {
                    fg_stroke: Stroke::new(1.0, Color32::LIGHT_BLUE),
                    ..s.visuals.widgets.inactive
                };
                s.visuals.widgets.hovered = WidgetVisuals {
                    fg_stroke: Stroke::new(1.0, Color32::YELLOW),
                    ..s.visuals.widgets.inactive
                };
                s.visuals.widgets.noninteractive = WidgetVisuals {
                    fg_stroke: Stroke::new(1.0, Color32::RED),
                    ..s.visuals.widgets.inactive
                };
            });

            // ui.heading("My egui Application");
            // ui.horizontal(|ui| {
            //     let name_label = ui.label("Your name: ");
            //     ui.text_edit_singleline(&mut name)
            //         .labelled_by(name_label.id);
            // });
            // ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
            // if ui.button("Increment").clicked() {
            //     age += 1;
            // }

            // Button test
            // ui.add(Button::new("no frame").frame(false));
            // ui.add(Button::new("small").small());
            // ui.add_enabled(false, Button::new("disabled"));
            // ui.add(Button::new("no frame inactive").frame_when_inactive(false));

            ui.label("Normal text");
            // Should not be affected by WidgetStyle
            ui.label(
                RichText::new("Unaffected by style")
                    .font(FontId::monospace(15.0))
                    .color(Color32::KHAKI),
            );

            ui.add(Label::new("interaction click").sense(Sense::click()));
            ui.add(Label::new("focusable").sense(Sense::focusable_noninteractive()))
                .request_focus();
        });
    })
}
