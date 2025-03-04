#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eframe::egui::{Align, Button, Layout, Popup, TextWrapMode, Widget};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    // Our application state:
    let mut name = "Arthur".to_owned();
    let mut age = 42;

    let mut checked = true;

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        ctx.options_mut(|o| o.max_passes = 1.try_into().unwrap());
        egui::CentralPanel::default().show(ctx, |ui| {
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
            // ui.label(format!("Hello '{name}', age {age}"));

            std::thread::sleep(std::time::Duration::from_millis(100));
            let response = ui.button("Hiiii");

            let text = if checked {
                "short"
            } else {
                "Very long text for this item that should be wrapped"
            };
            Popup::from_response(&response)
                .layout(Layout::top_down_justified(Align::Min))
                .show(|ui| {
                    // ui.checkbox(&mut checked, text);

                    ui.button("Hiiii");

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            if Button::new(text)
                                // .wrap_mode(TextWrapMode::Extend)
                                .ui(ui)
                                .clicked()
                            {
                                checked = !checked;
                            }
                            ui.button("Button1");
                        });

                        ui.button("Button2")
                    });

                    ui.button("Some other button");
                });
        });
    })
}
