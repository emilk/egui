//! Backend-agnostic interface for writing apps using [`egui`].
//!
//! `epi` provides interfaces for window management, serialization and http requests.
//! An app written for `epi` can then be plugged into [`eframe`](https://docs.rs/eframe),
//! the egui framework crate.
//!
//! Start by looking at the [`App`] trait, and implement [`App::update`].

#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::dbg_macro,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::filter_map_next,
    clippy::fn_params_excessive_bools,
    clippy::if_let_mutex,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::match_on_vec_items,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::pub_enum_variant_names,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::todo,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::verbose_file_reads,
    future_incompatible,
    missing_crate_level_docs,
    missing_doc_code_examples,
    missing_docs,
    rust_2018_idioms,
    unused_doc_comments
)]

pub use egui; // Re-export for user convenience

// ----------------------------------------------------------------------------

/// Implement this trait to write apps that can be compiled both natively using the [`egui_glium`](https://crates.io/crates/egui_glium) crate,
/// and deployed as a web site using the [`egui_web`](https://crates.io/crates/egui_web) crate.
pub trait App {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a [`egui::SidePanel`], [`egui::TopPanel`], [`egui::CentralPanel`], [`egui::Window`] or [`egui::Area`].
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut Frame<'_>);

    /// Called once before the first frame.
    /// Allows you to do setup code and to call `ctx.set_fonts()`.
    /// Optional.
    fn setup(&mut self, _ctx: &egui::CtxRef) {}

    /// If `true` a warm-up call to [`Self::update`] will be issued where
    /// `ctx.memory().everything_is_visible()` will be set to `true`.
    ///
    /// In this warm-up call, all painted shapes will be ignored.
    fn warm_up_enabled(&self) -> bool {
        false
    }

    /// Called once on start. Allows you to restore state.
    fn load(&mut self, _storage: &dyn Storage) {}

    /// Called on shutdown, and perhaps at regular intervals. Allows you to save state.
    ///
    /// On web the states is stored to "Local Storage".
    /// On native the path is picked using [`directories_next::ProjectDirs`](https://docs.rs/directories-next/latest/directories_next/struct.ProjectDirs.html) which is:
    /// * Linux:   `/home/UserName/.config/appname`
    /// * macOS:   `/Users/UserName/Library/Application Support/appname`
    /// * Windows: `C:\Users\UserName\AppData\Roaming\appname`
    fn save(&mut self, _storage: &mut dyn Storage) {}

    /// Called once on shutdown (before or after `save()`)
    fn on_exit(&mut self) {}

    // ---------
    // Settings:

    /// The name of your App.
    fn name(&self) -> &str;

    /// The initial size of the native window in points (logical pixels).
    fn initial_window_size(&self) -> Option<egui::Vec2> {
        None
    }

    /// Time between automatic calls to `save()`
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    /// Returns true if this app window should be resizable.
    fn is_resizable(&self) -> bool {
        true
    }

    /// The size limit of the web app canvas
    fn max_size_points(&self) -> egui::Vec2 {
        // Some browsers get slow with huge WebGL canvases, so we limit the size:
        egui::Vec2::new(1024.0, 2048.0)
    }

    /// Background color for the app, e.g. what is sent to `gl.clearColor`.
    /// This is the background of your windows if you don't set a central panel.
    fn clear_color(&self) -> egui::Rgba {
        // NOTE: a bright gray makes the shadows of the windows look weird.
        egui::Color32::from_rgb(12, 12, 12).into()
    }

    /// The application icon, e.g. in the Windows task bar etc.
    fn icon_data(&self) -> Option<IconData> {
        None
    }
}

/// Image data for the icon.
pub struct IconData {
    /// RGBA pixels.
    pub rgba: Vec<u8>,

    /// Image width. This should be a multiple of 4.
    pub width: u32,

    /// Image height. This should be a multiple of 4.
    pub height: u32,
}

/// Represents the surroundings of your app.
///
/// It provides methods to inspect the surroundings (are we on the web?),
/// allocate textures, do http requests, and change settings (e.g. window size).
pub struct Frame<'a>(backend::FrameBuilder<'a>);

