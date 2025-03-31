#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eframe::egui::{
    include_image, Button, Image, Key, KeyboardShortcut, ModifierNames, Modifiers, Popup, RichText,
    Widget,
};

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
            egui_extras::install_image_loaders(ctx);
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                age += 1;
            }
            ui.label(format!("Hello '{name}', age {age}"));

            if Button::new("WL Button").ui(ui).clicked() {
                age += 1;
            };

            let source = include_image!("../../../crates/eframe/data/icon.png");
            let response = Button::image_and_text(source.clone(), "Hello World").ui(ui);

            Button::new((Image::new(source).tint(egui::Color32::RED), "Tuple Button")).ui(ui);

            Popup::menu(&response).show(|ui| {
                Button::new("Print")
                    .right_text(
                        RichText::new(
                            KeyboardShortcut::new(Modifiers::COMMAND, Key::P)
                                .format(&ModifierNames::SYMBOLS, true),
                        )
                        .weak(),
                    )
                    .ui(ui);
                Button::new("A very long button")
                    .right_text(
                        RichText::new(
                            KeyboardShortcut::new(Modifiers::COMMAND, Key::O)
                                .format(&ModifierNames::SYMBOLS, true),
                        )
                        .weak(),
                    )
                    .ui(ui);
            });
        });
    })
}
