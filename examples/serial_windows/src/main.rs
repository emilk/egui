#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        run_and_return: true,
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    log::info!("Starting first window…");
    eframe::run_native(
        "First Window",
        options.clone(),
        Box::new(|_cc| Ok(Box::new(MyApp { has_next: true }))),
    )?;

    std::thread::sleep(std::time::Duration::from_secs(2));

    log::info!("Starting second window…");
    eframe::run_native(
        "Second Window",
        options.clone(),
        Box::new(|_cc| Ok(Box::new(MyApp { has_next: true }))),
    )?;

    std::thread::sleep(std::time::Duration::from_secs(2));

    log::info!("Starting third window…");
    eframe::run_native(
        "Third Window",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp { has_next: false }))),
    )
}

struct MyApp {
    pub(crate) has_next: bool,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let label_text = if self.has_next {
                "When this window is closed the next will be opened after a short delay"
            } else {
                "This is the last window. Program will end when closed"
            };
            ui.label(label_text);

            if ui.button("Close").clicked() {
                log::info!("Pressed Close button");
                ui.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}
