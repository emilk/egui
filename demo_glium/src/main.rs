#![deny(warnings)]
#![warn(clippy::all)]

use egui_glium::storage::FileStorage;

fn main() {
    let title = "Egui glium demo";
    let storage = FileStorage::from_path(".egui_demo_glium.json".into());
    let app: egui::DemoApp = egui::app::get_value(&storage, egui::app::APP_KEY).unwrap_or_default();
    egui_glium::run(title, storage, app);
}
