//! Traits and helper for writing Egui apps.
//!
//! This module is very experimental, and you don't need to use it.
//!
//! Egui can be used as a library, but you can also use it as a framework to write apps in.
//! This module defined the `App` trait that can be implemented and used with the `egui_web` and `egui_glium` crates.

// TODO: move egui/src/app.rs to own crate, e.g. egui_framework ?

use crate::Ui;

/// Implement this trait to write apps that can be compiled both natively using the [`egui_glium`](https://crates.io/crates/egui_glium) crate,
/// and deployed as a web site using the [`egui_web`](https://crates.io/crates/egui_web) crate.
pub trait App {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn ui(
        &mut self,
        ui: &mut Ui,
        info: &BackendInfo,
        tex_allocator: Option<&mut dyn TextureAllocator>,
    ) -> AppOutput;

    /// Called once on shutdown. Allows you to save state.
    fn on_exit(&mut self, _storage: &mut dyn Storage) {}
}

pub struct WebInfo {
    /// e.g. "#fragment" part of "www.example.com/index.html#fragment"
    pub web_location_hash: String,
}

/// Information about the backend passed to the use app each frame.
pub struct BackendInfo {
    /// If the app is running in a Web context, this returns information about the environment.
    pub web_info: Option<WebInfo>,

    /// Seconds of cpu usage (in seconds) of UI code on the previous frame.
    /// `None` if this is the first frame.
    pub cpu_usage: Option<f32>,

    /// Local time. Used for the clock in the demo app.
    /// Set to `None` if you don't know.
    pub seconds_since_midnight: Option<f64>,
}

/// Action that can be taken by the user app.
#[derive(Clone, Copy, Debug, Default)]
pub struct AppOutput {
    /// Set to `true` to stop the app.
    /// This does nothing for web apps.
    pub quit: bool,
}

pub trait TextureAllocator {
    /// Allocate a user texture (EXPERIMENTAL!)
    fn new_texture_srgba_premultiplied(
        &mut self,
        size: (usize, usize),
        pixels: &[crate::Srgba],
    ) -> crate::TextureId;
}

/// A place where you can store custom data in a way that persists when you restart the app.
///
/// On the web this is backed by [local storage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage).
/// On desktop this is backed by the file system.
pub trait Storage {
    fn get_string(&self, key: &str) -> Option<&str>;
    fn set_string(&mut self, key: &str, value: String);
}

#[cfg(feature = "serde_json")]
pub fn get_value<T: serde::de::DeserializeOwned>(storage: &dyn Storage, key: &str) -> Option<T> {
    storage
        .get_string(key)
        .and_then(|value| serde_json::from_str(value).ok())
}

#[cfg(feature = "serde_json")]
pub fn set_value<T: serde::Serialize>(storage: &mut dyn Storage, key: &str, value: &T) {
    storage.set_string(key, serde_json::to_string(value).unwrap());
}

/// storage key used for app
pub const APP_KEY: &str = "app";
