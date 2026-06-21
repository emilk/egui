#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::widget_style::WidgetStyle;
use eframe::egui::{
    self, Button, Frame, Margin, Panel, UiBuilder,
    widget_style::{ButtonStyle, HasClasses as _},
};

use crate::custom_engine::ESSEngine;

mod custom_engine;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    let mut style_code = "
.red {
    fill: #f00;
}

.blue {
    fill: #00f;
    border: 10;
}
"
    .to_owned();

    let mut toggled = false;

    eframe::run_ui_native("My egui App", options, move |ui, _frame| {
        // Register the theme plugin and which style they implement
        if let Ok(engine) = ESSEngine::try_parse(&style_code) {
            ui.add_theme::<WidgetStyle>(engine.clone());
            ui.add_theme::<ButtonStyle>(engine);
        }

        ui.scope_builder(UiBuilder::new().with_class("body"), |ui| {
            ui.label("body");
            ui.label("central panel");

            Panel::left("style_code").show(ui, |ui| {
                ui.scope_builder(UiBuilder::new().with_class("panel_left"), |ui| {
                    ui.label(
                        "Live editor\n(type color hex to change the color of the dynamic button)",
                    );

                    if ui.text_edit_multiline(&mut style_code).changed()
                        && let Ok(engine) = ESSEngine::try_parse(&style_code)
                    {
                        // Overwrite the current theme with the new one.clear
                        ui.replace_theme::<WidgetStyle>(engine.clone());
                        ui.replace_theme::<ButtonStyle>(engine);
                    }
                });
            });

            ui.scope_builder(UiBuilder::new().with_class("grid"), |ui| {
                Frame::new().inner_margin(Margin::same(10)).show(ui, |ui| {
                    ui.scope_builder(UiBuilder::new().with_class("frame1"), |ui| {
                        let mut parent = Some(ui.stack());
                        let mut text = vec![];
                        let mut i: i32 = 0;
                        while let Some(p) = parent {
                            text.push(format!(
                                "{}{}class : '{}', kind : {:?}",
                                " ".repeat((2 * 0_i32.max(i - 1) + 1.min(i)) as usize),
                                if i > 0 { "\\- " } else { "" },
                                p.classes,
                                p.kind()
                            ));
                            i += 1;
                            parent = p.parent.as_ref();
                        }
                        ui.label(format!(
                            "Current hierarchy (child to root):\n{}",
                            text.join("\n")
                        ));
                    })
                })
            });

            ui.add(Button::new("Normal"));
            ui.add(Button::new("red").with_class("red"));
            ui.add(Button::new("blue").with_class("blue"));
            ui.add(Button::new("dynamic in engine A").with_class("dynamic"));
            if ui
                .add(
                    Button::new("red/blue")
                        .with_class_if("red", toggled)
                        .with_class_if("blue", !toggled),
                )
                .clicked()
            {
                toggled = !toggled;
            }
        });
    })
}
