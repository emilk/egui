#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    name: String,
    name2: String,
    age: u32,
    age2: u32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            name2: "Afadsf".to_owned(),
            age: 42,
            age2: 42,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_debug_on_hover(true);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.name);
                    ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age 1"));
                    if ui.button("First button").clicked() {
                        self.age += 1;
                    }
                });
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.name2);
                    ui.add(egui::Slider::new(&mut self.age2, 0..=120).text("age 2"));
                    if ui.button("Second button").clicked() {
                        self.age += 1;
                    }
                })
            })
        });
    }
}
