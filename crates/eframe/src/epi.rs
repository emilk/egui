//! Platform-agnostic interface for writing apps using [`egui`] (epi = egui programming interface).
//!
//! `epi` provides interfaces for window management and serialization.
//!
//! Start by looking at the [`App`] trait, and implement [`App::ui`].

#![warn(missing_docs)] // Let's keep `epi` well-documented.

#[cfg(target_arch = "wasm32")]
use std::any::Any;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
pub use crate::native::winit_integration::UserEvent;

#[cfg(not(target_arch = "wasm32"))]
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};
#[cfg(not(target_arch = "wasm32"))]
use static_assertions::assert_not_impl_any;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
pub use winit::{event_loop::EventLoopBuilder, window::WindowAttributes};

/// Hook into the building of an event loop before it is run
///
/// You can configure any platform specific details required on top of the default configuration
/// done by `EFrame`.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
pub type EventLoopBuilderHook = Box<dyn FnOnce(&mut EventLoopBuilder<UserEvent>)>;

/// Hook into the building of a the native window.
///
/// You can configure any platform specific details required on top of the default configuration
/// done by `eframe`.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
pub type WindowBuilderHook = Box<dyn FnOnce(egui::ViewportBuilder) -> egui::ViewportBuilder>;

type DynError = Box<dyn std::error::Error + Send + Sync>;

/// This is how your app is created.
///
/// You can use the [`CreationContext`] to setup egui, restore state, setup OpenGL things, etc.
pub type AppCreator<'app> =
    Box<dyn 'app + FnOnce(&CreationContext<'_>) -> Result<Box<dyn 'app + App>, DynError>>;

/// Data that is passed to [`AppCreator`] that can be used to setup and initialize your app.
pub struct CreationContext<'s> {
    /// The egui Context.
    ///
    /// You can use this to customize the look of egui, e.g to call [`egui::Context::set_fonts`],
    /// [`egui::Context::set_visuals_of`] etc.
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
    pub get_proc_address:
        Option<std::sync::Arc<dyn Fn(&std::ffi::CStr) -> *const std::ffi::c_void + Send + Sync>>,

    /// The underlying WGPU render state.
    ///
    /// Only available when compiling with the `wgpu` feature and using [`Renderer::Wgpu`].
    ///
    /// Can be used to manage GPU resources for custom rendering with WGPU using [`egui::PaintCallback`]s.
    #[cfg(feature = "wgpu_no_default_features")]
    pub wgpu_render_state: Option<egui_wgpu::RenderState>,

    /// The root [`winit::window::Window`].
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) window: Option<std::sync::Arc<winit::window::Window>>,

    /// Raw platform window handle
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_window_handle: Result<RawWindowHandle, HandleError>,

    /// Raw platform display handle for window
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_display_handle: Result<RawDisplayHandle, HandleError>,
}

#[expect(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
impl HasWindowHandle for CreationContext<'_> {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // Safety: the lifetime is correct.
        unsafe { Ok(WindowHandle::borrow_raw(self.raw_window_handle.clone()?)) }
    }
}

#[expect(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
impl HasDisplayHandle for CreationContext<'_> {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // Safety: the lifetime is correct.
        unsafe { Ok(DisplayHandle::borrow_raw(self.raw_display_handle.clone()?)) }
    }
}

impl CreationContext<'_> {
    /// Create a new empty [CreationContext] for testing [App]s in kittest.
    #[doc(hidden)]
    pub fn _new_kittest(egui_ctx: egui::Context) -> Self {
        Self {
            egui_ctx,
            integration_info: IntegrationInfo::mock(),
            storage: None,
            #[cfg(feature = "glow")]
            gl: None,
            #[cfg(feature = "glow")]
            get_proc_address: None,
            #[cfg(feature = "wgpu_no_default_features")]
            wgpu_render_state: None,
            #[cfg(not(target_arch = "wasm32"))]
            window: None,
            #[cfg(not(target_arch = "wasm32"))]
            raw_window_handle: Err(HandleError::NotSupported),
            #[cfg(not(target_arch = "wasm32"))]
            raw_display_handle: Err(HandleError::NotSupported),
        }
    }

    /// Access to the root [`winit::window::Window`].
    ///
    /// `None` for headless (tests etc).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn winit_window(&self) -> Option<&std::sync::Arc<winit::window::Window>> {
        self.window.as_ref()
    }
}

// ----------------------------------------------------------------------------

/// Implement this trait to write apps that can be compiled for both web/wasm and desktop/native using [`eframe`](https://github.com/emilk/egui/tree/main/crates/eframe).
pub trait App {
    /// Called once before each call to [`Self::ui`],
    /// and additionally also called when the UI is hidden, but [`egui::Context::request_repaint`] was called.
    ///
    /// You may NOT show any ui or do any painting during the call to [`Self::logic`].
    ///
    /// The [`egui::Context`] can be cloned and saved if you like.
    ///
    /// To force another call to [`Self::logic`], call [`egui::Context::request_repaint`] at any time (e.g. from another thread).
    fn logic(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        _ = (ctx, frame);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    ///
    /// The given [`egui::Ui`] has no margin or background color.
    /// You can wrap your UI code in [`egui::CentralPanel`] or a [`egui::Frame::central_panel`] to remedy this.
    ///
    /// The [`egui::Ui::ctx`] can be cloned and saved if you like.
    /// To force a repaint, call [`egui::Context::request_repaint`] at any time (e.g. from another thread).
    ///
    /// This is called for the root viewport ([`egui::ViewportId::ROOT`]).
    /// Use [`egui::Context::show_viewport_deferred`] to spawn additional viewports (windows).
    /// (A "viewport" in egui means an native OS window).
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame);

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
    /// The path can be customized via [`NativeOptions::persistence_path`].
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

    /// A hook for manipulating or filtering raw input before it is processed by [`Self::ui`].
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

    /// What rendering backend to use.
    #[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
    pub renderer: Renderer,

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
    #[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
    pub event_loop_builder: Option<EventLoopBuilderHook>,

