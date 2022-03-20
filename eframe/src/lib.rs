//! eframe - the egui framework crate
//!
//! If you are planning to write an app for web or native,
//! and are happy with just using egui for all visuals,
//! Then `eframe` is for you!
//!
//! To get started, look at <https://github.com/emilk/eframe_template>.
//!
//! You can also take a look at [the `eframe` examples folder](https://github.com/emilk/egui/tree/master/eframe/examples).
//!
//! You write your application code for [`epi`] (implementing [`epi::App`]) and then
//! call from [`crate::run_native`] your `main.rs`, and/or call `eframe::start_web` from your `lib.rs`.
//!
//! `eframe` is implemented using [`egui_web`](https://github.com/emilk/egui/tree/master/egui_web) for web and
//! [`egui_glium`](https://github.com/emilk/egui/tree/master/egui_glium) or [`egui_glow`](https://github.com/emilk/egui/tree/master/egui_glow) for native.
//!
//! ## Usage, native:
//! ``` no_run
//! use eframe::egui;
//!
//! fn main() {
//!     let native_options = eframe::NativeOptions::default();
//!     eframe::run_native("My egui App", native_options, Box::new(|cc| Box::new(MyEguiApp::new(cc))));
//! }
//!
//! #[derive(Default)]
//! struct MyEguiApp {}
//!
//! impl MyEguiApp {
//!     fn new(cc: &eframe::CreationContext<'_>) -> Self {
//!         // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
//!         // Restore app state using cc.storage (requires the "persistence" feature).
//!         // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
//!         // for e.g. egui::PaintCallback.
//!         Self::default()
//!     }
//! }
//!
//! impl eframe::App for MyEguiApp {
//!    fn update(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
//!        egui::CentralPanel::default().show(ctx, |ui| {
//!            ui.heading("Hello World!");
//!        });
//!    }
//! }
//! ```
//!
//! ## Usage, web:
//! ``` no_run
//! #[cfg(target_arch = "wasm32")]
//! use wasm_bindgen::prelude::*;
//!
//! /// Call this once from the HTML.
//! #[cfg(target_arch = "wasm32")]
//! #[wasm_bindgen]
//! pub fn start(canvas_id: &str) -> Result<(), eframe::wasm_bindgen::JsValue> {
//!     eframe::start_web(canvas_id, Box::new(|cc| Box::new(MyApp::new(cc))))
//! }
//! ```

#![allow(clippy::needless_doctest_main)]

// Re-export all useful libraries:
pub use {egui, egui::emath, egui::epaint, epi};

// Re-export everything in `epi` so `eframe` users don't have to care about what `epi` is:
pub use epi::*;

// ----------------------------------------------------------------------------
// When compiling for web

#[cfg(target_arch = "wasm32")]
pub use egui_web::wasm_bindgen;

/// Install event listeners to register different input events
/// and start running the given app.
///
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
///     eframe::start_web(canvas_id, Box::new(|cc| Box::new(MyEguiApp::new(cc))))
/// }
/// ```
#[cfg(target_arch = "wasm32")]
pub fn start_web(canvas_id: &str, app_creator: AppCreator) -> Result<(), wasm_bindgen::JsValue> {
    egui_web::start(canvas_id, app_creator)?;
    Ok(())
}

// ----------------------------------------------------------------------------
// When compiling natively

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub use native::run_native;
