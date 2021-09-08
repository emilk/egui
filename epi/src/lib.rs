//! Backend-agnostic interface for writing apps using [`egui`].
//!
//! `epi` provides interfaces for window management and serialization.
//! An app written for `epi` can then be plugged into [`eframe`](https://docs.rs/eframe),
//! the egui framework crate.
//!
//! Start by looking at the [`App`] trait, and implement [`App::update`].

// Forbid warnings in release builds:
#![cfg_attr(not(debug_assertions), deny(warnings))]
#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::if_let_mutex,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values,
    future_incompatible,
    missing_crate_level_docs,
    nonstandard_style,
    rust_2018_idioms
)]
#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]
#![warn(missing_docs)] // Let's keep `epi` well-documented.

pub use egui; // Re-export for user convenience

// ----------------------------------------------------------------------------

/// Implement this trait to write apps that can be compiled both natively using the [`egui_glium`](https://crates.io/crates/egui_glium) crate,
/// and deployed as a web site using the [`egui_web`](https://crates.io/crates/egui_web) crate.
pub trait App {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a [`egui::SidePanel`], [`egui::TopBottomPanel`], [`egui::CentralPanel`], [`egui::Window`] or [`egui::Area`].
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut Frame<'_>);

    /// Called once before the first frame.
    ///
    /// Allows you to do setup code, e.g to call `[egui::Context::set_fonts]`,
    /// `[egui::Context::set_visuals]` etc.
    ///
    /// Also allows you to restore state, if there is a storage.
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut Frame<'_>,
        _storage: Option<&dyn Storage>,
    ) {
    }

    /// If `true` a warm-up call to [`Self::update`] will be issued where
    /// `ctx.memory().everything_is_visible()` will be set to `true`.
    ///
    /// In this warm-up call, all painted shapes will be ignored.
    fn warm_up_enabled(&self) -> bool {
        false
    }

    /// Called on shutdown, and perhaps at regular intervals. Allows you to save state.
    ///
    /// On web the states is stored to "Local Storage".
    /// On native the path is picked using [`directories_next::ProjectDirs::data_dir`](https://docs.rs/directories-next/2.0.0/directories_next/struct.ProjectDirs.html#method.data_dir) which is:
    /// * Linux:   `/home/UserName/.local/share/APPNAME`
    /// * macOS:   `/Users/UserName/Library/Application Support/APPNAME`
    /// * Windows: `C:\Users\UserName\AppData\Roaming\APPNAME`
    ///
    /// where `APPNAME` is what is returned by [`Self::name()`].
    fn save(&mut self, _storage: &mut dyn Storage) {}

    /// Called once on shutdown (before or after [`Self::save`])
    fn on_exit(&mut self) {}

    // ---------
    // Settings:

    /// The name of your App.
    fn name(&self) -> &str;

    /// Time between automatic calls to [`Self::save`]
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    /// The size limit of the web app canvas.
    fn max_size_points(&self) -> egui::Vec2 {
        // Some browsers get slow with huge WebGL canvases, so we limit the size:
        egui::Vec2::new(1024.0, 2048.0)
    }

    /// Background color for the app, e.g. what is sent to `gl.clearColor`.
    /// This is the background of your windows if you don't set a central panel.
    fn clear_color(&self) -> egui::Rgba {
        // NOTE: a bright gray makes the shadows of the windows look weird.
        // We use a bit of transparency so that if the user switches on the
        // `transparent()` option they get immediate results.
        egui::Color32::from_rgba_unmultiplied(12, 12, 12, 180).into()
    }
}

/// Options controlling the behavior of a native window
#[derive(Clone)]
pub struct NativeOptions {
    /// Sets whether or not the window will always be on top of other windows.
    pub always_on_top: bool,

    /// Show window in maximized mode
    pub maximized: bool,

    /// On desktop: add window decorations (i.e. a frame around your app)?
    /// If false it will be difficult to move and resize the app.
    pub decorated: bool,

