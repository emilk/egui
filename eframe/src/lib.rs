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
//! use eframe::{epi, egui};
//!
//! #[derive(Default)]
//! struct MyEguiApp {}
//!
//! impl epi::App for MyEguiApp {
//!    fn name(&self) -> &str {
//!        "My egui App"
//!    }
//!
//!    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
//!        egui::CentralPanel::default().show(ctx, |ui| {
//!            ui.heading("Hello World!");
//!        });
//!    }
//!}
//!
//! fn main() {
//!     let app = MyEguiApp::default();
//!     let native_options = eframe::NativeOptions::default();
//!     eframe::run_native(Box::new(app), native_options);
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
//!     let app = MyEguiApp::default();
//!     eframe::start_web(canvas_id, Box::new(app))
//! }
//! ```

// Forbid warnings in release builds:
#![cfg_attr(not(debug_assertions), deny(warnings))]
#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    missing_docs,
    rust_2018_idioms,
    rustdoc::missing_crate_level_docs
)]
#![allow(clippy::needless_doctest_main)]

pub use {egui, egui::emath, egui::epaint, epi};

#[cfg(not(target_arch = "wasm32"))]
pub use epi::NativeOptions;

// ----------------------------------------------------------------------------
// When compiling for web

#[cfg(target_arch = "wasm32")]
pub use egui_web::wasm_bindgen;

/// Install event listeners to register different input events
/// and start running the given app.
///
/// For performance reasons (on some browsers) the egui canvas does not, by default,
/// fill the whole width of the browser.
/// This can be changed by overriding [`epi::Frame::max_size_points`].
///
/// ### Usage, native:
/// ``` no_run
/// fn main() {
///     let app = MyEguiApp::default();
///     let native_options = eframe::NativeOptions::default();
///     eframe::run_native(Box::new(app), native_options);
/// }
/// ```
///
/// ### Web
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

/// Call from `fn main` like this:
/// ``` no_run
/// use eframe::{epi, egui};
///
/// #[derive(Default)]
/// struct MyEguiApp {}
///
/// impl epi::App for MyEguiApp {
///    fn name(&self) -> &str {
///        "My egui App"
///    }
///
///    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
///        egui::CentralPanel::default().show(ctx, |ui| {
///            ui.heading("Hello World!");
///        });
///    }
///}
///
/// fn main() {
///     let app = MyEguiApp::default();
///     let native_options = eframe::NativeOptions::default();
///     eframe::run_native(Box::new(app), native_options);
/// }
/// ```
#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "egui_glium")]
pub fn run_native(app: Box<dyn epi::App>, native_options: epi::NativeOptions) -> ! {
    egui_glium::run(app, &native_options)
}

/// Call from `fn main` like this:
/// ``` no_run
/// use eframe::{epi, egui};
///
/// #[derive(Default)]
/// struct MyEguiApp {}
///
/// impl epi::App for MyEguiApp {
///    fn name(&self) -> &str {
///        "My egui App"
///    }
///
///    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
///        egui::CentralPanel::default().show(ctx, |ui| {
///            ui.heading("Hello World!");
///        });
///    }
///}
///
/// fn main() {
///     let app = MyEguiApp::default();
///     let native_options = eframe::NativeOptions::default();
///     eframe::run_native(Box::new(app), native_options);
/// }
/// ```
#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(feature = "egui_glium"))] // make sure we still compile with `--all-features`
#[cfg(feature = "egui_glow")]
pub fn run_native(app: Box<dyn epi::App>, native_options: epi::NativeOptions) -> ! {
    egui_glow::run(app, &native_options)
}

// disabled since we want to be able to compile with `--all-features`
// #[cfg(all(feature = "egui_glium", feature = "egui_glow"))]
// compile_error!("Enable either egui_glium or egui_glow, not both");

#[cfg(not(any(feature = "egui_glium", feature = "egui_glow")))]
compile_error!("Enable either egui_glium or egui_glow");
