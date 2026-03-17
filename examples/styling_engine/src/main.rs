#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::{
    self, Frame, Grid, Margin, Panel, UiBuilder,
    widget_style::{Classes, HasClasses as _},
};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    let mut style_code = "// future style code live editor".to_owned();

    eframe::run_ui_native("My egui App", options, move |ui, _frame| {
        ui.scope_builder(
            UiBuilder::new().classes(Classes::default().with_class("body")),
            |ui| {
                ui.label("body");
                ui.label("central panel");

                Panel::left("style_code").show_inside(ui, |ui| {
                    ui.scope_builder(
                        UiBuilder::new().classes(Classes::default().with_class("panel_left")),
                        |ui| {
                            ui.label("style code editor");

                            ui.text_edit_multiline(&mut style_code);
                        },
                    );
                });

                Grid::new("grid").show(ui, |ui| {
                    ui.scope_builder(
                        UiBuilder::new().classes(Classes::default().with_class("grid")),
                        |ui| {
                            Frame::new().inner_margin(Margin::same(10)).show(ui, |ui| {
                                ui.scope_builder(
                                    UiBuilder::new()
                                        .classes(Classes::default().with_class("frame1")),
                                    |ui| {
                                        let mut parent = Some(ui.stack());
                                        let mut text = vec![];
                                        let mut i: i32 = 0;
                                        while let Some(p) = parent {
                                            text.push(format!(
                                                "{}{}class : '{}', kind : {:?}",
                                                " ".repeat(
                                                    (2 * 0_i32.max(i - 1) + 1.min(i)) as usize
                                                ),
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
                                    },
                                )
                            })
                        },
                    );
                });
            },
        );
    })
}