    /// On Windows: enable drag and drop support.
    /// Default is `false` to avoid issues with crates such as cpal which
    /// uses that use multi-threaded COM API <https://github.com/rust-windowing/winit/pull/1524>
    pub drag_and_drop_support: bool,

    /// The application icon, e.g. in the Windows task bar etc.
    pub icon_data: Option<IconData>,

    /// The initial size of the native window in points (logical pixels).
    pub initial_window_size: Option<egui::Vec2>,

    /// Should the app window be resizable?
    pub resizable: bool,

    /// On desktop: make the window transparent.
    /// You control the transparency with [`App::clear_color()`].
    /// You should avoid having a [`egui::CentralPanel`], or make sure its frame is also transparent.
    pub transparent: bool,
}

impl Default for NativeOptions {
    fn default() -> Self {
        Self {
            always_on_top: false,
            maximized: false,
            decorated: true,
            drag_and_drop_support: false,
            icon_data: None,
            initial_window_size: None,
            resizable: true,
            transparent: false,
        }
    }
}

/// Image data for the icon.
#[derive(Clone)]
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
/// allocate textures, and change settings (e.g. window size).
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
    /// The framework will not quit immediately, but at the end of the this frame.
    pub fn quit(&mut self) {
        self.0.output.quit = true;
    }

    /// Set the desired inner size of the window (in egui points).
    pub fn set_window_size(&mut self, size: egui::Vec2) {
        self.0.output.window_size = Some(size);
    }

    /// Set whether to show window decorations (i.e. a frame around you app).
    /// If false it will be difficult to move and resize the app.
    pub fn set_decorations(&mut self, decorated: bool) {
        self.0.output.decorated = Some(decorated);
    }

    /// If you need to request a repaint from another thread, clone this and send it to that other thread.
    pub fn repaint_signal(&self) -> std::sync::Arc<dyn RepaintSignal> {
        self.0.repaint_signal.clone()
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

    /// Does the system prefer dark mode (over light mode)?
    /// `None` means "don't know".
    pub prefer_dark_mode: Option<bool>,

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
    ///
    /// There is no way to change a texture.
    /// Instead allocate a new texture and free the previous one with [`Self::free`].
    fn alloc_srgba_premultiplied(
        &mut self,
        size: (usize, usize),
        srgba_pixels: &[egui::Color32],
    ) -> egui::TextureId;

    /// Free the given texture.
    fn free(&mut self, id: egui::TextureId);
}

/// Abstraction for platform dependent texture reference
pub trait NativeTexture {
    /// The native texture type.
    type Texture;

    /// Bind native texture to egui texture
    fn register_native_texture(&mut self, native: Self::Texture) -> egui::TextureId;

    /// Change id's actual pointing texture
    /// only for user texture
    fn replace_native_texture(&mut self, id: egui::TextureId, replacing: Self::Texture);
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

/// Get and deserialize the [RON](https://github.com/ron-rs/ron) stored at the given key.
#[cfg(feature = "ron")]
pub fn get_value<T: serde::de::DeserializeOwned>(storage: &dyn Storage, key: &str) -> Option<T> {
    storage
        .get_string(key)
        .and_then(|value| ron::from_str(&value).ok())
}

/// Serialize the given value as [RON](https://github.com/ron-rs/ron) and store with the given key.
#[cfg(feature = "ron")]
pub fn set_value<T: serde::Serialize>(storage: &mut dyn Storage, key: &str, value: &T) {
    storage.set_string(
        key,
        ron::ser::to_string_pretty(value, Default::default()).unwrap(),
    );
}

/// [`Storage`] key used for app
pub const APP_KEY: &str = "app";

// ----------------------------------------------------------------------------

/// You only need to look here if you are writing a backend for `epi`.
pub mod backend {
    use super::*;

    /// The data required by [`Frame`] each frame.
    pub struct FrameBuilder<'a> {
        /// Information about the integration.
        pub info: IntegrationInfo,
        /// A way to allocate textures (on integrations that support it).
        pub tex_allocator: &'a mut dyn TextureAllocator,
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

        /// Set to some bool to change window decorations
        pub decorated: Option<bool>,
    }
}
