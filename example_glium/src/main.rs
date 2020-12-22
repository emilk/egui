//! Example of how to use Egui
#![forbid(unsafe_code)]
#![deny(warnings)]
#![warn(clippy::all)]

mod example_app;
use example_app::ExampleApp;

fn main() {
    let title = "My Egui Window";

    // Persist app state to file:
    let storage = egui_glium::storage::FileStorage::from_path(".egui_example_glium.json");

    // Alternative: store nowhere
    // let storage = egui::app::DummyStorage::default();

    // Restore `example_app` from file, or create new `ExampleApp`:
    let app: ExampleApp = egui::app::get_value(&storage, egui::app::APP_KEY).unwrap_or_default();

    egui_glium::run(title, Box::new(storage), Box::new(app));
}
