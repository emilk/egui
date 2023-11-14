#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Confirm exit",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

#[derive(Default)]
struct MyApp {
    show_child_viewport: bool,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello from the root viewport");

            ui.checkbox(&mut self.show_child_viewport, "Show secondary viewport");
        });

        let mut viewport = egui::ViewportBuilder::CHILD;
        viewport.with_title("Secondary Viewport");

        if self.show_child_viewport {
            ctx.show_viewport(
                egui::ViewportId::from_hash_of("secondary_viewport"),
                viewport,
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label("Hello from secondary viewport");
                    });
                },
            );
        }
    }
}
