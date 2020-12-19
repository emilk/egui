#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all)]

// When compiling natively:
fn main() {
    let app = egui::DemoApp::default();
    egui_glium::run(Box::new(app));
}