impl<'a> Frame<'a> {
    /// True if you are in a web environment.
    pub fn is_web(&self) -> bool {
        self.info().web_info.is_some()
    }

    /// Information about the integration.
    pub fn info(&self) -> &IntegrationInfo {
        &self.0.info
    }

    /// A way to allocate textures.
    pub fn tex_allocator(&mut self) -> &mut dyn TextureAllocator {
        self.0.tex_allocator
    }

    /// Signal the app to stop/exit/quit the app (only works for native apps, not web apps).
    /// The framework will NOT quick immediately, but at the end of the this frame.
    pub fn quit(&mut self) {
        self.0.output.quit = true;
    }

    /// Set the desired inner size of the window (in egui points).
    pub fn set_window_size(&mut self, size: egui::Vec2) {
        self.0.output.window_size = Some(size);
    }

    /// Use [`egui::Context::set_pixels_per_point`] instead
    #[deprecated = "Use egui::Context::set_pixels_per_point instead"]
    pub fn set_pixels_per_point(&mut self, _: f32) {}

    /// If you need to request a repaint from another thread, clone this and send it to that other thread.
    pub fn repaint_signal(&self) -> std::sync::Arc<dyn RepaintSignal> {
        self.0.repaint_signal.clone()
    }

    /// Very simple Http fetch API.
    /// Calls the given callback when done.
    #[cfg(feature = "http")]
    pub fn http_fetch(
        &self,
        request: http::Request,
        on_done: impl 'static + Send + FnOnce(Result<http::Response, http::Error>),
    ) {
        self.0.http.fetch_dyn(request, Box::new(on_done))
    }
}

/// Information about the web environment (if applicable).
#[derive(Clone, Debug)]
pub struct WebInfo {
    /// e.g. "#fragment" part of "www.example.com/index.html#fragment".
    /// Note that the leading `#` is included in the string.
    /// Also known as "hash-link" or "anchor".
    pub web_location_hash: String,
}

/// Information about the integration passed to the use app each frame.
#[derive(Clone, Debug)]
pub struct IntegrationInfo {
    /// If the app is running in a Web context, this returns information about the environment.
    pub web_info: Option<WebInfo>,

    /// Seconds of cpu usage (in seconds) of UI code on the previous frame.
    /// `None` if this is the first frame.
    pub cpu_usage: Option<f32>,

    /// Local time. Used for the clock in the demo app.
    /// Set to `None` if you don't know.
    pub seconds_since_midnight: Option<f64>,

    /// The OS native pixels-per-point
    pub native_pixels_per_point: Option<f32>,
}

/// How to allocate textures (images) to use in [`egui`].
pub trait TextureAllocator {
    /// Allocate a new user texture.
    fn alloc_srgba_premultiplied(
        &mut self,
        size: (usize, usize),
        srgba_pixels: &[egui::Color32],
    ) -> egui::TextureId;

    /// Free the given texture.
    fn free(&mut self, id: egui::TextureId);
}

/// How to signal the [`egui`] integration that a repaint is required.
pub trait RepaintSignal: Send + Sync {
    /// This signals the [`egui`] integration that a repaint is required.
    /// This is meant to be called when a background process finishes in an async context and/or background thread.
    fn request_repaint(&self);
}

// ----------------------------------------------------------------------------

/// A place where you can store custom data in a way that persists when you restart the app.
///
/// On the web this is backed by [local storage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage).
/// On desktop this is backed by the file system.
pub trait Storage {
    /// Get the value for the given key.
    fn get_string(&self, key: &str) -> Option<String>;
    /// Set the value for the given key.
    fn set_string(&mut self, key: &str, value: String);

    /// write-to-disk or similar
    fn flush(&mut self);
}

/// Stores nothing.
#[derive(Clone, Default)]
pub struct DummyStorage {}

impl Storage for DummyStorage {
    fn get_string(&self, _key: &str) -> Option<String> {
        None
    }
    fn set_string(&mut self, _key: &str, _value: String) {}
    fn flush(&mut self) {}
}

