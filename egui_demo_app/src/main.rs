#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![deny(broken_intra_doc_links)]
#![deny(invalid_codeblock_attributes)]
#![deny(private_intra_doc_links)]
#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]

// When compiling natively:
fn main() {
    let app = egui_demo_lib::WrapApp::default();
    eframe::run_native(Box::new(app));
}
