//! eframe - the [`egui`] framework crate
//!
//! If you are planning to write an app for web or native,
//! and want to use [`egui`] for everything, then `eframe` is for you!
//!
//! To get started, see the [examples](https://github.com/emilk/egui/tree/master/examples).
//! To learn how to set up `eframe` for web and native, go to <https://github.com/emilk/eframe_template/> and follow the instructions there!
//!
//! In short, you implement [`App`] (especially [`App::update`]) and then
//! call [`crate::run_native`] from your `main.rs`, and/or use `eframe::WebRunner` from your `lib.rs`.
//!
//! ## Compiling for web
//! To get copy-paste working on web, you need to compile with
//! `export RUSTFLAGS=--cfg=web_sys_unstable_apis`.
//!
//! You need to install the `wasm32` target with `rustup target add wasm32-unknown-unknown`.
//!
//! Build the `.wasm` using `cargo build --target wasm32-unknown-unknown`
//! and then use [`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen) to generate the JavaScript glue code.
//!
//! See the [`eframe_template` repository](https://github.com/emilk/eframe_template/) for more.
//!
//! ## Simplified usage
//! If your app is only for native, and you don't need advanced features like state persistence,
//! then you can use the simpler function [`run_simple_native`].
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
//! # #[cfg(target_arch = "wasm32")]
//! use wasm_bindgen::prelude::*;
//!
//! /// Your handle to the web app from JavaScript.
//! # #[cfg(target_arch = "wasm32")]
//! #[derive(Clone)]
//! #[wasm_bindgen]
//! pub struct WebHandle {
//!     runner: eframe::WebRunner,
//! }
//!
//! # #[cfg(target_arch = "wasm32")]
//! #[wasm_bindgen]
//! impl WebHandle {
//!     /// Installs a panic hook, then returns.
//!     #[allow(clippy::new_without_default)]
//!     #[wasm_bindgen(constructor)]
//!     pub fn new() -> Self {
//!         // Redirect [`log`] message to `console.log` and friends:
//!         eframe::WebLogger::init(log::LevelFilter::Debug).ok();
//!
//!         Self {
//!             runner: eframe::WebRunner::new(),
//!         }
//!     }
//!
//!     /// Call this once from JavaScript to start your app.
//!     #[wasm_bindgen]
//!     pub async fn start(&self, canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
//!         self.runner
//!             .start(
//!                 canvas_id,
//!                 eframe::WebOptions::default(),
//!                 Box::new(|cc| Box::new(MyEguiApp::new(cc))),
//!             )
//!             .await
//!     }
//!
//!     // The following are optional:
//!
//!     /// Shut down eframe and clean up resources.
//!     #[wasm_bindgen]
//!     pub fn destroy(&self) {
//!         self.runner.destroy();
//!     }
//!
//!     /// Example on how to call into your app from JavaScript.
//!     #[wasm_bindgen]
//!     pub fn example(&self) {
//!         if let Some(app) = self.runner.app_mut::<MyEguiApp>() {
//!             app.example();
//!         }
//!     }
//!
//!     /// The JavaScript can check whether or not your app has crashed:
//!     #[wasm_bindgen]
//!     pub fn has_panicked(&self) -> bool {
//!         self.runner.has_panicked()
//!     }
//!
//!     #[wasm_bindgen]
//!     pub fn panic_message(&self) -> Option<String> {
//!         self.runner.panic_summary().map(|s| s.message())
//!     }
//!
//!     #[wasm_bindgen]
//!     pub fn panic_callstack(&self) -> Option<String> {
//!         self.runner.panic_summary().map(|s| s.callstack())
//!     }
//! }
//! ```
//!
//! ## Feature flags
#![doc = document_features::document_features!()]
//!

#![warn(missing_docs)] // let's keep eframe well-documented
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

pub(crate) mod stopwatch;

// ----------------------------------------------------------------------------
// When compiling for web

#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen;

#[cfg(target_arch = "wasm32")]
pub use web_sys;

#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(target_arch = "wasm32")]
pub use web::{WebLogger, WebRunner};