    /// Hook into the building of a window.
    ///
    /// Specify a callback here in case you need to make platform specific changes to the
    /// window appearance.
    ///
    /// Note: A [`NativeOptions`] clone will not include any `window_builder` hook.
    #[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
    pub window_builder: Option<WindowBuilderHook>,

    /// On desktop: make the window position to be centered at initialization.
    ///
    /// Platform specific:
    ///
    /// Wayland desktop currently not supported.
    pub centered: bool,

    /// Configures glow instance.
    #[cfg(feature = "glow")]
    pub glow_options: egui_glow::GlowConfiguration,

    /// Configures wgpu instance/device/adapter/surface creation and renderloop.
    #[cfg(feature = "wgpu_no_default_features")]
    pub wgpu_options: egui_wgpu::WgpuConfiguration,

    /// Controls whether or not the native window position and size will be
    /// persisted (only if the "persistence" feature is enabled).
    pub persist_window: bool,

    /// The folder where `eframe` will store the app state. If not set, eframe will use a default
    /// data storage path for each target system.
    pub persistence_path: Option<std::path::PathBuf>,

    /// Controls whether to apply dithering to minimize banding artifacts.
    ///
    /// Dithering assumes an sRGB output and thus will apply noise to any input value that lies between
    /// two 8bit values after applying the sRGB OETF function, i.e. if it's not a whole 8bit value in "gamma space".
    /// This means that only inputs from texture interpolation and vertex colors should be affected in practice.
    ///
    /// Defaults to true.
    pub dithering: bool,

    /// Android application for `winit`'s event loop.
    ///
    /// This value is required on Android to correctly create the event loop. See
    /// [`EventLoopBuilder::build`] and [`with_android_app`] for details.
    ///
    /// [`EventLoopBuilder::build`]: winit::event_loop::EventLoopBuilder::build
    /// [`with_android_app`]: winit::platform::android::EventLoopBuilderExtAndroid::with_android_app
    #[cfg(target_os = "android")]
    pub android_app: Option<winit::platform::android::activity::AndroidApp>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Clone for NativeOptions {
    fn clone(&self) -> Self {
        Self {
            viewport: self.viewport.clone(),

            #[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
            event_loop_builder: None, // Skip any builder callbacks if cloning

            #[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
            window_builder: None, // Skip any builder callbacks if cloning

            #[cfg(feature = "glow")]
            glow_options: self.glow_options.clone(),

            #[cfg(feature = "wgpu_no_default_features")]
            wgpu_options: self.wgpu_options.clone(),

            persistence_path: self.persistence_path.clone(),

            #[cfg(target_os = "android")]
            android_app: self.android_app.clone(),

            ..*self
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for NativeOptions {
    fn default() -> Self {
        Self {
            viewport: Default::default(),

            multisampling: 0,
            depth_buffer: 0,
            stencil_buffer: 0,

            #[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
            renderer: Renderer::default(),

            run_and_return: true,

            #[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
            event_loop_builder: None,

            #[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
            window_builder: None,

            centered: false,

            #[cfg(feature = "glow")]
            glow_options: egui_glow::GlowConfiguration::default(),

            #[cfg(feature = "wgpu_no_default_features")]
            wgpu_options: egui_wgpu::WgpuConfiguration::default()
                .with_surface_config(egui_wgpu::SurfaceConfig::LOW_LATENCY),

            persist_window: true,

            persistence_path: None,

            dithering: true,

            #[cfg(target_os = "android")]
            android_app: None,
        }
    }
}

// ----------------------------------------------------------------------------

/// Options when using `eframe` in a web page.
#[cfg(target_arch = "wasm32")]
pub struct WebOptions {
    /// What rendering backend to use.
    #[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
    pub renderer: Renderer,

    /// Sets the number of bits in the depth buffer.
    ///
    /// `egui` doesn't need the depth buffer, so the default value is 0.
    /// Unused by webgl context as of writing.
    pub depth_buffer: u8,

    /// Which version of WebGL context to select
    ///
    /// Default: [`WebGlContextOption::BestFirst`].
    #[cfg(feature = "glow")]
    pub webgl_context_option: WebGlContextOption,

    /// Configures glow instance.
    #[cfg(feature = "glow")]
    pub glow_options: egui_glow::GlowConfiguration,

    /// Configures wgpu instance/device/adapter/surface creation and renderloop.
    #[cfg(feature = "wgpu_no_default_features")]
    pub wgpu_options: egui_wgpu::WgpuConfiguration,

    /// Controls whether to apply dithering to minimize banding artifacts.
    ///
    /// Dithering assumes an sRGB output and thus will apply noise to any input value that lies between
    /// two 8bit values after applying the sRGB OETF function, i.e. if it's not a whole 8bit value in "gamma space".
    /// This means that only inputs from texture interpolation and vertex colors should be affected in practice.
    ///
    /// Defaults to true.
    pub dithering: bool,

    /// If the web event corresponding to an egui event should be propagated
    /// to the rest of the web page.
    ///
    /// The default is `true`, meaning
    /// [`stopPropagation`](https://developer.mozilla.org/en-US/docs/Web/API/Event/stopPropagation)
    /// is called on every event, and the event is not propagated to the rest of the web page.
    pub should_stop_propagation: Box<dyn Fn(&egui::Event) -> bool>,

    /// Whether the web event corresponding to an egui event should have `prevent_default` called
    /// on it or not.
    ///
    /// Defaults to true.
    pub should_prevent_default: Box<dyn Fn(&egui::Event) -> bool>,

    /// Maximum rate at which to repaint. This can be used to artificially reduce the repaint rate below
    /// vsync in order to save resources.
    pub max_fps: Option<u32>,
}

#[cfg(target_arch = "wasm32")]
impl Default for WebOptions {
    fn default() -> Self {
        Self {
            #[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
            renderer: Renderer::default(),

            depth_buffer: 0,

            #[cfg(feature = "glow")]
            webgl_context_option: WebGlContextOption::BestFirst,

            #[cfg(feature = "glow")]
            glow_options: egui_glow::GlowConfiguration::default(),

            #[cfg(feature = "wgpu_no_default_features")]
            wgpu_options: egui_wgpu::WgpuConfiguration::default(),

            dithering: true,

            should_stop_propagation: Box::new(|_| true),
            should_prevent_default: Box::new(|_| true),

            max_fps: None,
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

    /// Use WebGL2 first.
    BestFirst,

    /// Use WebGL1 first
    CompatibilityFirst,
}

// ----------------------------------------------------------------------------

/// What rendering backend to use.
///
/// You need to enable the "glow" and "wgpu" features to have a choice.
#[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Renderer {
    /// Use [`egui_glow`] renderer for [`glow`](https://github.com/grovesNL/glow).
    #[cfg(feature = "glow")]
    Glow,

    /// Use [`egui_wgpu`] renderer for [`wgpu`](https://github.com/gfx-rs/wgpu).
    #[cfg(feature = "wgpu_no_default_features")]
    Wgpu,
}

#[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
impl Default for Renderer {
    fn default() -> Self {
        #[cfg(not(feature = "glow"))]
        #[cfg(not(feature = "wgpu_no_default_features"))]
        compile_error!(
            "eframe: you must enable at least one of the rendering backend features: 'glow' or 'wgpu'"
        );

        #[cfg(feature = "glow")]
        #[cfg(not(feature = "wgpu_no_default_features"))]
        return Self::Glow;

        #[cfg(not(feature = "glow"))]
        #[cfg(feature = "wgpu_no_default_features")]
        return Self::Wgpu;

        // It's weird that the user has enabled both glow and wgpu,
        // but let's pick the better of the two (wgpu):
        #[cfg(feature = "glow")]
        #[cfg(feature = "wgpu_no_default_features")]
        return Self::Wgpu;
    }
}

#[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
impl std::fmt::Display for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "glow")]
            Self::Glow => "glow".fmt(f),

            #[cfg(feature = "wgpu_no_default_features")]
            Self::Wgpu => "wgpu".fmt(f),
        }
    }
}

