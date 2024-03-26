//! Platform-agnostic interface for writing apps using [`egui`] (epi = egui programming interface).
//!
//! `epi` provides interfaces for window management and serialization.
//!
//! Start by looking at the [`App`] trait, and implement [`App::update`].

#![warn(missing_docs)] // Let's keep `epi` well-documented.

#[cfg(target_arch = "wasm32")]
use std::any::Any;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
pub use crate::native::winit_integration::UserEvent;

#[cfg(not(target_arch = "wasm32"))]
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};
#[cfg(not(target_arch = "wasm32"))]
use static_assertions::assert_not_impl_any;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
pub use winit::{event_loop::EventLoopBuilder, window::WindowBuilder};

/// Hook into the building of an event loop before it is run
///
/// You can configure any platform specific details required on top of the default configuration
/// done by `EFrame`.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
pub type EventLoopBuilderHook = Box<dyn FnOnce(&mut EventLoopBuilder<UserEvent>)>;

/// Hook into the building of a the native window.
///
/// You can configure any platform specific details required on top of the default configuration
/// done by `eframe`.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
pub type WindowBuilderHook = Box<dyn FnOnce(egui::ViewportBuilder) -> egui::ViewportBuilder>;

/// This is how your app is created.
///
/// You can use the [`CreationContext`] to setup egui, restore state, setup OpenGL things, etc.
pub type AppCreator = Box<dyn FnOnce(&CreationContext<'_>) -> Box<dyn App>>;

/// Data that is passed to [`AppCreator`] that can be used to setup and initialize your app.
pub struct CreationContext<'s> {
    /// The egui Context.
    ///
    /// You can use this to customize the look of egui, e.g to call [`egui::Context::set_fonts`],
    /// [`egui::Context::set_visuals`] etc.
    pub egui_ctx: egui::Context,

    /// Information about the surrounding environment.
    pub integration_info: IntegrationInfo,

    /// You can use the storage to restore app state(requires the "persistence" feature).
    pub storage: Option<&'s dyn Storage>,

    /// The [`glow::Context`] allows you to initialize OpenGL resources (e.g. shaders) that
    /// you might want to use later from a [`egui::PaintCallback`].
    ///
    /// Only available when compiling with the `glow` feature and using [`Renderer::Glow`].
    #[cfg(feature = "glow")]
    pub gl: Option<std::sync::Arc<glow::Context>>,

    /// The `get_proc_address` wrapper of underlying GL context
    #[cfg(feature = "glow")]
    pub get_proc_address: Option<&'s dyn Fn(&std::ffi::CStr) -> *const std::ffi::c_void>,

    /// The underlying WGPU render state.
    ///
    /// Only available when compiling with the `wgpu` feature and using [`Renderer::Wgpu`].
    ///
    /// Can be used to manage GPU resources for custom rendering with WGPU using [`egui::PaintCallback`]s.
    #[cfg(feature = "wgpu")]
    pub wgpu_render_state: Option<egui_wgpu::RenderState>,

    /// Raw platform window handle
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_window_handle: Result<RawWindowHandle, HandleError>,

    /// Raw platform display handle for window
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_display_handle: Result<RawDisplayHandle, HandleError>,
}

#[allow(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
impl HasWindowHandle for CreationContext<'_> {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // Safety: the lifetime is correct.
        unsafe { Ok(WindowHandle::borrow_raw(self.raw_window_handle.clone()?)) }
    }
}

#[allow(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
impl HasDisplayHandle for CreationContext<'_> {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // Safety: the lifetime is correct.
        unsafe { Ok(DisplayHandle::borrow_raw(self.raw_display_handle.clone()?)) }
    }
}

// ----------------------------------------------------------------------------

