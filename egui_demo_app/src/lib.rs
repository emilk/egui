#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![deny(broken_intra_doc_links)]
#![deny(invalid_codeblock_attributes)]
#![deny(private_intra_doc_links)]
#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

/// This is the entry-point for all the web-assembly.
/// This is called once from the HTML.
/// It loads the app, installs some callbacks, then returns.
/// You can add more callbacks like this if you want to call in to your code.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
    let app = egui_demo_lib::WrapApp::default();
    eframe::start_web(canvas_id, Box::new(app))
}