#[cfg(any(feature = "glow", feature = "wgpu_no_default_features"))]
impl std::str::FromStr for Renderer {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, String> {
        match name.to_lowercase().as_str() {
            #[cfg(feature = "glow")]
            "glow" => Ok(Self::Glow),

            #[cfg(feature = "wgpu_no_default_features")]
            "wgpu" => Ok(Self::Wgpu),

            _ => Err(format!(
                "eframe renderer {name:?} is not available. Make sure that the corresponding eframe feature is enabled."
            )),
        }
    }
}

// ----------------------------------------------------------------------------

/// Controls how eframe intercepts numpad keys before egui-winit processes them.
///
/// Set via [`Frame::set_numpad_capture_mode`]. Captured events never reach egui
/// (no text insertion, no navigation) and are exposed through [`Frame::numpad_keys`]
/// so the application can run its own keybinds.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NumpadCaptureMode {
    /// Don't intercept anything; numpad keys behave as in stock eframe/egui.
    Off,

    /// Capture digits and decimal when `NumLock` is OFF (they'd otherwise act as
    /// arrow/Home/End navigation); pass them through for text input when `NumLock` is ON.
    /// Operators (`+ - * /`) and `NumpadEnter` are always captured, since their produced
    /// character/action doesn't depend on `NumLock`.
    ///
    /// This is the default, and matches Wrayth-style behavior on Windows/Linux.
    /// Note: macOS has no `NumLock`, so digits are never captured there — use
    /// [`Self::Always`] on macOS if you want numpad keybinds.
    #[default]
    NumLockAware,

    /// Always capture all 16 numpad keys for keybinds, regardless of `NumLock` state.
    /// With this mode the numpad can never be used for text input, so it is best
    /// offered as a user setting. This is the only way to get numpad digit keybinds
    /// on macOS (which has no `NumLock`).
    Always,
}

/// A numpad key event captured before egui-winit processes it.
///
/// This preserves the distinction between numpad keys and regular keys,
/// which egui-winit normally merges together. Use this to implement
/// numpad-specific keybinds in your application.
///
/// The `consumed` field says whether eframe swallowed the event (egui never saw it).
/// Only consumed events should trigger keybinds — see [`Self::keybind_name`].
/// Events with `consumed == false` were also processed normally by egui (e.g. the
/// digit was inserted into a focused `TextEdit`), so do NOT insert text for them.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct NumpadKeyEvent {
    /// The physical key that was pressed (e.g., `Numpad1`, `NumpadAdd`).
    pub physical_key: winit::keyboard::PhysicalKey,

    /// True if eframe consumed this event, meaning egui never processed it.
    /// Consumed events are the ones that should trigger application keybinds.
    /// Non-consumed events were passed through to egui for normal handling
    /// (text input, widget interaction) and are exposed here for information only.
    pub consumed: bool,

    /// Whether `NumLock` was active, when it can be inferred from the event.
    /// Only meaningful for digits and decimal (`Some(true)`/`Some(false)`);
    /// `None` for operators and `NumpadEnter`, whose logical key doesn't depend
    /// on `NumLock`. Always `Some(true)` on macOS, which has no `NumLock`.
    pub numlock_on: Option<bool>,

    /// Whether the key was pressed or released.
    pub pressed: bool,

    /// True if this event is an OS auto-repeat of a held key.
    /// Check this if a keybind should fire once per physical keystroke.
    pub repeat: bool,

    /// Active modifier keys (Ctrl, Shift, Alt, etc.)
    pub modifiers: egui::Modifiers,

    /// The character this key produces according to the active keyboard layout
    /// (e.g. `'5'`, `'+'`, or `','` for the decimal key on some locales).
    /// `None` for `NumpadEnter` and for digit/decimal keys while `NumLock` is OFF.
    pub character: Option<char>,
}

