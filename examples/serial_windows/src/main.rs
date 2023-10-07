#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    if cfg!(target_os = "macos") {
        eprintln!("WARNING: this example does not work on Mac! See https://github.com/emilk/egui/issues/1918");
    }

    let options = eframe::NativeOptions {
        run_and_return: true,
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    eprintln!("Starting first window…");
    eframe::run_native(
        "First Window",
        options.clone(),
        Box::new(|_cc| Box::new(MyApp { has_next: true })),
    )?;

    std::thread::sleep(std::time::Duration::from_secs(2));

    eprintln!("Starting second window…");
    eframe::run_native(
        "Second Window",
        options.clone(),
        Box::new(|_cc| Box::new(MyApp { has_next: true })),
    )?;

    std::thread::sleep(std::time::Duration::from_secs(2));

    eprintln!("Starting third window…");
    eframe::run_native(
        "Third Window",
        options,
        Box::new(|_cc| Box::new(MyApp { has_next: false })),
    )
}

struct MyApp {
    pub(crate) has_next: bool,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let label_text = if self.has_next {
                "When this window is closed the next will be opened after a short delay"
            } else {
                "This is the last window. Program will end when closed"
            };
            ui.label(label_text);
            if ui.button("Close").clicked() {
                eprintln!("Pressed Close button");
                frame.close();
            }
        });
    }
}
