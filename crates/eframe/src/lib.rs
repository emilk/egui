//! eframe - the [`egui`] framework crate
//!
//! If you are planning to write an app for web or native,
//! and want to use [`egui`] for everything, then `eframe` is for you!
//!
//! To get started, see the [examples](https://github.com/emilk/egui/tree/master/examples).
//! To learn how to set up `eframe` for web and native, go to <https://github.com/emilk/eframe_template/> and follow the instructions there!
//!
//! In short, you implement [`App`] (especially [`App::update`]) and then
//! call [`crate::run_native`] from your `main.rs`, and/or call `eframe::start_web` from your `lib.rs`.
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
//!    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
//! pub async fn start(canvas_id: &str) -> Result<AppRunnerRef, eframe::wasm_bindgen::JsValue> {
//!     let web_options = eframe::WebOptions::default();
//!     eframe::start_web(canvas_id, web_options, Box::new(|cc| Box::new(MyEguiApp::new(cc)))).await
//! }
//! ```
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![allow(clippy::needless_doctest_main)]

// Re-export all useful libraries:
pub use {egui, egui::emath, egui::epaint};

#[cfg(feature = "glow")]
pub use {egui_glow, glow};

#[cfg(feature = "wgpu")]
pub use {egui_wgpu, wgpu};

mod epi;

// Re-export everything in `epi` so `eframe` users don't have to care about what `epi` is:
pub use epi::*;

// ----------------------------------------------------------------------------
// When compiling for web

#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen;

#[cfg(target_arch = "wasm32")]
use web::AppRunnerRef;

#[cfg(target_arch = "wasm32")]
pub use web_sys;

/// Install event listeners to register different input events
/// and start running the given app.
///
/// ``` no_run
/// #[cfg(target_arch = "wasm32")]
/// use wasm_bindgen::prelude::*;
///
/// /// This is the entry-point for all the web-assembly.
/// /// This is called from the HTML.
/// /// It loads the app, installs some callbacks, then returns.
/// /// It returns a handle to the running app that can be stopped calling `AppRunner::stop_web`.
/// /// You can add more callbacks like this if you want to call in to your code.
/// #[cfg(target_arch = "wasm32")]
/// #[wasm_bindgen]
/// pub struct WebHandle {
///     handle: AppRunnerRef,
/// }
/// #[cfg(target_arch = "wasm32")]
/// #[wasm_bindgen]
/// pub async fn start(canvas_id: &str) -> Result<WebHandle, eframe::wasm_bindgen::JsValue> {
///     let web_options = eframe::WebOptions::default();
///     eframe::start_web(
///         canvas_id,
///         web_options,
///         Box::new(|cc| Box::new(MyEguiApp::new(cc))),
///     )
///     .await
///     .map(|handle| WebHandle { handle })
/// }
/// ```
///
/// # Errors
/// Failing to initialize WebGL graphics.
#[cfg(target_arch = "wasm32")]
pub async fn start_web(
    canvas_id: &str,
    web_options: WebOptions,
    app_creator: AppCreator,
) -> Result<AppRunnerRef, wasm_bindgen::JsValue> {
    let handle = web::start(canvas_id, web_options, app_creator).await?;

    Ok(handle)
}

// ----------------------------------------------------------------------------
// When compiling natively

#[cfg(not(target_arch = "wasm32"))]
mod native;

/// This is how you start a native (desktop) app.
///
/// The first argument is name of your app, used for the title bar of the native window
/// and the save location of persistence (see [`App::save`]).
///
/// Call from `fn main` like this:
/// ``` no_run
/// use eframe::egui;
///
/// fn main() {
///     let native_options = eframe::NativeOptions::default();
///     eframe::run_native("MyApp", native_options, Box::new(|cc| Box::new(MyEguiApp::new(cc))));
/// }
///
/// #[derive(Default)]
/// struct MyEguiApp {}
///
/// impl MyEguiApp {
///     fn new(cc: &eframe::CreationContext<'_>) -> Self {
///         // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
///         // Restore app state using cc.storage (requires the "persistence" feature).
///         // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
///         // for e.g. egui::PaintCallback.
///         Self::default()
///     }
/// }
///
/// impl eframe::App for MyEguiApp {
///    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
///        egui::CentralPanel::default().show(ctx, |ui| {
///            ui.heading("Hello World!");
///        });
///    }
/// }
/// ```
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::needless_pass_by_value)]
pub fn run_native(app_name: &str, native_options: NativeOptions, app_creator: AppCreator) {
    let renderer = native_options.renderer;

    #[cfg(not(feature = "__screenshot"))]
    assert!(
        std::env::var("EFRAME_SCREENSHOT_TO").is_err(),
        "EFRAME_SCREENSHOT_TO found without compiling with the '__screenshot' feature"
    );

    match renderer {
        #[cfg(feature = "glow")]
        Renderer::Glow => {
            tracing::debug!("Using the glow renderer");
            native::run::run_glow(app_name, native_options, app_creator);
        }

        #[cfg(feature = "wgpu")]
        Renderer::Wgpu => {
            tracing::debug!("Using the wgpu renderer");
            native::run::run_wgpu(app_name, native_options, app_creator);
        }
    }
}

// ---------------------------------------------------------------------------

/// Profiling macro for feature "puffin"
#[cfg(not(target_arch = "wasm32"))]
macro_rules! profile_function {
    ($($arg: tt)*) => {
        #[cfg(feature = "puffin")]
        puffin::profile_function!($($arg)*);
    };
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use profile_function;

/// Profiling macro for feature "puffin"
#[cfg(not(target_arch = "wasm32"))]
macro_rules! profile_scope {
    ($($arg: tt)*) => {
        #[cfg(feature = "puffin")]
        puffin::profile_scope!($($arg)*);
    };
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use profile_scope;