/// The keybind name for a numpad key code (e.g. "`num_1`", "`num_plus`"),
/// or `None` if it isn't one of the 16 numpad keys we support.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn numpad_keybind_name(code: winit::keyboard::KeyCode) -> Option<&'static str> {
    use winit::keyboard::KeyCode;
    match code {
        KeyCode::Numpad0 => Some("num_0"),
        KeyCode::Numpad1 => Some("num_1"),
        KeyCode::Numpad2 => Some("num_2"),
        KeyCode::Numpad3 => Some("num_3"),
        KeyCode::Numpad4 => Some("num_4"),
        KeyCode::Numpad5 => Some("num_5"),
        KeyCode::Numpad6 => Some("num_6"),
        KeyCode::Numpad7 => Some("num_7"),
        KeyCode::Numpad8 => Some("num_8"),
        KeyCode::Numpad9 => Some("num_9"),
        KeyCode::NumpadAdd => Some("num_plus"),
        KeyCode::NumpadSubtract => Some("num_minus"),
        KeyCode::NumpadMultiply => Some("num_multiply"),
        KeyCode::NumpadDivide => Some("num_divide"),
        KeyCode::NumpadEnter => Some("num_enter"),
        KeyCode::NumpadDecimal => Some("num_decimal"),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl NumpadKeyEvent {
    /// Returns the keybind name for this numpad key (e.g., "`num_1`", "`num_plus`").
    ///
    /// Returns `None` if the event was not consumed by eframe — in that case egui
    /// already handled the key normally (e.g. as text input), so no keybind should fire.
    pub fn keybind_name(&self) -> Option<&'static str> {
        if !self.consumed {
            return None;
        }

        match self.physical_key {
            winit::keyboard::PhysicalKey::Code(code) => numpad_keybind_name(code),
            winit::keyboard::PhysicalKey::Unidentified(_) => None,
        }
    }

    /// The character this key produces according to the active keyboard layout.
    ///
    /// If `consumed` is false, egui already inserted this character into any focused
    /// text widget — do not insert it again. If `consumed` is true (e.g. in
    /// [`NumpadCaptureMode::Always`]), the application may use this to forward the
    /// character itself, e.g. when no keybind is configured for the key.
    pub fn to_char(&self) -> Option<char> {
        self.character
    }
}

/// The kind of numpad key, used to decide capture policy.
#[cfg(not(target_arch = "wasm32"))]
enum NumpadKeyKind {
    /// `Numpad0`–`Numpad9`: produce digits with `NumLock` ON, navigation with `NumLock` OFF.
    Digit,
    /// `NumpadDecimal`: produces `.`/`,` with `NumLock` ON, Delete with `NumLock` OFF.
    Decimal,
    /// `+ - * /`: always produce the same character, unaffected by `NumLock`.
    Operator,
    /// `NumpadEnter`: always acts as Enter, unaffected by `NumLock`.
    Enter,
}

/// Decides whether a winit keyboard event is a numpad key we intercept,
/// and builds the [`NumpadKeyEvent`] for it.
///
/// Returns `None` if the event should be handled entirely by the normal
/// egui-winit path. If `Some`, the event must be pushed to [`Frame::numpad_keys`];
/// if additionally `.consumed` is true, the event must NOT be forwarded to egui-winit.
#[cfg(not(target_arch = "wasm32"))]
#[expect(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
pub(crate) fn intercept_numpad_key(
    physical_key: winit::keyboard::PhysicalKey,
    logical_key: &winit::keyboard::Key,
    location: winit::keyboard::KeyLocation,
    pressed: bool,
    repeat: bool,
    is_synthetic: bool,
    modifiers: egui::Modifiers,
    mode: NumpadCaptureMode,
    capture_keys: Option<&std::collections::HashSet<String>>,
) -> Option<NumpadKeyEvent> {
    use winit::keyboard::{Key, KeyCode, KeyLocation, PhysicalKey};

    if mode == NumpadCaptureMode::Off || location != KeyLocation::Numpad {
        return None;
    }

    // Mirror egui-winit: ignore synthetic key presses (e.g. sent by Windows for keys
    // already held when the window gains focus), so they can't fire spurious keybinds.
    if is_synthetic && pressed {
        return None;
    }

    let PhysicalKey::Code(code) = physical_key else {
        return None;
    };

    let kind = match code {
        KeyCode::Numpad0
        | KeyCode::Numpad1
        | KeyCode::Numpad2
        | KeyCode::Numpad3
        | KeyCode::Numpad4
        | KeyCode::Numpad5
        | KeyCode::Numpad6
        | KeyCode::Numpad7
        | KeyCode::Numpad8
        | KeyCode::Numpad9 => NumpadKeyKind::Digit,
        KeyCode::NumpadDecimal => NumpadKeyKind::Decimal,
        KeyCode::NumpadAdd
        | KeyCode::NumpadSubtract
        | KeyCode::NumpadMultiply
        | KeyCode::NumpadDivide => NumpadKeyKind::Operator,
        KeyCode::NumpadEnter => NumpadKeyKind::Enter,
        _ => return None,
    };

    // Digits/decimal produce a `Character` logical key when NumLock is ON and a
    // `Named` key (End, Down, Delete, …) when it is OFF. Operators and Enter don't
    // change with NumLock, so their state can't be inferred from the event.
    let numlock_on = match kind {
        NumpadKeyKind::Digit | NumpadKeyKind::Decimal => {
            Some(matches!(logical_key, Key::Character(_)))
        }
        NumpadKeyKind::Operator | NumpadKeyKind::Enter => None,
    };

    let consumed = match mode {
        NumpadCaptureMode::Off => unreachable!("handled above"),
        NumpadCaptureMode::Always => true,
        NumpadCaptureMode::NumLockAware => match kind {
            // With NumLock OFF these would double as navigation keys — capture them.
            // With NumLock ON they type digits — let egui handle them.
            NumpadKeyKind::Digit | NumpadKeyKind::Decimal => numlock_on == Some(false),
            NumpadKeyKind::Operator | NumpadKeyKind::Enter => true,
        },
    };

    // If the app registered the set of keys it actually has bindings for, only
    // consume those; unbound keys keep their native behavior (typing, Enter, …).
    let consumed = consumed
        && capture_keys.is_none_or(|keys| {
            numpad_keybind_name(code).is_some_and(|name| keys.contains(name))
        });

    let character = match logical_key {
        Key::Character(s) => s.chars().next(),
        _ => None,
    };

    Some(NumpadKeyEvent {
        physical_key,
        consumed,
        numlock_on,
        pressed,
        repeat,
        modifiers,
        character,
    })
}

