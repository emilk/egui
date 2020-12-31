//! Backend-agnostic interface for writing apps using Egui.
//!
//! Egui is a GUI library, which can be plugged in to e.g. a game engine.
//!
//! This crate provides a common interface for programming an app, using Egui,
//! so you can then easily plug it in to a backend such as `egui_web` or `egui_glium`.
//!
//! This crate is primarily used by the `egui_web` and `egui_glium` crates.

#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all)]

pub use {egui, epi};

// ----------------------------------------------------------------------------
// When compiling for web

#[cfg(target_arch = "wasm32")]
pub use egui_web::wasm_bindgen;

/// Install event listeners to register different input events
/// and start running the given app.
///
/// Usage:
/// ``` ignore
/// #[cfg(target_arch = "wasm32")]
/// use wasm_bindgen::prelude::*;
///
/// /// This is the entry-point for all the web-assembly.
/// /// This is called once from the HTML.
/// /// It loads the app, installs some callbacks, then returns.
/// /// You can add more callbacks like this if you want to call in to your code.
/// #[cfg(target_arch = "wasm32")]
/// #[wasm_bindgen]
/// pub fn start(canvas_id: &str) -> Result<(), eframe::wasm_bindgen::JsValue> {
///     let app = MyEguiApp::default();
///     eframe::start_web(canvas_id, Box::new(app))
/// }
/// ```
#[cfg(target_arch = "wasm32")]
pub fn start_web(canvas_id: &str, app: Box<dyn epi::App>) -> Result<(), wasm_bindgen::JsValue> {
    egui_web::start(canvas_id, app)?;
    Ok(())
}

// ----------------------------------------------------------------------------
// When compiling natively

/// Call from main as `eframe::run_native(Box::new(MyEguiApp::default()))`
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native(app: Box<dyn epi::App>) {
    egui_glium::run(app)
}