/// Implement this trait to write apps that can be compiled for both web/wasm and desktop/native using [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe).
pub trait App {
    /// Called each time the UI needs repainting, which may be many times per second.
    ///
    /// Put your widgets into a [`egui::SidePanel`], [`egui::TopBottomPanel`], [`egui::CentralPanel`], [`egui::Window`] or [`egui::Area`].
    ///
    /// The [`egui::Context`] can be cloned and saved if you like.
    ///
    /// To force a repaint, call [`egui::Context::request_repaint`] at any time (e.g. from another thread).
    ///
    /// This is called for the root viewport ([`egui::ViewportId::ROOT`]).
    /// Use [`egui::Context::show_viewport_deferred`] to spawn additional viewports (windows).
    /// (A "viewport" in egui means an native OS window).
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame);

    /// Get a handle to the app.
    ///
    /// Can be used from web to interact or other external context.
    ///
    /// You need to implement this if you want to be able to access the application from JS using [`crate::WebRunner::app_mut`].
    ///
    /// This is needed because downcasting `Box<dyn App>` -> `Box<dyn Any>` to get &`ConcreteApp` is not simple in current rust.
    ///
    /// Just copy-paste this as your implementation:
    /// ```ignore
    /// #[cfg(target_arch = "wasm32")]
    /// fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
    ///     Some(&mut *self)
    /// }
    /// ```
    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }

    /// Called on shutdown, and perhaps at regular intervals. Allows you to save state.
    ///
    /// Only called when the "persistence" feature is enabled.
    ///
    /// On web the state is stored to "Local Storage".
    ///
    /// On native the path is picked using [`crate::storage_dir`].
    fn save(&mut self, _storage: &mut dyn Storage) {}

    /// Called once on shutdown, after [`Self::save`].
    ///
    /// If you need to abort an exit check `ctx.input(|i| i.viewport().close_requested())`
    /// and respond with [`egui::ViewportCommand::CancelClose`].
    ///
    /// To get a [`glow`] context you need to compile with the `glow` feature flag,
    /// and run eframe with the glow backend.
    #[cfg(feature = "glow")]
    fn on_exit(&mut self, _gl: Option<&glow::Context>) {}

    /// Called once on shutdown, after [`Self::save`].
    ///
    /// If you need to abort an exit use [`Self::on_close_event`].
    #[cfg(not(feature = "glow"))]
    fn on_exit(&mut self) {}

    // ---------
    // Settings:

    /// Time between automatic calls to [`Self::save`]
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    /// Background color values for the app, e.g. what is sent to `gl.clearColor`.
    ///
    /// This is the background of your windows if you don't set a central panel.
    ///
    /// ATTENTION:
    /// Since these float values go to the render as-is, any color space conversion as done
    /// e.g. by converting from [`egui::Color32`] to [`egui::Rgba`] may cause incorrect results.
    /// egui recommends that rendering backends use a normal "gamma-space" (non-sRGB-aware) blending,
    ///  which means the values you return here should also be in `sRGB` gamma-space in the 0-1 range.
    /// You can use [`egui::Color32::to_normalized_gamma_f32`] for this.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // NOTE: a bright gray makes the shadows of the windows look weird.
        // We use a bit of transparency so that if the user switches on the
        // `transparent()` option they get immediate results.
        egui::Color32::from_rgba_unmultiplied(12, 12, 12, 180).to_normalized_gamma_f32()

        // _visuals.window_fill() would also be a natural choice
    }

    /// Controls whether or not the egui memory (window positions etc) will be
    /// persisted (only if the "persistence" feature is enabled).
    fn persist_egui_memory(&self) -> bool {
        true
    }

    /// A hook for manipulating or filtering raw input before it is processed by [`Self::update`].
    ///
    /// This function provides a way to modify or filter input events before they are processed by egui.
    ///
    /// It can be used to prevent specific keyboard shortcuts or mouse events from being processed by egui.
    ///
    /// Additionally, it can be used to inject custom keyboard or mouse events into the input stream, which can be useful for implementing features like a virtual keyboard.
    ///
    /// # Arguments
    ///
    /// * `_ctx` - The context of the egui, which provides access to the current state of the egui.
    /// * `_raw_input` - The raw input events that are about to be processed. This can be modified to change the input that egui processes.
    ///
    /// # Note
    ///
    /// This function does not return a value. Any changes to the input should be made directly to `_raw_input`.
    fn raw_input_hook(&mut self, _ctx: &egui::Context, _raw_input: &mut egui::RawInput) {}
}

/// Selects the level of hardware graphics acceleration.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HardwareAcceleration {
    /// Require graphics acceleration.
    Required,

    /// Prefer graphics acceleration, but fall back to software.
    Preferred,

    /// Do NOT use graphics acceleration.
    ///
    /// On some platforms (MacOS) this is ignored and treated the same as [`Self::Preferred`].
    Off,
}

