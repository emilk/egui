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
    clippy::disallowed_method,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
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
    nonstandard_style,
    rust_2018_idioms,
    rustdoc::missing_crate_level_docs
)]
#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]
#![warn(missing_docs)] // Let's keep `epi` well-documented.

/// File storage which can be used by native backends.
#[cfg(feature = "file_storage")]
pub mod file_storage;

pub use egui; // Re-export for user convenience

use std::sync::{Arc, Mutex};

// ----------------------------------------------------------------------------

/// Implement this trait to write apps that can be compiled both natively using the [`egui_glium`](https://github.com/emilk/egui/tree/master/egui_glium) crate,
/// and deployed as a web site using the [`egui_web`](https://github.com/emilk/egui/tree/master/egui_web) crate.
pub trait App {
    /// Called each time the UI needs repainting, which may be many times per second.
    ///
    /// Put your widgets into a [`egui::SidePanel`], [`egui::TopBottomPanel`], [`egui::CentralPanel`], [`egui::Window`] or [`egui::Area`].
    ///
    /// The [`egui::Context`] and [`Frame`] can be cloned and saved if you like.
    ///
    /// To force a repaint, call either [`egui::Context::request_repaint`] during the call to `update`,
    /// or call [`Frame::request_repaint`] at any time (e.g. from another thread).
    fn update(&mut self, ctx: &egui::Context, frame: &Frame);

    /// Called once before the first frame.
    ///
    /// Allows you to do setup code, e.g to call [`egui::Context::set_fonts`],
    /// [`egui::Context::set_visuals`] etc.
    ///
    /// Also allows you to restore state, if there is a storage (required the "persistence" feature).
    fn setup(&mut self, _ctx: &egui::Context, _frame: &Frame, _storage: Option<&dyn Storage>) {}

    /// Called on shutdown, and perhaps at regular intervals. Allows you to save state.
    ///
    /// Only called when the "persistence" feature is enabled.
    ///
    /// On web the states is stored to "Local Storage".
    /// On native the path is picked using [`directories_next::ProjectDirs::data_dir`](https://docs.rs/directories-next/2.0.0/directories_next/struct.ProjectDirs.html#method.data_dir) which is:
    /// * Linux:   `/home/UserName/.local/share/APPNAME`
    /// * macOS:   `/Users/UserName/Library/Application Support/APPNAME`
    /// * Windows: `C:\Users\UserName\AppData\Roaming\APPNAME`
    ///
    /// where `APPNAME` is what is returned by [`Self::name()`].
    fn save(&mut self, _storage: &mut dyn Storage) {}

    /// Called before an exit that can be aborted.
    /// By returning `false` the exit will be aborted. To continue the exit return `true`.
    ///
    /// A scenario where this method will be run is after pressing the close button on a native
    /// window, which allows you to ask the user whether they want to do something before exiting.
    /// See the example `eframe/examples/confirm_exit.rs` for practical usage.
    ///
    /// It will _not_ be called on the web or when the window is forcefully closed.
    fn on_exit_event(&mut self) -> bool {
        true
    }

    /// Called once on shutdown (before or after [`Self::save`]). If you need to abort an exit use
    /// [`Self::on_exit_event`]
    fn on_exit(&mut self) {}

    // ---------
    // Settings:

    /// The name of your App, used for the title bar of native windows
    /// and the save location of persistence (see [`Self::save`]).
    fn name(&self) -> &str;

    /// Time between automatic calls to [`Self::save`]
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    /// The size limit of the web app canvas.
    ///
    /// By default the size if limited to 1024x2048.
    ///
    /// A larger canvas can lead to bad frame rates on some browsers on some platforms.
    /// In particular, Firefox on Mac and Linux is really bad at handling large WebGL canvases:
    /// <https://bugzilla.mozilla.org/show_bug.cgi?id=1010527#c0> (unfixed since 2014).
    fn max_size_points(&self) -> egui::Vec2 {
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

    /// Controls whether or not the native window position and size will be
    /// persisted (only if the "persistence" feature is enabled).
    fn persist_native_window(&self) -> bool {
        true
    }

    /// Controls whether or not the egui memory (window positions etc) will be
    /// persisted (only if the "persistence" feature is enabled).
    fn persist_egui_memory(&self) -> bool {
        true
    }

    /// If `true` a warm-up call to [`Self::update`] will be issued where
    /// `ctx.memory().everything_is_visible()` will be set to `true`.
    ///
    /// This can help pre-caching resources loaded by different parts of the UI, preventing stutter later on.
    ///
    /// In this warm-up call, all painted shapes will be ignored.
    ///
    /// The default is `false`, and it is unlikely you will want to change this.
    fn warm_up_enabled(&self) -> bool {
        false
    }
}

/// Options controlling the behavior of a native window.
///
/// Only a single native window is currently supported.
#[derive(Clone)]
pub struct NativeOptions {
    /// Sets whether or not the window will always be on top of other windows.
    pub always_on_top: bool,

