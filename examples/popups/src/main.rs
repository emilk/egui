#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::*;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions::default();

    eframe::run_native("Popups", options, Box::new(|_| Ok(Box::<MyApp>::default())))
}

#[derive(Default)]
struct MyApp {
    checkbox: bool,
    number: u8,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.label("PopupCloseBehavior::CloseOnClickAway popup");
            let response = ui.button("Open");
            let popup_id = Id::new("popup_id");

            if response.clicked() {
                ui.memory_mut(|mem| mem.toggle_popup(popup_id));
            }

            popup_below_widget(
                ui,
                popup_id,
                &response,
                PopupCloseBehavior::CloseOnClickOutside,
                |ui| {
                    ui.set_min_width(300.0);
                    ui.label("This popup will be open even if you click the checkbox");
                    ui.checkbox(&mut self.checkbox, "Checkbox");
                },
            );

            ui.label("PopupCloseBehavior::CloseOnClick popup");
            ComboBox::from_label("ComboBox")
                .selected_text(format!("{}", self.number))
                .show_ui(ui, |ui| {
                    for num in 0..10 {
                        ui.selectable_value(&mut self.number, num, format!("{num}"));
                    }
                });
        });
    }
}
