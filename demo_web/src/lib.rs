#![deny(warnings)]
#![warn(clippy::all)]

use wasm_bindgen::prelude::*;

/// This is the entry-point for all the web-assembly.
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
    let backend = egui_web::WebBackend::new(canvas_id)?;
    let app = Box::new(egui::DemoApp::default());
    let runner = egui_web::AppRunner::new(backend, app)?;
    egui_web::start(runner)?;
    Ok(())
}