/// A key event captured because its [`winit::keyboard::KeyCode`] is in the set given to
/// [`Frame::set_key_capture`].
///
/// Use this for keys egui has no [`egui::Key`] for (Pause, Scroll Lock, Print Screen,
/// Caps Lock, the context-menu key, …), which egui-winit would otherwise drop.
/// Captured events never reach egui.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct CapturedKeyEvent {
    /// The physical key that was pressed (e.g. `Pause`, `ScrollLock`).
    pub physical_key: winit::keyboard::PhysicalKey,

    /// Whether the key was pressed or released.
    pub pressed: bool,

    /// True if this event is an OS auto-repeat of a held key.
    /// Check this if a keybind should fire once per physical keystroke.
    pub repeat: bool,

    /// Active modifier keys (Ctrl, Shift, Alt, etc.)
    pub modifiers: egui::Modifiers,
}

/// Decides whether a winit keyboard event is in the app's key-capture set,
/// and builds the [`CapturedKeyEvent`] for it.
///
/// Returns `None` if the event should be handled by the normal path. If `Some`,
/// the event must be pushed to [`Frame::captured_keys`] and must NOT be forwarded
/// to egui-winit.
#[cfg(not(target_arch = "wasm32"))]
#[expect(clippy::fn_params_excessive_bools)]
pub(crate) fn intercept_captured_key(
    physical_key: winit::keyboard::PhysicalKey,
    pressed: bool,
    repeat: bool,
    is_synthetic: bool,
    modifiers: egui::Modifiers,
    capture: &std::collections::HashSet<winit::keyboard::KeyCode>,
) -> Option<CapturedKeyEvent> {
    let winit::keyboard::PhysicalKey::Code(code) = physical_key else {
        return None;
    };

    if !capture.contains(&code) {
        return None;
    }

    // Mirror egui-winit: ignore synthetic key presses (e.g. sent by Windows for keys
    // already held when the window gains focus), so they can't fire spurious keybinds.
    if is_synthetic && pressed {
        return None;
    }

    Some(CapturedKeyEvent {
        physical_key,
        pressed,
        repeat,
        modifiers,
    })
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

    /// Used to convert user custom [`glow::Texture`] to [`egui::TextureId`]
    #[cfg(all(feature = "glow", not(target_arch = "wasm32")))]
    pub(crate) glow_register_native_texture:
        Option<Box<dyn FnMut(glow::Texture) -> egui::TextureId>>,

    /// Can be used to manage GPU resources for custom rendering with WGPU using [`egui::PaintCallback`]s.
    #[cfg(feature = "wgpu_no_default_features")]
    #[doc(hidden)]
    pub wgpu_render_state: Option<egui_wgpu::RenderState>,

    /// The current [`winit::window::Window`] (i.e. the one the active viewport is rendered to).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) window: Option<std::sync::Arc<winit::window::Window>>,

    /// Raw platform window handle
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_window_handle: Result<RawWindowHandle, HandleError>,

    /// Raw platform display handle for window
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_display_handle: Result<RawDisplayHandle, HandleError>,

    /// Numpad key events captured this frame, before egui-winit processes them.
    /// This preserves the distinction between numpad keys and regular number keys.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) numpad_keys: Vec<NumpadKeyEvent>,

    /// How numpad keys are intercepted; see [`NumpadCaptureMode`].
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) numpad_capture_mode: NumpadCaptureMode,

    /// If `Some`, only numpad keys whose keybind name is in this set are consumed;
    /// the rest keep their native behavior. See [`Frame::set_numpad_capture_keys`].
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) numpad_capture_keys: Option<std::collections::HashSet<String>>,

    /// Key events captured this frame because their key code is in `key_capture`.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) captured_keys: Vec<CapturedKeyEvent>,

    /// Key codes intercepted before egui-winit. See [`Frame::set_key_capture`].
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) key_capture: std::collections::HashSet<winit::keyboard::KeyCode>,
}

// Implementing `Clone` would violate the guarantees of `HasWindowHandle` and `HasDisplayHandle`.
#[cfg(not(target_arch = "wasm32"))]
assert_not_impl_any!(Frame: Clone);

#[expect(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
impl HasWindowHandle for Frame {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // Safety: the lifetime is correct.
        unsafe { Ok(WindowHandle::borrow_raw(self.raw_window_handle.clone()?)) }
    }
}

#[expect(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
impl HasDisplayHandle for Frame {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // Safety: the lifetime is correct.
        unsafe { Ok(DisplayHandle::borrow_raw(self.raw_display_handle.clone()?)) }
    }
}

impl Frame {
    /// Create a new empty [Frame] for testing [App]s in kittest.
    #[doc(hidden)]
    pub fn _new_kittest() -> Self {
        Self {
            #[cfg(feature = "glow")]
            gl: None,
            #[cfg(all(feature = "glow", not(target_arch = "wasm32")))]
            glow_register_native_texture: None,
            info: IntegrationInfo::mock(),
            #[cfg(not(target_arch = "wasm32"))]
            raw_display_handle: Err(HandleError::NotSupported),
            #[cfg(not(target_arch = "wasm32"))]
            raw_window_handle: Err(HandleError::NotSupported),
            #[cfg(not(target_arch = "wasm32"))]
            window: None,
            storage: None,
            #[cfg(feature = "wgpu_no_default_features")]
            wgpu_render_state: None,
            #[cfg(not(target_arch = "wasm32"))]
            numpad_keys: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            numpad_capture_mode: NumpadCaptureMode::default(),
            #[cfg(not(target_arch = "wasm32"))]
            numpad_capture_keys: None,
            #[cfg(not(target_arch = "wasm32"))]
            captured_keys: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            key_capture: Default::default(),
        }
    }