/// Get an deserialize the JSON stored at the given key.
#[cfg(feature = "serde_json")]
pub fn get_value<T: serde::de::DeserializeOwned>(storage: &dyn Storage, key: &str) -> Option<T> {
    storage
        .get_string(key)
        .and_then(|value| serde_json::from_str(&value).ok())
}

/// Serialize the given value as JSON and store with the given key.
#[cfg(feature = "serde_json")]
pub fn set_value<T: serde::Serialize>(storage: &mut dyn Storage, key: &str, value: &T) {
    storage.set_string(key, serde_json::to_string_pretty(value).unwrap());
}

/// [`Storage`] key used for app
pub const APP_KEY: &str = "app";

// ----------------------------------------------------------------------------

#[cfg(feature = "http")]
/// `epi` supports simple HTTP requests with [`Frame::http_fetch`].
pub mod http {
    /// A simple http requests.
    pub struct Request {
        /// "GET", …
        pub method: String,
        /// https://…
        pub url: String,
        /// x-www-form-urlencoded body
        pub body: String,
    }

    impl Request {
        /// Create a `GET` requests with the given url.
        pub fn get(url: impl Into<String>) -> Self {
            Self {
                method: "GET".to_owned(),
                url: url.into(),
                body: "".to_string(),
            }
        }

        /// Create a `POST` requests with the give url and body.
        pub fn post(url: impl Into<String>, body: impl Into<String>) -> Self {
            Self {
                method: "POST".to_owned(),
                url: url.into(),
                body: body.into(),
            }
        }
    }

    /// Response from a completed HTTP request.
    pub struct Response {
        /// The URL we ended up at. This can differ from the request url when we have followed redirects.
        pub url: String,
        /// Did we get a 2xx response code?
        pub ok: bool,
        /// Status code (e.g. `404` for "File not found").
        pub status: u16,
        /// Status tex (e.g. "File not found" for status code `404`).
        pub status_text: String,

        /// Content-Type header, or empty string if missing.
        pub header_content_type: String,

        /// The raw bytes.
        pub bytes: Vec<u8>,

        /// UTF-8 decoded version of bytes.
        /// ONLY if `header_content_type` starts with "text" and bytes is UTF-8.
        pub text: Option<String>,
    }

    /// Possible errors does NOT include e.g. 404, which is NOT considered an error.
    pub type Error = String;
}

// ----------------------------------------------------------------------------

/// You only need to look here if you are writing a backend for `epi`.
pub mod backend {
    use super::*;

    /// Implements `Http` requests.
    #[cfg(feature = "http")]
    pub trait Http {
        /// Calls the given callback when done.
        fn fetch_dyn(
            &self,
            request: http::Request,
            on_done: Box<dyn FnOnce(Result<http::Response, http::Error>) + Send>,
        );
    }

    /// The data required by [`Frame`] each frame.
    pub struct FrameBuilder<'a> {
        /// Information about the integration.
        pub info: IntegrationInfo,
        /// A way to allocate textures (on integrations that support it).
        pub tex_allocator: &'a mut dyn TextureAllocator,
        /// Do http requests.
        #[cfg(feature = "http")]
        pub http: std::sync::Arc<dyn backend::Http>,
        /// Where the app can issue commands back to the integration.
        pub output: &'a mut AppOutput,
        /// If you need to request a repaint from another thread, clone this and send it to that other thread.
        pub repaint_signal: std::sync::Arc<dyn RepaintSignal>,
    }

    impl<'a> FrameBuilder<'a> {
        /// Wrap us in a [`Frame`] to send to [`App::update`].
        pub fn build(self) -> Frame<'a> {
            Frame(self)
        }
    }

    /// Action that can be taken by the user app.
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct AppOutput {
        /// Set to `true` to stop the app.
        /// This does nothing for web apps.
        pub quit: bool,

        /// Set to some size to resize the outer window (e.g. glium window) to this size.
        pub window_size: Option<egui::Vec2>,
    }
}
