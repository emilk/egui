//! eframe - the egui framework crate
//!
//! If you are planning to write an app for web or native,
//! and are happy with just using egui for all visuals,
//! Then `eframe` is for you!
//!
//! To get started, look at <https://github.com/emilk/egui_template>.
//!
//! You write your application code for [`epi`] (implementing [`epi::App`]) and then
//! call from [`crate::run_native`] your `main.rs`, and/or call `eframe::start_web` from your `lib.rs`.
//!
//! `eframe` is implemented using [`egui_web`](https://docs.rs/egui_web) and [`egui_glium`](https://docs.rs/egui_glium).

#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::missing_crate_level_docs,
    rustdoc::private_intra_doc_links
)]
#![forbid(unsafe_code)]
#![warn(clippy::all, missing_docs, rust_2018_idioms)]

pub use {egui, epi};

#[cfg(not(target_arch = "wasm32"))]
pub use epi::NativeOptions;

// ----------------------------------------------------------------------------
// When compiling for web

#[cfg(target_arch = "wasm32")]
pub use egui_web::wasm_bindgen;

/// Install event listeners to register different input events
/// and start running the given app.
///
/// Usage:
/// ``` no_run
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

/// Call from `fn main` like this: `eframe::run_native(Box::new(MyEguiApp::default()))`
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native(app: Box<dyn epi::App>, native_options: epi::NativeOptions) -> ! {
    egui_glium::run(app, native_options)
}
