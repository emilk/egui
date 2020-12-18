#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all)]

fn main() {
    // Persist app state to file:
    let storage = egui_glium::storage::FileStorage::from_path(".egui_demo_glium.json");

    // Alternative: store nowhere
    // let storage = egui::app::DummyStorage::default();

    let app: egui::DemoApp = egui::app::get_value(&storage, egui::app::APP_KEY).unwrap_or_default();
    egui_glium::run(Box::new(storage), Box::new(app));
}