/// Options controlling the behavior of a native window.
///
/// Additional windows can be opened using (egui viewports)[`egui::viewport`].
///
/// Set the window title and size using [`Self::viewport`].
///
/// ### Application id
/// [`egui::ViewportBuilder::with_app_id`] is used for determining the folder to persist the app to.
///
/// On native the path is picked using [`crate::storage_dir`].
///
/// If you don't set an app id, the title argument to [`crate::run_native`]
/// will be used as app id instead.
#[cfg(not(target_arch = "wasm32"))]
pub struct NativeOptions {
    /// Controls the native window of the root viewport.
    ///
    /// This is where you set things like window title and size.
    ///
    /// If you don't set an icon, a default egui icon will be used.
    /// To avoid this, set the icon to [`egui::IconData::default`].
    pub viewport: egui::ViewportBuilder,

    /// Turn on vertical syncing, limiting the FPS to the display refresh rate.
    ///
    /// The default is `true`.
    pub vsync: bool,

    /// Set the level of the multisampling anti-aliasing (MSAA).
    ///
    /// Must be a power-of-two. Higher = more smooth 3D.
    ///
    /// A value of `0` turns it off (default).
    ///
    /// `egui` already performs anti-aliasing via "feathering"
    /// (controlled by [`egui::epaint::TessellationOptions`]),
    /// but if you are embedding 3D in egui you may want to turn on multisampling.
    pub multisampling: u16,

    /// Sets the number of bits in the depth buffer.
    ///
    /// `egui` doesn't need the depth buffer, so the default value is 0.
    pub depth_buffer: u8,

    /// Sets the number of bits in the stencil buffer.
    ///
    /// `egui` doesn't need the stencil buffer, so the default value is 0.
    pub stencil_buffer: u8,

    /// Specify whether or not hardware acceleration is preferred, required, or not.
    ///
    /// Default: [`HardwareAcceleration::Preferred`].
    pub hardware_acceleration: HardwareAcceleration,

    /// What rendering backend to use.
    #[cfg(any(feature = "glow", feature = "wgpu"))]
    pub renderer: Renderer,

    /// Try to detect and follow the system preferred setting for dark vs light mode.
    ///
    /// The theme will automatically change when the dark vs light mode preference is changed.
    ///
    /// Does not work on Linux (see <https://github.com/rust-windowing/winit/issues/1549>).
    ///
    /// See also [`Self::default_theme`].
    pub follow_system_theme: bool,

    /// Which theme to use in case [`Self::follow_system_theme`] is `false`
    /// or eframe fails to detect the system theme.
    ///
    /// Default: [`Theme::Dark`].
    pub default_theme: Theme,

    /// This controls what happens when you close the main eframe window.
    ///
    /// If `true`, execution will continue after the eframe window is closed.
    /// If `false`, the app will close once the eframe window is closed.
    ///
    /// This is `true` by default, and the `false` option is only there
    /// so we can revert if we find any bugs.
    ///
    /// This feature was introduced in <https://github.com/emilk/egui/pull/1889>.
    ///
    /// When `true`, [`winit::platform::run_on_demand::EventLoopExtRunOnDemand`] is used.
    /// When `false`, [`winit::event_loop::EventLoop::run`] is used.
    pub run_and_return: bool,

    /// Hook into the building of an event loop before it is run.
    ///
    /// Specify a callback here in case you need to make platform specific changes to the
    /// event loop before it is run.
    ///
    /// Note: A [`NativeOptions`] clone will not include any `event_loop_builder` hook.
    #[cfg(any(feature = "glow", feature = "wgpu"))]
    pub event_loop_builder: Option<EventLoopBuilderHook>,

    /// Hook into the building of a window.
    ///
    /// Specify a callback here in case you need to make platform specific changes to the
    /// window appearance.
    ///
    /// Note: A [`NativeOptions`] clone will not include any `window_builder` hook.
    #[cfg(any(feature = "glow", feature = "wgpu"))]
    pub window_builder: Option<WindowBuilderHook>,

    #[cfg(feature = "glow")]
    /// Needed for cross compiling for VirtualBox VMSVGA driver with OpenGL ES 2.0 and OpenGL 2.1 which doesn't support SRGB texture.
    /// See <https://github.com/emilk/egui/pull/1993>.
    ///
    /// For OpenGL ES 2.0: set this to [`egui_glow::ShaderVersion::Es100`] to solve blank texture problem (by using the "fallback shader").
    pub shader_version: Option<egui_glow::ShaderVersion>,

    /// On desktop: make the window position to be centered at initialization.
    ///
    /// Platform specific:
    ///
    /// Wayland desktop currently not supported.
    pub centered: bool,

