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
//! ## Simplified usage
//! If your app is only for native, and you don't need advanced features like state persistence,
//! then you can use the simpler function [`run_simple_native`].
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
pub use wasm_bindgen;

#[cfg(target_arch = "wasm32")]
pub use web_sys;

#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(target_arch = "wasm32")]
pub use web::start_web;

// ----------------------------------------------------------------------------
// When compiling natively

#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
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
/// fn main() -> eframe::Result<()> {
///     let native_options = eframe::NativeOptions::default();
///     eframe::run_native("MyApp", native_options, Box::new(|cc| Box::new(MyEguiApp::new(cc))))
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
///
/// # Errors
/// This function can fail if we fail to set up a graphics context.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
#[allow(clippy::needless_pass_by_value)]
pub fn run_native(
    app_name: &str,
    native_options: NativeOptions,
    app_creator: AppCreator,
) -> Result<()> {
    let renderer = native_options.renderer;

    #[cfg(not(feature = "__screenshot"))]
    assert!(
        std::env::var("EFRAME_SCREENSHOT_TO").is_err(),
        "EFRAME_SCREENSHOT_TO found without compiling with the '__screenshot' feature"
    );

    match renderer {
        #[cfg(feature = "glow")]
        Renderer::Glow => {
            log::debug!("Using the glow renderer");
            native::run::run_glow(app_name, native_options, app_creator)
        }

        #[cfg(feature = "wgpu")]
        Renderer::Wgpu => {
            log::debug!("Using the wgpu renderer");
            native::run::run_wgpu(app_name, native_options, app_creator)
        }
    }
}

// ----------------------------------------------------------------------------

/// The simplest way to get started when writing a native app.
///
/// This does NOT support persistence. For that you need to use [`run_native`].
///
/// # Example
/// ``` no_run
/// fn main() -> eframe::Result<()> {
///     // Our application state:
///     let mut name = "Arthur".to_owned();
///     let mut age = 42;
///
///     let options = eframe::NativeOptions::default();
///     eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
///         egui::CentralPanel::default().show(ctx, |ui| {
///             ui.heading("My egui Application");
///             ui.horizontal(|ui| {
///                 let name_label = ui.label("Your name: ");
///                 ui.text_edit_singleline(&mut name)
///                     .labelled_by(name_label.id);
///             });
///             ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
///             if ui.button("Click each year").clicked() {
///                 age += 1;
///             }
///             ui.label(format!("Hello '{}', age {}", name, age));
///         });
///     })
/// }
/// ```
///
/// # Errors
/// This function can fail if we fail to set up a graphics context.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
pub fn run_simple_native(
    app_name: &str,
    native_options: NativeOptions,
    update_fun: impl FnMut(&egui::Context, &mut Frame) + 'static,
) -> Result<()> {
    struct SimpleApp<U> {
        update_fun: U,
    }
    impl<U: FnMut(&egui::Context, &mut Frame)> App for SimpleApp<U> {
        fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
            (self.update_fun)(ctx, frame);
        }
    }

    run_native(
        app_name,
        native_options,
        Box::new(|_cc| Box::new(SimpleApp { update_fun })),
    )
}

// ----------------------------------------------------------------------------

/// The different problems that can occur when trying to run `eframe`.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[cfg(not(target_arch = "wasm32"))]
    #[error("winit error: {0}")]
    Winit(#[from] winit::error::OsError),

    #[cfg(all(feature = "glow", not(target_arch = "wasm32")))]
    #[error("glutin error: {0}")]
    Glutin(#[from] glutin::error::Error),

    #[cfg(all(feature = "glow", not(target_arch = "wasm32")))]
    #[error("Found no glutin configs matching the template: {0:?}. error: {1:?}")]
    NoGlutinConfigs(glutin::config::ConfigTemplate, Box<dyn std::error::Error>),

    #[cfg(feature = "wgpu")]
    #[error("WGPU error: {0}")]
    Wgpu(#[from] egui_wgpu::WgpuError),
}

pub type Result<T> = std::result::Result<T, Error>;

// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
mod profiling_scopes {
    /// Profiling macro for feature "puffin"
    macro_rules! profile_function {
    ($($arg: tt)*) => {
        #[cfg(feature = "puffin")]
        puffin::profile_function!($($arg)*);
    };
}
    pub(crate) use profile_function;

    /// Profiling macro for feature "puffin"
    macro_rules! profile_scope {
    ($($arg: tt)*) => {
        #[cfg(feature = "puffin")]
        puffin::profile_scope!($($arg)*);
    };
}
    pub(crate) use profile_scope;
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
pub(crate) use profiling_scopes::*;