    /// Show window in maximized mode
    pub maximized: bool,

    /// On desktop: add window decorations (i.e. a frame around your app)?
    /// If false it will be difficult to move and resize the app.
    pub decorated: bool,

    /// On Windows: enable drag and drop support. Drag and drop can
    /// not be disabled on other platforms.
    ///
    /// See [winit's documentation][drag_and_drop] for information on why you
    /// might want to disable this on windows.
    ///
    /// [drag_and_drop]: https://docs.rs/winit/latest/x86_64-pc-windows-msvc/winit/platform/windows/trait.WindowBuilderExtWindows.html#tymethod.with_drag_and_drop
    pub drag_and_drop_support: bool,

    /// The application icon, e.g. in the Windows task bar etc.
    pub icon_data: Option<IconData>,

    /// The initial (inner) position of the native window in points (logical pixels).
    pub initial_window_pos: Option<egui::Pos2>,

    /// The initial inner size of the native window in points (logical pixels).
    pub initial_window_size: Option<egui::Vec2>,

    /// The minimum inner window size
    pub min_window_size: Option<egui::Vec2>,

    /// The maximum inner window size
    pub max_window_size: Option<egui::Vec2>,

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
            drag_and_drop_support: true,
            icon_data: None,
            initial_window_pos: None,
            initial_window_size: None,
            min_window_size: None,
            max_window_size: None,
            resizable: true,
            transparent: false,
        }
    }
}

/// Image data for the icon.
#[derive(Clone)]
pub struct IconData {
    /// RGBA pixels, unmultiplied.
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
///
/// [`Frame`] is cheap to clone and is safe to pass to other threads.
#[derive(Clone)]
pub struct Frame(pub Arc<Mutex<backend::FrameData>>);

impl Frame {
    /// Create a `Frame` - called by the integration.
    #[doc(hidden)]
    pub fn new(frame_data: backend::FrameData) -> Self {
        Self(Arc::new(Mutex::new(frame_data)))
    }

    /// Access the underlying [`backend::FrameData`].
    #[doc(hidden)]
    #[inline]
    pub fn lock(&self) -> std::sync::MutexGuard<'_, backend::FrameData> {
        self.0.lock().unwrap()
    }

    /// True if you are in a web environment.
    pub fn is_web(&self) -> bool {
        self.lock().info.web_info.is_some()
    }

    /// Information about the integration.
    pub fn info(&self) -> IntegrationInfo {
        self.lock().info.clone()
    }

    /// Signal the app to stop/exit/quit the app (only works for native apps, not web apps).
    /// The framework will not quit immediately, but at the end of the this frame.
    pub fn quit(&self) {
        self.lock().output.quit = true;
    }

    /// Set the desired inner size of the window (in egui points).
    pub fn set_window_size(&self, size: egui::Vec2) {
        self.lock().output.window_size = Some(size);
    }

    /// Set the desired title of the window.
    pub fn set_window_title(&self, title: &str) {
        self.lock().output.window_title = Some(title.to_owned());
    }

    /// Set whether to show window decorations (i.e. a frame around you app).
    /// If false it will be difficult to move and resize the app.
    pub fn set_decorations(&self, decorated: bool) {
        self.lock().output.decorated = Some(decorated);
    }

    /// When called, the native window will follow the
    /// movement of the cursor while the primary mouse button is down.
    ///
    /// Does not work on the web.
    pub fn drag_window(&self) {
        self.lock().output.drag_window = true;
    }

    /// This signals the [`egui`] integration that a repaint is required.
    ///
    /// Call this e.g. when a background process finishes in an async context and/or background thread.
    pub fn request_repaint(&self) {
        self.lock().repaint_signal.request_repaint();
    }

    /// for integrations only: call once per frame
    pub fn take_app_output(&self) -> crate::backend::AppOutput {
        std::mem::take(&mut self.lock().output)
    }
}