    /// Configures wgpu instance/device/adapter/surface creation and renderloop.
    #[cfg(feature = "wgpu")]
    pub wgpu_options: egui_wgpu::WgpuConfiguration,

    /// Controls whether or not the native window position and size will be
    /// persisted (only if the "persistence" feature is enabled).
    pub persist_window: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl Clone for NativeOptions {
    fn clone(&self) -> Self {
        Self {
            viewport: self.viewport.clone(),

            #[cfg(any(feature = "glow", feature = "wgpu"))]
            event_loop_builder: None, // Skip any builder callbacks if cloning

            #[cfg(any(feature = "glow", feature = "wgpu"))]
            window_builder: None, // Skip any builder callbacks if cloning

            #[cfg(feature = "wgpu")]
            wgpu_options: self.wgpu_options.clone(),

            ..*self
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for NativeOptions {
    fn default() -> Self {
        Self {
            viewport: Default::default(),

            vsync: true,
            multisampling: 0,
            depth_buffer: 0,
            stencil_buffer: 0,
            hardware_acceleration: HardwareAcceleration::Preferred,

            #[cfg(any(feature = "glow", feature = "wgpu"))]
            renderer: Renderer::default(),

            follow_system_theme: cfg!(target_os = "macos") || cfg!(target_os = "windows"),
            default_theme: Theme::Dark,
            run_and_return: true,

            #[cfg(any(feature = "glow", feature = "wgpu"))]
            event_loop_builder: None,

            #[cfg(any(feature = "glow", feature = "wgpu"))]
            window_builder: None,

            #[cfg(feature = "glow")]
            shader_version: None,

            centered: false,

            #[cfg(feature = "wgpu")]
            wgpu_options: egui_wgpu::WgpuConfiguration::default(),

            persist_window: true,
        }
    }
}

// ----------------------------------------------------------------------------

/// Options when using `eframe` in a web page.
#[cfg(target_arch = "wasm32")]
pub struct WebOptions {
    /// Try to detect and follow the system preferred setting for dark vs light mode.
    ///
    /// See also [`Self::default_theme`].
    ///
    /// Default: `true`.
    pub follow_system_theme: bool,

    /// Which theme to use in case [`Self::follow_system_theme`] is `false`
    /// or system theme detection fails.
    ///
    /// Default: `Theme::Dark`.
    pub default_theme: Theme,

    /// Sets the number of bits in the depth buffer.
    ///
    /// `egui` doesn't need the depth buffer, so the default value is 0.
    /// Unused by webgl context as of writing.
    pub depth_buffer: u8,

    /// Which version of WebGl context to select
    ///
    /// Default: [`WebGlContextOption::BestFirst`].
    #[cfg(feature = "glow")]
    pub webgl_context_option: WebGlContextOption,

    /// Configures wgpu instance/device/adapter/surface creation and renderloop.
    #[cfg(feature = "wgpu")]
    pub wgpu_options: egui_wgpu::WgpuConfiguration,

    /// The size limit of the web app canvas.
    ///
    /// By default the max size is [`egui::Vec2::INFINITY`], i.e. unlimited.
    pub max_size_points: egui::Vec2,
}

#[cfg(target_arch = "wasm32")]
impl Default for WebOptions {
    fn default() -> Self {
        Self {
            follow_system_theme: true,
            default_theme: Theme::Dark,
            depth_buffer: 0,

            #[cfg(feature = "glow")]
            webgl_context_option: WebGlContextOption::BestFirst,

            #[cfg(feature = "wgpu")]
            wgpu_options: egui_wgpu::WgpuConfiguration::default(),

            max_size_points: egui::Vec2::INFINITY,
        }
    }
}

// ----------------------------------------------------------------------------

/// Dark or Light theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Theme {
    /// Dark mode: light text on a dark background.
    Dark,

    /// Light mode: dark text on a light background.
    Light,
}

impl Theme {
    /// Get the egui visuals corresponding to this theme.
    ///
    /// Use with [`egui::Context::set_visuals`].
    pub fn egui_visuals(self) -> egui::Visuals {
        match self {
            Self::Dark => egui::Visuals::dark(),
            Self::Light => egui::Visuals::light(),
        }
    }
}

// ----------------------------------------------------------------------------

/// WebGL Context options
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum WebGlContextOption {
    /// Force Use WebGL1.
    WebGl1,

    /// Force use WebGL2.
    WebGl2,

    /// Use WebGl2 first.
    BestFirst,

    /// Use WebGl1 first
    CompatibilityFirst,
}

// ----------------------------------------------------------------------------

/// What rendering backend to use.
///
/// You need to enable the "glow" and "wgpu" features to have a choice.
#[cfg(any(feature = "glow", feature = "wgpu"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Renderer {
    /// Use [`egui_glow`] renderer for [`glow`](https://github.com/grovesNL/glow).
    #[cfg(feature = "glow")]
    Glow,

    /// Use [`egui_wgpu`] renderer for [`wgpu`](https://github.com/gfx-rs/wgpu).
    #[cfg(feature = "wgpu")]
    Wgpu,
}

#[cfg(any(feature = "glow", feature = "wgpu"))]
impl Default for Renderer {
    fn default() -> Self {
        #[cfg(not(feature = "glow"))]
        #[cfg(not(feature = "wgpu"))]
        compile_error!("eframe: you must enable at least one of the rendering backend features: 'glow' or 'wgpu'");

        #[cfg(feature = "glow")]
        #[cfg(not(feature = "wgpu"))]
        return Self::Glow;

        #[cfg(not(feature = "glow"))]
        #[cfg(feature = "wgpu")]
        return Self::Wgpu;

        // By default, only the `glow` feature is enabled, so if the user added `wgpu` to the feature list
        // they probably wanted to use wgpu:
        #[cfg(feature = "glow")]
        #[cfg(feature = "wgpu")]
        return Self::Wgpu;
    }
}

#[cfg(any(feature = "glow", feature = "wgpu"))]
impl std::fmt::Display for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "glow")]
            Self::Glow => "glow".fmt(f),

            #[cfg(feature = "wgpu")]
            Self::Wgpu => "wgpu".fmt(f),
        }
    }
}

