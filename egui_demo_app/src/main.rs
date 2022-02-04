#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// Forbid warnings in release builds:
#![cfg_attr(not(debug_assertions), deny(warnings))]
#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]

// When compiling natively:
fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let app = egui_demo_lib::WrapApp::default();
    let options = eframe::NativeOptions {
        // Let's show off that we support transparent windows
        transparent: true,
        drag_and_drop_support: true,
        ..Default::default()
    };
    eframe::run_native(Box::new(app), options);
}
