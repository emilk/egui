//! Traits and helper for writing Egui apps.
//!
//! Egui can be used as a library, but you can also use it as a framework to write apps in.
//! This module defined the `App` trait that can be implemented and used with the `egui_web` and `egui_glium` crates.

use crate::Ui;

/// Implement this trait to write apps that can be compiled both natively using the `egui_glium` crate,
/// and deployed as a web site using the `egui_web` crate.
pub trait App {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn ui(&mut self, ui: &mut Ui, backend: &mut dyn Backend);

    /// Called once on shutdown. Allows you to save state.
    fn on_exit(&mut self, _storage: &mut dyn Storage) {}
}

// TODO: replace with manually calling `egui::Context::request_repaint()` each frame.
/// How the backend runs the app
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunMode {
    /// Rapint the UI all the time (at the display refresh rate of e.g. 60 Hz).
    /// This is good for games where things are constantly moving.
    /// This can also be achieved with `RunMode::Reactive` combined with calling `egui::Context::request_repaint()` each frame.
    Continuous,

    /// Only repaint when there are animations or input (mouse movement, keyboard input etc).
    /// This saves CPU.
    Reactive,
}

pub struct WebInfo {
    /// e.g. "#fragment" part of "www.example.com/index.html#fragment"
    pub web_location_hash: String,
}

pub trait Backend {
    fn run_mode(&self) -> RunMode;
    fn set_run_mode(&mut self, run_mode: RunMode);

    /// If the app is running in a Web context, this returns information about the environment.
    fn web_info(&self) -> Option<WebInfo> {
        None
    }

    /// excludes painting
    fn cpu_time(&self) -> f32;

    /// Smoothed frames per second
    fn fps(&self) -> f32;

    /// Signal the backend that we'd like to exit the app now.
    /// This does nothing for web apps.s
    fn quit(&mut self) {}
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