#[cfg(any(feature = "glow", feature = "wgpu"))]
impl std::str::FromStr for Renderer {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, String> {
        match name.to_lowercase().as_str() {
            #[cfg(feature = "glow")]
            "glow" => Ok(Self::Glow),

            #[cfg(feature = "wgpu")]
            "wgpu" => Ok(Self::Wgpu),

            _ => Err(format!("eframe renderer {name:?} is not available. Make sure that the corresponding eframe feature is enabled."))
        }
    }
}

// ----------------------------------------------------------------------------

/// Represents the surroundings of your app.
///
/// It provides methods to inspect the surroundings (are we on the web?),
/// access to persistent storage, and access to the rendering backend.
pub struct Frame {
    /// Information about the integration.
    pub(crate) info: IntegrationInfo,

    /// A place where you can store custom data in a way that persists when you restart the app.
    pub(crate) storage: Option<Box<dyn Storage>>,

    /// A reference to the underlying [`glow`] (OpenGL) context.
    #[cfg(feature = "glow")]
    pub(crate) gl: Option<std::sync::Arc<glow::Context>>,

    /// Can be used to manage GPU resources for custom rendering with WGPU using [`egui::PaintCallback`]s.
    #[cfg(feature = "wgpu")]
    pub(crate) wgpu_render_state: Option<egui_wgpu::RenderState>,

    /// Raw platform window handle
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_window_handle: Result<RawWindowHandle, HandleError>,

    /// Raw platform display handle for window
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_display_handle: Result<RawDisplayHandle, HandleError>,
}

// Implementing `Clone` would violate the guarantees of `HasWindowHandle` and `HasDisplayHandle`.
#[cfg(not(target_arch = "wasm32"))]
assert_not_impl_any!(Frame: Clone);

#[allow(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
impl HasWindowHandle for Frame {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // Safety: the lifetime is correct.
        unsafe { Ok(WindowHandle::borrow_raw(self.raw_window_handle.clone()?)) }
    }
}

#[allow(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
impl HasDisplayHandle for Frame {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // Safety: the lifetime is correct.
        unsafe { Ok(DisplayHandle::borrow_raw(self.raw_display_handle.clone()?)) }
    }
}

impl Frame {
    /// True if you are in a web environment.
    ///
    /// Equivalent to `cfg!(target_arch = "wasm32")`
    #[allow(clippy::unused_self)]
    pub fn is_web(&self) -> bool {
        cfg!(target_arch = "wasm32")
    }

    /// Information about the integration.
    pub fn info(&self) -> &IntegrationInfo {
        &self.info
    }

    /// A place where you can store custom data in a way that persists when you restart the app.
    pub fn storage(&self) -> Option<&dyn Storage> {
        self.storage.as_deref()
    }

