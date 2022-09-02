#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() {
    if cfg!(target_os = "macos") {
        eprintln!("WARNING: this example does not work on Mac! See https://github.com/emilk/egui/issues/1918");
    }

    let options = eframe::NativeOptions {
        run_and_return: true,
        ..Default::default()
    };

    eprintln!("Starting first window…");
    eframe::run_native(
        "First Window",
        options.clone(),
        Box::new(|_cc| Box::new(MyApp::default())),
    );

    std::thread::sleep(std::time::Duration::from_secs(2));

    eprintln!("Starting second window…");
    eframe::run_native(
        "Second Window",
        options.clone(),
        Box::new(|_cc| Box::new(MyApp::default())),
    );

    std::thread::sleep(std::time::Duration::from_secs(2));

    eprintln!("Starting third window…");
    eframe::run_native(
        "Third Window",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

#[derive(Default)]
struct MyApp {}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Close").clicked() {
                eprintln!("Pressed Close button");
                frame.close();
            }
        });
    }
}
