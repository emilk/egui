#![forbid(unsafe_code)]
#![deny(warnings)]
#![warn(clippy::all)]

fn main() {
    let title = "Egui glium demo";

    // Persist app state to file:
    let storage = egui_glium::storage::FileStorage::from_path(".egui_demo_glium.json");

    // Alternative: store nowhere
    // let storage = egui::app::DummyStorage::default();

    let app: egui::DemoApp = egui::app::get_value(&storage, egui::app::APP_KEY).unwrap_or_default();        
    egui_glium::run(title, true, Box::new(storage), Box::new(app));
}