    /// Returns numpad key events captured this frame.
    ///
    /// These events are captured before egui-winit processes them, preserving
    /// the distinction between numpad keys and regular number keys.
    ///
    /// Only events with `consumed == true` were swallowed by eframe; for those,
    /// [`NumpadKeyEvent::keybind_name`] returns the keybind to run (e.g. "`num_1`").
    /// Events with `consumed == false` were also handled normally by egui (e.g.
    /// the digit was inserted into a focused `TextEdit`), so do not insert text
    /// for them yourself.
    ///
    /// # Example
    /// ```ignore
    /// fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
    ///     for key in frame.numpad_keys() {
    ///         if !key.pressed || key.repeat {
    ///             continue;
    ///         }
    ///         // `keybind_name()` is `Some` only for events eframe consumed
    ///         // (egui never saw them), so this can't double-act with text input:
    ///         if let Some(name) = key.keybind_name() {
    ///             if let Some(cmd) = self.keybinds.get(name) {
    ///                 self.execute(cmd);
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn numpad_keys(&self) -> &[NumpadKeyEvent] {
        &self.numpad_keys
    }

    /// How numpad keys are currently intercepted; see [`NumpadCaptureMode`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn numpad_capture_mode(&self) -> NumpadCaptureMode {
        self.numpad_capture_mode
    }

    /// Sets how numpad keys are intercepted; see [`NumpadCaptureMode`].
    ///
    /// Can be changed at any time (e.g. from a user setting). On macOS you must
    /// use [`NumpadCaptureMode::Always`] to get numpad keybinds, since macOS has
    /// no `NumLock` and numpad digits always arrive as regular digit input otherwise.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_numpad_capture_mode(&mut self, mode: NumpadCaptureMode) {
        self.numpad_capture_mode = mode;
    }

    /// Restricts capture to the numpad keys the application actually has bindings for.
    ///
    /// Pass `Some` with a set of keybind names ("`num_0`" … "`num_9`", "`num_plus`",
    /// "`num_minus`", "`num_multiply`", "`num_divide`", "`num_enter`", "`num_decimal")`:
    /// only those keys are consumed for keybinds; all other numpad keys keep their
    /// native behavior (typing digits, Enter submitting text, navigation, …).
    /// Pass `None` (the default) to capture every key the mode selects.
    ///
    /// Update this whenever the user adds or removes a binding. Combined with
    /// [`NumpadCaptureMode::Always`] this gives per-key fallback on macOS:
    /// bound keys run macros, unbound keys still type into whatever is focused.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_numpad_capture_keys(
        &mut self,
        keys: Option<std::collections::HashSet<String>>,
    ) {
        self.numpad_capture_keys = keys;
    }

    /// Clears the numpad key events. Called internally after each frame.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn clear_numpad_keys(&mut self) {
        self.numpad_keys.clear();
    }

    /// Returns key events captured this frame because their key code is in the
    /// set given to [`Self::set_key_capture`]. egui never saw these events.
    ///
    /// # Example
    /// ```ignore
    /// for key in frame.captured_keys() {
    ///     if key.pressed && !key.repeat
    ///         && key.physical_key == PhysicalKey::Code(KeyCode::Pause)
    ///     {
    ///         self.execute_pause_bind();
    ///     }
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn captured_keys(&self) -> &[CapturedKeyEvent] {
        &self.captured_keys
    }

    /// The key codes currently intercepted; see [`Self::set_key_capture`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn key_capture(&self) -> &std::collections::HashSet<winit::keyboard::KeyCode> {
        &self.key_capture
    }

    /// Intercepts every key whose [`winit::keyboard::KeyCode`] is in `keys` before
    /// egui-winit sees it, and reports it through [`Self::captured_keys`].
    ///
    /// Meant for keys egui has no [`egui::Key`] for (e.g. `Pause`, `ScrollLock`,
    /// `PrintScreen`, `CapsLock`, `ContextMenu`, or macOS's Clear key, which winit
    /// reports as `NumLock`). Captured keys never reach egui, so avoid putting keys
    /// here that egui should still handle. Keys in this set take precedence over
    /// numpad interception (see [`Self::set_numpad_capture_mode`]).
    ///
    /// Update this whenever the user adds or removes a binding. Empty by default.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_key_capture(&mut self, keys: std::collections::HashSet<winit::keyboard::KeyCode>) {
        self.key_capture = keys;
    }

    /// Clears the captured key events. Called internally after each frame.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn clear_captured_keys(&mut self) {
        self.captured_keys.clear();
    }

    /// True if you are in a web environment.
    ///
    /// Equivalent to `cfg!(target_arch = "wasm32")`
    #[expect(clippy::unused_self)]
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

    /// Access to the current [`winit::window::Window`] (i.e. the one the active viewport is rendered to).
    ///
    /// `None` for headless (tests etc).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn winit_window(&self) -> Option<&std::sync::Arc<winit::window::Window>> {
        self.window.as_ref()
    }

    /// A reference to the underlying [`glow`] (OpenGL) context.
    ///
    /// This can be used, for instance, to:
    /// * Render things to offscreen buffers.
    /// * Read the pixel buffer from the previous frame (`glow::Context::read_pixels`).
    /// * Render things behind the egui windows.
    ///
    /// Note that all egui painting is deferred to after the call to [`App::ui`]
    /// ([`egui`] only collects [`egui::Shape`]s and then eframe paints them all in one go later on).
    ///
    /// To get a [`glow`] context you need to compile with the `glow` feature flag,
    /// and run eframe using [`Renderer::Glow`].
    #[cfg(feature = "glow")]
    pub fn gl(&self) -> Option<&std::sync::Arc<glow::Context>> {
        self.gl.as_ref()
    }

    /// Register your own [`glow::Texture`],
    /// and then you can use the returned [`egui::TextureId`] to render your texture with [`egui`].
    ///
    /// This function will take the ownership of your [`glow::Texture`], so please do not delete your [`glow::Texture`] after registering.
    #[cfg(all(feature = "glow", not(target_arch = "wasm32")))]
    pub fn register_native_glow_texture(&mut self, native: glow::Texture) -> egui::TextureId {
        #[expect(clippy::unwrap_used)]
        self.glow_register_native_texture.as_mut().unwrap()(native)
    }

    /// The underlying WGPU render state.
    ///
    /// Only available when compiling with the `wgpu` feature and using [`Renderer::Wgpu`].
    ///
    /// Can be used to manage GPU resources for custom rendering with WGPU using [`egui::PaintCallback`]s.
    #[cfg(feature = "wgpu_no_default_features")]
    pub fn wgpu_render_state(&self) -> Option<&egui_wgpu::RenderState> {
        self.wgpu_render_state.as_ref()
    }

    /// The currently-applied runtime surface config (present mode, frame latency)
    /// used by the `wgpu` renderer, if any.
    ///
    /// Returns `None` when not using the `wgpu` backend.
    #[cfg(feature = "wgpu_no_default_features")]
    pub fn wgpu_surface_config(&self) -> Option<egui_wgpu::SurfaceConfig> {
        self.wgpu_render_state
            .as_ref()
            .map(|state| state.surface_config)
    }

    /// Set the runtime surface config (present mode, frame latency) for the `wgpu`
    /// renderer. The surface is reconfigured on the next paint.
    ///
    /// No-op when not using the `wgpu` backend.
    #[cfg(feature = "wgpu_no_default_features")]
    pub fn set_wgpu_surface_config(&mut self, config: egui_wgpu::SurfaceConfig) {
        if let Some(state) = &mut self.wgpu_render_state {
            state.surface_config = config;
        }
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

    /// Seconds of cpu usage (in seconds) on the previous frame.
    ///
    /// This includes [`App::ui`] as well as rendering (except for vsync waiting).
    ///
    /// For a more detailed view of cpu usage, connect your preferred profiler by enabling it's feature in [`profiling`](https://crates.io/crates/profiling).
    ///
    /// `None` if this is the first frame.
    pub cpu_usage: Option<f32>,
}

