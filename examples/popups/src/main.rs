#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::{CentralPanel, ComboBox, Popup, PopupCloseBehavior};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions::default();

    eframe::run_native("Popups", options, Box::new(|_| Ok(Box::<MyApp>::default())))
}

#[derive(Default)]
struct MyApp {
    checkbox: bool,
    number: u8,
    numbers: [bool; 10],
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        CentralPanel::default().show_inside(ui, |ui| {
            ui.label("PopupCloseBehavior::CloseOnClick popup");
            ComboBox::from_label("ComboBox")
                .selected_text(format!("{}", self.number))
                .show_ui(ui, |ui| {
                    for num in 0..10 {
                        ui.selectable_value(&mut self.number, num, format!("{num}"));
                    }
                });

            ui.label("PopupCloseBehavior::CloseOnClickOutside popup");
            ComboBox::from_label("Ignore Clicks")
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .selected_text("Select Numbers")
                .show_ui(ui, |ui| {
                    ui.label("This popup will be open even if you click the checkboxes");
                    for (i, num) in self.numbers.iter_mut().enumerate() {
                        ui.checkbox(num, format!("Checkbox {}", i + 1));
                    }
                });

            ui.label("PopupCloseBehavior::IgnoreClicks popup");
            let response = ui.button("Open");

            Popup::menu(&response)
                .close_behavior(PopupCloseBehavior::IgnoreClicks)
                .show(|ui| {
                    ui.set_min_width(310.0);
                    ui.label("This popup will be open until you press the button again");
                    ui.checkbox(&mut self.checkbox, "Checkbox");
                });
        });
    }
}
