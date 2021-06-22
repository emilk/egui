// Forbid warnings in release builds:
#![cfg_attr(not(debug_assertions), deny(warnings))]
// Disabled so we can support rust 1.51:
// #![deny(
//     rustdoc::broken_intra_doc_links,
//     rustdoc::invalid_codeblock_attributes,
//     rustdoc::missing_crate_level_docs,
//     rustdoc::private_intra_doc_links
// )]
#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]

// When compiling natively:
fn main() {
    let app = egui_demo_lib::WrapApp::default();
    let options = eframe::NativeOptions {
        // Let's show off that we support transparent windows
        transparent: true,
        ..Default::default()
    };
    eframe::run_native(Box::new(app), options);
}
