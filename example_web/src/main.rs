#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all)]

// When compiling natively:
fn main() {
    let app = example_web::ExampleApp::default();
    eframe::run_native(Box::new(app));
}
