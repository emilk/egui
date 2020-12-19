//! Traits and helper for writing Egui apps.
//!
//! This module is very experimental, and you don't need to use it.
//!
//! Egui can be used as a library, but you can also use it as a framework to write apps in.
//! This module defined the `App` trait that can be implemented and used with the `egui_web` and `egui_glium` crates.

// TODO: move egui/src/app.rs to own crate, e.g. egui_framework ?

/// Implement this trait to write apps that can be compiled both natively using the [`egui_glium`](https://crates.io/crates/egui_glium) crate,
/// and deployed as a web site using the [`egui_web`](https://crates.io/crates/egui_web) crate.
pub trait App {
    /// The name of your App.
    fn name(&self) -> &str;

    /// Background color for the app.
    /// e.g. what is sent to `gl.clearColor`
    fn clear_color(&self) -> crate::Rgba {
        crate::Srgba::from_rgb(16, 16, 16).into()
    }

    /// Called once before the first frame.
    /// Allows you to do setup code and to call `ctx.set_fonts()`.
    /// Optional.
    fn setup(&mut self, _ctx: &crate::CtxRef) {}

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn ui(&mut self, ctx: &crate::CtxRef, integration_context: &mut IntegrationContext<'_>);

    /// Called once on shutdown. Allows you to save state.
    fn on_exit(&mut self, _storage: &mut dyn Storage) {}
}

pub struct IntegrationContext<'a> {
    /// Information about the integration.
    pub info: IntegrationInfo,
    /// A way to allocate textures (on integrations that support it).
    pub tex_allocator: Option<&'a mut dyn TextureAllocator>,
    /// Where the app can issue commands back to the integration.
    pub output: AppOutput,
    /// If you need to request a repaint from another thread, clone this and give to that other thread
    pub repaint_signal: std::sync::Arc<dyn RepaintSignal>,
}

#[derive(Clone, Debug)]
pub struct WebInfo {
    /// e.g. "#fragment" part of "www.example.com/index.html#fragment"
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

/// Action that can be taken by the user app.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AppOutput {
    /// Set to `true` to stop the app.
    /// This does nothing for web apps.
    pub quit: bool,

    /// Set to some size to resize the outer window (e.g. glium window) to this size.
    pub window_size: Option<crate::Vec2>,

    /// If the app sets this, change the `pixels_per_point` of Egui to this next frame.
    pub pixels_per_point: Option<f32>,
}

pub trait TextureAllocator {
    /// A.locate a new user texture.
    fn alloc(&mut self) -> crate::TextureId;

    /// Set or change the pixels of a user texture.
    fn set_srgba_premultiplied(
        &mut self,
        id: crate::TextureId,
        size: (usize, usize),
        srgba_pixels: &[crate::Srgba],
    );

    /// Free the given texture.
    fn free(&mut self, id: crate::TextureId);
}

pub trait RepaintSignal: Send {
    /// This signals the Egui integration that a repaint is required.
    /// This is meant to be called when a background process finishes in an async context and/or background thread.
    fn request_repaint(&self);
}

/// A place where you can store custom data in a way that persists when you restart the app.
///
/// On the web this is backed by [local storage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage).
/// On desktop this is backed by the file system.
pub trait Storage {
    fn get_string(&self, key: &str) -> Option<String>;
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

#[cfg(feature = "serde_json")]
pub fn get_value<T: serde::de::DeserializeOwned>(storage: &dyn Storage, key: &str) -> Option<T> {
    storage
        .get_string(key)
        .and_then(|value| serde_json::from_str(&value).ok())
}

#[cfg(feature = "serde_json")]
pub fn set_value<T: serde::Serialize>(storage: &mut dyn Storage, key: &str, value: &T) {
    storage.set_string(key, serde_json::to_string(value).unwrap());
}

/// storage key used for app
pub const APP_KEY: &str = "app";