#[cfg(test)]
#[test]
fn frame_impl_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Frame>();
}

/// Information about the web environment (if applicable).
#[derive(Clone, Debug)]
pub struct WebInfo {
    /// Information about the URL.
    pub location: Location,
}

/// Information about the URL.
///
/// Everything has been percent decoded (`%20` -> ` ` etc).
#[derive(Clone, Debug)]
pub struct Location {
    /// The full URL (`location.href`) without the hash.
    ///
    /// Example: `"http://www.example.com:80/index.html?foo=bar"`.
    pub url: String,

    /// `location.protocol`
    ///
    /// Example: `"http:"`.
    pub protocol: String,

    /// `location.host`
    ///
    /// Example: `"example.com:80"`.
    pub host: String,

    /// `location.hostname`
    ///
    /// Example: `"example.com"`.
    pub hostname: String,

    /// `location.port`
    ///
    /// Example: `"80"`.
    pub port: String,

    /// The "#fragment" part of "www.example.com/index.html?query#fragment".
    ///
    /// Note that the leading `#` is included in the string.
    /// Also known as "hash-link" or "anchor".
    pub hash: String,

    /// The "query" part of "www.example.com/index.html?query#fragment".
    ///
    /// Note that the leading `?` is NOT included in the string.
    ///
    /// Use [`Self::web_query_map]` to get the parsed version of it.
    pub query: String,

    /// The parsed "query" part of "www.example.com/index.html?query#fragment".
    ///
    /// "foo=42&bar%20" is parsed as `{"foo": "42",  "bar ": ""}`
    pub query_map: std::collections::BTreeMap<String, String>,

    /// `location.origin`
    ///
    /// Example: `"http://www.example.com:80"`.
    pub origin: String,
}

/// Information about the integration passed to the use app each frame.
#[derive(Clone, Debug)]
pub struct IntegrationInfo {
    /// The name of the integration, e.g. `egui_web`, `egui_glium`, `egui_glow`
    pub name: &'static str,

    /// If the app is running in a Web context, this returns information about the environment.
    pub web_info: Option<WebInfo>,

    /// Does the system prefer dark mode (over light mode)?
    /// `None` means "don't know".
    pub prefer_dark_mode: Option<bool>,

    /// Seconds of cpu usage (in seconds) of UI code on the previous frame.
    /// `None` if this is the first frame.
    pub cpu_usage: Option<f32>,

    /// The OS native pixels-per-point
    pub native_pixels_per_point: Option<f32>,
}

/// Abstraction for platform dependent texture reference
pub trait NativeTexture {
    /// The native texture type.
    type Texture;

    /// Bind native texture to an egui texture id.
    fn register_native_texture(&mut self, native: Self::Texture) -> egui::TextureId;

    /// Change what texture the given id refers to.
    fn replace_native_texture(&mut self, id: egui::TextureId, replacing: Self::Texture);
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
    storage.set_string(key, ron::ser::to_string(value).unwrap());
}

/// [`Storage`] key used for app
pub const APP_KEY: &str = "app";

// ----------------------------------------------------------------------------

/// You only need to look here if you are writing a backend for `epi`.
pub mod backend {
    use super::*;

    /// How to signal the [`egui`] integration that a repaint is required.
    pub trait RepaintSignal: Send + Sync {
        /// This signals the [`egui`] integration that a repaint is required.
        ///
        /// Call this e.g. when a background process finishes in an async context and/or background thread.
        fn request_repaint(&self);
    }

    /// The data required by [`Frame`] each frame.
    pub struct FrameData {
        /// Information about the integration.
        pub info: IntegrationInfo,

        /// Where the app can issue commands back to the integration.
        pub output: AppOutput,

        /// If you need to request a repaint from another thread, clone this and send it to that other thread.
        pub repaint_signal: std::sync::Arc<dyn RepaintSignal>,
    }

    /// Action that can be taken by the user app.
    #[derive(Default)]
    #[must_use]
    pub struct AppOutput {
        /// Set to `true` to stop the app.
        /// This does nothing for web apps.
        pub quit: bool,

        /// Set to some size to resize the outer window (e.g. glium window) to this size.
        pub window_size: Option<egui::Vec2>,

        /// Set to some string to rename the outer window (e.g. glium window) to this title.
        pub window_title: Option<String>,

        /// Set to some bool to change window decorations.
        pub decorated: Option<bool>,

        /// Set to true to drag window while primary mouse button is down.
        pub drag_window: bool,
    }
}