impl IntegrationInfo {
    fn mock() -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            web_info: WebInfo {
                user_agent: "kittest".to_owned(),
                location: Location {
                    url: "http://localhost".to_owned(),
                    protocol: "http:".to_owned(),
                    host: "localhost".to_owned(),
                    hostname: "localhost".to_owned(),
                    port: "80".to_owned(),
                    hash: String::new(),
                    query: String::new(),
                    query_map: Default::default(),
                    origin: "http://localhost".to_owned(),
                },
            },
            cpu_usage: None,
        }
    }
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

    /// Remove a given key.
    fn remove_string(&mut self, key: &str);

    /// write-to-disk or similar
    fn flush(&mut self);
}

/// Get and deserialize the [RON](https://github.com/ron-rs/ron) stored at the given key.
#[cfg(feature = "ron")]
pub fn get_value<T: serde::de::DeserializeOwned>(storage: &dyn Storage, key: &str) -> Option<T> {
    profiling::function_scope!(key);
    let value = storage.get_string(key)?;
    match ron::from_str(&value) {
        Ok(value) => Some(value),
        Err(err) => {
            // This happens on when we break the format, e.g. when updating egui.
            log::debug!("Failed to decode RON: {err}");
            None
        }
    }
}

/// Serialize the given value as [RON](https://github.com/ron-rs/ron) and store with the given key.
#[cfg(feature = "ron")]
pub fn set_value<T: serde::Serialize>(storage: &mut dyn Storage, key: &str, value: &T) {
    profiling::function_scope!(key);
    match ron::ser::to_string(value) {
        Ok(string) => storage.set_string(key, string),
        Err(err) => log::error!("eframe failed to encode data using ron: {err}"),
    }
}

/// [`Storage`] key used for app
pub const APP_KEY: &str = "app";

#[cfg(all(test, not(target_arch = "wasm32")))]
mod numpad_tests {
    use super::{NumpadCaptureMode, NumpadKeyEvent, intercept_numpad_key};
    use winit::keyboard::{Key, KeyCode, KeyLocation, NamedKey, PhysicalKey, SmolStr};

    fn intercept(
        code: KeyCode,
        logical_key: Key,
        mode: NumpadCaptureMode,
        capture_keys: Option<&std::collections::HashSet<String>>,
    ) -> Option<NumpadKeyEvent> {
        intercept_numpad_key(
            PhysicalKey::Code(code),
            &logical_key,
            KeyLocation::Numpad,
            true,  // pressed
            false, // repeat
            false, // is_synthetic
            egui::Modifiers::default(),
            mode,
            capture_keys,
        )
    }

    fn character(s: &str) -> Key {
        Key::Character(SmolStr::new(s))
    }

    #[test]
    fn numlock_off_digit_is_captured() {
        // NumLock OFF: Numpad7 arrives as the named Home key.
        let event = intercept(
            KeyCode::Numpad7,
            Key::Named(NamedKey::Home),
            NumpadCaptureMode::NumLockAware,
            None,
        )
        .unwrap();
        assert!(event.consumed);
        assert_eq!(event.numlock_on, Some(false));
        assert_eq!(event.keybind_name(), Some("num_7"));
        assert_eq!(event.to_char(), None);
    }

    #[test]
    fn numlock_on_digit_passes_through() {
        let event = intercept(
            KeyCode::Numpad7,
            character("7"),
            NumpadCaptureMode::NumLockAware,
            None,
        )
        .unwrap();
        assert!(!event.consumed);
        assert_eq!(event.numlock_on, Some(true));
        assert_eq!(event.keybind_name(), None, "must not fire keybinds for keys egui handled");
        assert_eq!(event.to_char(), Some('7'));
    }

    #[test]
    fn operators_are_always_captured_in_numlock_aware_mode() {
        let event = intercept(
            KeyCode::NumpadAdd,
            character("+"),
            NumpadCaptureMode::NumLockAware,
            None,
        )
        .unwrap();
        assert!(event.consumed);
        assert_eq!(event.numlock_on, None);
        assert_eq!(event.keybind_name(), Some("num_plus"));
        assert_eq!(event.to_char(), Some('+'));
    }