    /// A place where you can store custom data in a way that persists when you restart the app.
    pub fn storage_mut(&mut self) -> Option<&mut (dyn Storage + 'static)> {
        self.storage.as_deref_mut()
    }

    /// A reference to the underlying [`glow`] (OpenGL) context.
    ///
    /// This can be used, for instance, to:
    /// * Render things to offscreen buffers.
    /// * Read the pixel buffer from the previous frame (`glow::Context::read_pixels`).
    /// * Render things behind the egui windows.
    ///
    /// Note that all egui painting is deferred to after the call to [`App::update`]
    /// ([`egui`] only collects [`egui::Shape`]s and then eframe paints them all in one go later on).
    ///
    /// To get a [`glow`] context you need to compile with the `glow` feature flag,
    /// and run eframe using [`Renderer::Glow`].
    #[cfg(feature = "glow")]
    pub fn gl(&self) -> Option<&std::sync::Arc<glow::Context>> {
        self.gl.as_ref()
    }

    /// The underlying WGPU render state.
    ///
    /// Only available when compiling with the `wgpu` feature and using [`Renderer::Wgpu`].
    ///
    /// Can be used to manage GPU resources for custom rendering with WGPU using [`egui::PaintCallback`]s.
    #[cfg(feature = "wgpu")]
    pub fn wgpu_render_state(&self) -> Option<&egui_wgpu::RenderState> {
        self.wgpu_render_state.as_ref()
    }
}

/// Information about the web environment (if applicable).
#[derive(Clone, Debug)]
#[cfg(target_arch = "wasm32")]
pub struct WebInfo {
    /// The browser user agent.
    pub user_agent: String,

    /// Information about the URL.
    pub location: Location,
}

/// Information about the URL.
///
/// Everything has been percent decoded (`%20` -> ` ` etc).
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug)]
pub struct Location {
    /// The full URL (`location.href`) without the hash, percent-decoded.
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
    /// Use [`Self::query_map`] to get the parsed version of it.
    pub query: String,

    /// The parsed "query" part of "www.example.com/index.html?query#fragment".
    ///
    /// "foo=hello&bar%20&foo=world" is parsed as `{"bar ": [""], "foo": ["hello", "world"]}`
    pub query_map: std::collections::BTreeMap<String, Vec<String>>,

    /// `location.origin`
    ///
    /// Example: `"http://www.example.com:80"`.
    pub origin: String,
}

/// Information about the integration passed to the use app each frame.
#[derive(Clone, Debug)]
pub struct IntegrationInfo {
    /// Information about the surrounding web environment.
    #[cfg(target_arch = "wasm32")]
    pub web_info: WebInfo,

    /// Does the OS use dark or light mode?
    ///
    /// `None` means "don't know".
    pub system_theme: Option<Theme>,

    /// Seconds of cpu usage (in seconds) on the previous frame.
    ///
    /// This includes [`App::update`] as well as rendering (except for vsync waiting).
    ///
    /// For a more detailed view of cpu usage, use the [`puffin`](https://crates.io/crates/puffin)
    /// profiler together with the `puffin` feature of `eframe`.
    ///
    /// `None` if this is the first frame.
    pub cpu_usage: Option<f32>,
}

// ----------------------------------------------------------------------------

/// A place where you can store custom data in a way that persists when you restart the app.
///
/// On the web this is backed by [local storage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage).
/// On desktop this is backed by the file system.
///
/// See [`CreationContext::storage`] and [`App::save`].
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
pub(crate) struct DummyStorage {}

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
    crate::profile_function!(key);
    storage
        .get_string(key)
        .and_then(|value| match ron::from_str(&value) {
            Ok(value) => Some(value),
            Err(err) => {
                // This happens on when we break the format, e.g. when updating egui.
                log::debug!("Failed to decode RON: {err}");
                None
            }
        })
}

/// Serialize the given value as [RON](https://github.com/ron-rs/ron) and store with the given key.
#[cfg(feature = "ron")]
pub fn set_value<T: serde::Serialize>(storage: &mut dyn Storage, key: &str, value: &T) {
    crate::profile_function!(key);
    match ron::ser::to_string(value) {
        Ok(string) => storage.set_string(key, string),
        Err(err) => log::error!("eframe failed to encode data using ron: {}", err),
    }
}

/// [`Storage`] key used for app
pub const APP_KEY: &str = "app";