// ----------------------------------------------------------------------------
// When compiling natively

#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
#[cfg(feature = "persistence")]
pub use native::file_storage::storage_dir;

#[cfg(not(target_arch = "wasm32"))]
pub mod icon_data;

/// This is how you start a native (desktop) app.
///
/// The first argument is name of your app, which is a an identifier
/// used for the save location of persistence (see [`App::save`]).
/// It is also used as the application id on wayland.
/// If you set no title on the viewport, the app id will be used
/// as the title.
///
/// For details about application ID conventions, see the
/// [Desktop Entry Spec](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#desktop-file-id)
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
    mut native_options: NativeOptions,
    app_creator: AppCreator,
) -> Result<()> {
    #[cfg(not(feature = "__screenshot"))]
    assert!(
        std::env::var("EFRAME_SCREENSHOT_TO").is_err(),
        "EFRAME_SCREENSHOT_TO found without compiling with the '__screenshot' feature"
    );

    if native_options.viewport.title.is_none() {
        native_options.viewport.title = Some(app_name.to_owned());
    }

    let renderer = native_options.renderer;

    #[cfg(all(feature = "glow", feature = "wgpu"))]
    {
        match renderer {
            Renderer::Glow => "glow",
            Renderer::Wgpu => "wgpu",
        };
        log::info!("Both the glow and wgpu renderers are available. Using {renderer}.");
    }

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
///             if ui.button("Increment").clicked() {
///                 age += 1;
///             }
///             ui.label(format!("Hello '{name}', age {age}"));
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

    impl<U: FnMut(&egui::Context, &mut Frame) + 'static> App for SimpleApp<U> {
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
    /// An error from [`winit`].
    #[cfg(not(target_arch = "wasm32"))]
    #[error("winit error: {0}")]
    Winit(#[from] winit::error::OsError),

    /// An error from [`winit::event_loop::EventLoop`].
    #[cfg(not(target_arch = "wasm32"))]
    #[error("winit EventLoopError: {0}")]
    WinitEventLoop(#[from] winit::error::EventLoopError),

    /// An error from [`glutin`] when using [`glow`].
    #[cfg(all(feature = "glow", not(target_arch = "wasm32")))]
    #[error("glutin error: {0}")]
    Glutin(#[from] glutin::error::Error),

    /// An error from [`glutin`] when using [`glow`].
    #[cfg(all(feature = "glow", not(target_arch = "wasm32")))]
    #[error("Found no glutin configs matching the template: {0:?}. Error: {1:?}")]
    NoGlutinConfigs(glutin::config::ConfigTemplate, Box<dyn std::error::Error>),

    /// An error from [`glutin`] when using [`glow`].
    #[cfg(feature = "glow")]
    #[error("egui_glow: {0}")]
    OpenGL(#[from] egui_glow::PainterError),

    /// An error from [`wgpu`].
    #[cfg(feature = "wgpu")]
    #[error("WGPU error: {0}")]
    Wgpu(#[from] egui_wgpu::WgpuError),
}

/// Short for `Result<T, eframe::Error>`.
pub type Result<T, E = Error> = std::result::Result<T, E>;

// ---------------------------------------------------------------------------

mod profiling_scopes {
    #![allow(unused_macros)]
    #![allow(unused_imports)]

    /// Profiling macro for feature "puffin"
    macro_rules! profile_function {
        ($($arg: tt)*) => {
            #[cfg(feature = "puffin")]
            #[cfg(not(target_arch = "wasm32"))] // Disabled on web because of the coarse 1ms clock resolution there.
            puffin::profile_function!($($arg)*);
        };
    }
    pub(crate) use profile_function;

    /// Profiling macro for feature "puffin"
    macro_rules! profile_scope {
        ($($arg: tt)*) => {
            #[cfg(feature = "puffin")]
            #[cfg(not(target_arch = "wasm32"))] // Disabled on web because of the coarse 1ms clock resolution there.
            puffin::profile_scope!($($arg)*);
        };
    }
    pub(crate) use profile_scope;
}

#[allow(unused_imports)]
pub(crate) use profiling_scopes::*;
