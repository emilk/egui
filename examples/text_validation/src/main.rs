#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::{self, TextEdit, Ui, text_edit::TextType};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    eframe::run_native(
        "My egui App",
        Default::default(),
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

struct MyApp {
    name: String,
    age: u8,
    favorite_letter: char,
    ice_cream: String,
    lowercase: NoCaps,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "James".to_owned(),
            age: 42,
            favorite_letter: 'H',
            ice_cream: "Raspberry".to_owned(),
            lowercase: NoCaps("no caps here!".to_owned()),
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ctx: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ctx, |ui| {
            ui.label(format!(
                "I am {}. I am {} years old. My favorite letter is {}.",
                self.name, self.age, self.favorite_letter
            ));
            ui.label(format!(
                "I know for sure that the best ice cream flaviour is {}!",
                self.ice_cream
            ));

            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });

            ui.horizontal(|ui| {
                let name_label = ui.label("Your Age: ");
                let output = TextEdit::singleline(&mut self.age).show(ui);
                output.response.labelled_by(name_label.id);

                if let Some(valid) = output.text_parsed
                    && !valid
                {
                    ui.label("That can't be my age!");
                }
            });

            ui.horizontal(|ui| {
                let name_label = ui.label("Favorite character: ");
                ui.text_edit_singleline(&mut self.favorite_letter)
                    .labelled_by(name_label.id);
            });

            ui.horizontal(|ui| {
                let name_label = ui.label("Ice cream: ");
                ui.text_edit_singleline(&mut self.ice_cream.as_str())
                    .labelled_by(name_label.id);
            });

            ui.separator();
            ui.heading("welcome to the no caps zone, where only lowercase is allowed.");

            ui.horizontal(|ui| {
                let name_label = ui.label("no caps allowed: ");
                ui.text_edit_singleline(&mut self.lowercase)
                    .labelled_by(name_label.id);
            });
        });
    }
}

struct NoCaps(String);

impl TextType for NoCaps {
    type Err = IncorrectCaseError;

    fn read_from_string(_previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>> {
        if modified.to_lowercase() == modified {
            Some(Ok(Self(modified.to_owned())))
        } else {
            Some(Err(IncorrectCaseError(
                "Contained uppercase letters".to_owned(),
            )))
        }
    }

    fn string_representation(&self) -> String {
        self.0.clone()
    }

    fn is_parsable() -> bool {
        true
    }
}

#[derive(Debug)]
pub struct IncorrectCaseError(String);

impl std::fmt::Display for IncorrectCaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for IncorrectCaseError {}