    #[test]
    fn numpad_enter_is_captured_and_distinguishable() {
        let event = intercept(
            KeyCode::NumpadEnter,
            Key::Named(NamedKey::Enter),
            NumpadCaptureMode::NumLockAware,
            None,
        )
        .unwrap();
        assert!(event.consumed);
        assert_eq!(event.keybind_name(), Some("num_enter"));
        assert_eq!(event.to_char(), None);
    }

    #[test]
    fn always_mode_captures_digits_even_with_numlock_on() {
        // This is how numpad keybinds work on macOS, which has no NumLock.
        let event = intercept(
            KeyCode::Numpad6,
            character("6"),
            NumpadCaptureMode::Always,
            None,
        )
        .unwrap();
        assert!(event.consumed);
        assert_eq!(event.keybind_name(), Some("num_6"));
        assert_eq!(event.to_char(), Some('6'));
    }

    #[test]
    fn off_mode_intercepts_nothing() {
        assert!(
            intercept(
                KeyCode::Numpad7,
                Key::Named(NamedKey::Home),
                NumpadCaptureMode::Off,
                None,
            )
            .is_none()
        );
    }

    #[test]
    fn capture_keys_filter_releases_unbound_keys() {
        let bound: std::collections::HashSet<String> = ["num_6".to_owned()].into();

        // Bound key: captured as usual.
        let event = intercept(
            KeyCode::Numpad6,
            character("6"),
            NumpadCaptureMode::Always,
            Some(&bound),
        )
        .unwrap();
        assert!(event.consumed);
        assert_eq!(event.keybind_name(), Some("num_6"));

        // Unbound key: passes through so it can type normally.
        let event = intercept(
            KeyCode::Numpad2,
            character("2"),
            NumpadCaptureMode::Always,
            Some(&bound),
        )
        .unwrap();
        assert!(!event.consumed);
        assert_eq!(event.keybind_name(), None);
        assert_eq!(event.to_char(), Some('2'));
    }

    #[test]
    fn synthetic_presses_are_ignored() {
        // Windows sends synthetic presses for keys already held when a window
        // gains focus; these must not fire keybinds.
        let event = intercept_numpad_key(
            PhysicalKey::Code(KeyCode::Numpad7),
            &Key::Named(NamedKey::Home),
            KeyLocation::Numpad,
            true, // pressed
            false,
            true, // is_synthetic
            egui::Modifiers::default(),
            NumpadCaptureMode::NumLockAware,
            None,
        );
        assert!(event.is_none());
    }

    #[test]
    fn non_numpad_keys_are_ignored() {
        let event = intercept_numpad_key(
            PhysicalKey::Code(KeyCode::Digit7),
            &character("7"),
            KeyLocation::Standard,
            true,
            false,
            false,
            egui::Modifiers::default(),
            NumpadCaptureMode::NumLockAware,
            None,
        );
        assert!(event.is_none());
    }

    #[test]
    fn locale_decimal_character_is_preserved() {
        // e.g. German layouts produce ',' for NumpadDecimal.
        let event = intercept(
            KeyCode::NumpadDecimal,
            character(","),
            NumpadCaptureMode::Always,
            None,
        )
        .unwrap();
        assert_eq!(event.to_char(), Some(','));
    }

    #[test]
    fn repeat_flag_is_propagated() {
        let event = intercept_numpad_key(
            PhysicalKey::Code(KeyCode::Numpad8),
            &Key::Named(NamedKey::ArrowUp),
            KeyLocation::Numpad,
            true,
            true, // repeat
            false,
            egui::Modifiers::default(),
            NumpadCaptureMode::NumLockAware,
            None,
        )
        .unwrap();
        assert!(event.repeat);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod key_capture_tests {
    use super::intercept_captured_key;
    use std::collections::HashSet;
    use winit::keyboard::{KeyCode, NativeKeyCode, PhysicalKey};

    fn capture() -> HashSet<KeyCode> {
        [KeyCode::Pause, KeyCode::ScrollLock, KeyCode::NumLock].into()
    }

    #[test]
    fn keys_in_set_are_captured() {
        let modifiers = egui::Modifiers::CTRL;
        let event = intercept_captured_key(
            PhysicalKey::Code(KeyCode::Pause),
            true,
            true,
            false,
            modifiers,
            &capture(),
        )
        .unwrap();
        assert_eq!(event.physical_key, PhysicalKey::Code(KeyCode::Pause));
        assert!(event.pressed);
        assert!(event.repeat);
        assert_eq!(event.modifiers, modifiers);
    }

    #[test]
    fn releases_are_captured() {
        let event = intercept_captured_key(
            PhysicalKey::Code(KeyCode::ScrollLock),
            false,
            false,
            false,
            egui::Modifiers::default(),
            &capture(),
        )
        .unwrap();
        assert!(!event.pressed);
    }

    #[test]
    fn keys_not_in_set_are_ignored() {
        let event = intercept_captured_key(
            PhysicalKey::Code(KeyCode::KeyA),
            true,
            false,
            false,
            egui::Modifiers::default(),
            &capture(),
        );
        assert!(event.is_none());
    }

    #[test]
    fn empty_set_captures_nothing() {
        let event = intercept_captured_key(
            PhysicalKey::Code(KeyCode::Pause),
            true,
            false,
            false,
            egui::Modifiers::default(),
            &HashSet::new(),
        );
        assert!(event.is_none());
    }

    #[test]
    fn synthetic_presses_are_ignored() {
        let event = intercept_captured_key(
            PhysicalKey::Code(KeyCode::Pause),
            true,
            false,
            true, // is_synthetic
            egui::Modifiers::default(),
            &capture(),
        );
        assert!(event.is_none());
    }

    #[test]
    fn unidentified_keys_are_ignored() {
        let event = intercept_captured_key(
            PhysicalKey::Unidentified(NativeKeyCode::Unidentified),
            true,
            false,
            false,
            egui::Modifiers::default(),
            &capture(),
        );
        assert!(event.is_none());
    }
}
