#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::{self, text_edit::TextType};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| Ok(Box::<MyApp>::default())),
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(&format!(
                "I am {}. I am {} years old. My favorite letter is {}.",
                self.name, self.age, self.favorite_letter
            ));
            ui.label(&format!(
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
                ui.text_edit_singleline(&mut self.age)
                    .labelled_by(name_label.id);
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
    type Err = std::convert::Infallible;

    fn read_from_string(_previous: &Self, modified: &str) -> Option<Result<Self, Self::Err>> {
        Some(Ok(NoCaps(modified.to_lowercase())))
    }

    fn string_representation(&self) -> String {
        self.0.clone()
    }

    fn is_mutable() -> bool {
        true
    }
}
