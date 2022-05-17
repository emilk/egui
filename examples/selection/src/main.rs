#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use std::fmt;

const STRING_OPTIONS: [&str; 8] = [
    "Option 1", "Option 2", "Option 3", "Option 4", "Option 5", "Option 6", "Option 7", "Option 8",
];

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui Selection example",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

// Derive debut to conver to string and PartialEq to be comparable and allow the combobox to distinguish the selected option
#[derive(Debug, PartialEq, Clone, Copy)]
enum MyOption {
    First,
    Second,
    Third,
}

impl fmt::Display for MyOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

struct MyApp {
    option: MyOption,
    string_option: String,
    int_option: usize,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            option: MyOption::First,
            string_option: String::from(""),
            int_option: 0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Selection example (ComboBox/SelectableLabel)");

            ui.add_space(16.);

            egui::ComboBox::from_label(format!("Currently selected enum: {}", self.option)) // When created from a label the text will b shown on the side of the combobox
                .selected_text(self.option.to_string()) // This is the currently selected option (in text form)
                .show_ui(ui, |ui| { // In this closure the various options can be added
                    for option in [MyOption::First, MyOption::Second, MyOption::Third] {
                        // The first parameter is a mutable reference to allow the choice to be modified when the user selects
                        // something else. The second parameter is the actual value of the option (to be compared with the currently)
                        // selected one to allow egui to highlight the correct label. The third parameter is the string to show.
                        ui.selectable_value(&mut self.option, option, option.to_string());
                    }
                });

            ui.label("These options are selectable just like the combobox before them");
            for option in [MyOption::First, MyOption::Second, MyOption::Third] {
                // SelectableLabel is a similar widget; it works like a button that can be checked. Here it serves the 
                // purpose of a radio button, with a single option being selected at any time
                if ui
                    .add(egui::SelectableLabel::new(
                        self.option == option,
                        option.to_string(),
                    ))
                    .clicked()
                {
                    self.option = option;
                }
            }

            ui.add_space(16.);

            egui::ComboBox::from_label(format!(
                "Currently selected string: {}",
                self.string_option
            ))
            .selected_text(self.string_option.clone())
            .show_ui(ui, |ui| {
                for option in STRING_OPTIONS {
                    // Selectable values can be anything: enums, strings or integers - as long as they can be compared and have a text repersentation
                    ui.selectable_value(&mut self.string_option, option.into(), option.clone());
                }
            });

            ui.label("When creating these options the 'clicked' condition is not checked, so they simply display the choice without being interactable");
            egui::ScrollArea::vertical()
                .auto_shrink([true, false])
                .max_height(64.)
                .show(ui, |ui| {
                    for option in STRING_OPTIONS {
                        ui.add(egui::SelectableLabel::new(
                            self.string_option == option,
                            option.clone(),
                        ));
                    }
                });

            ui.add_space(16.);

            ui.label(format!("Currently selected number: {}", self.int_option));
            egui::ComboBox::from_id_source(0)
                .selected_text(self.int_option.to_string())
                .show_ui(ui, |ui| {
                    for option in 0..3 {
                        ui.selectable_value(
                            &mut self.int_option,
                            option,
                            (option as u32).to_string(),
                        );
                    }
                });
        });
    }
}
