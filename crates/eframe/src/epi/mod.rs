//! Platform-agnostic interface for writing apps using [`egui`] (epi = egui programming interface).
//!
//! `epi` provides interfaces for window management and serialization.
//!
//! Start by looking at the [`App`] trait, and implement [`App::update`].

#![warn(missing_docs)] // Let's keep `epi` well-documented.

#[cfg(not(target_arch = "wasm32"))]
mod icon_data;

#[cfg(not(target_arch = "wasm32"))]
pub use icon_data::IconData;

#[cfg(target_arch = "wasm32")]
use std::any::Any;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(any(feature = "glow", feature = "wgpu"))]
pub use crate::native::run::UserEvent;

#[cfg(not(target_arch = "wasm32"))]
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
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
pub type WindowBuilderHook = Box<dyn FnOnce(WindowBuilder) -> WindowBuilder>;

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

    /// The underlying WGPU render state.
    ///
    /// Only available when compiling with the `wgpu` feature and using [`Renderer::Wgpu`].
    ///
    /// Can be used to manage GPU resources for custom rendering with WGPU using [`egui::PaintCallback`]s.
    #[cfg(feature = "wgpu")]
    pub wgpu_render_state: Option<egui_wgpu::RenderState>,

    /// Raw platform window handle
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_window_handle: RawWindowHandle,

    /// Raw platform display handle for window
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_display_handle: RawDisplayHandle,
}

// Implementing `Clone` would violate the guarantees of `HasRawWindowHandle` and `HasRawDisplayHandle`.
#[cfg(not(target_arch = "wasm32"))]
assert_not_impl_any!(CreationContext<'_>: Clone);

#[allow(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
unsafe impl HasRawWindowHandle for CreationContext<'_> {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.raw_window_handle
    }
}

#[allow(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
unsafe impl HasRawDisplayHandle for CreationContext<'_> {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.raw_display_handle
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

    /// Called when the user attempts to close the desktop window and/or quit the application.
    ///
    /// By returning `false` the closing will be aborted. To continue the closing return `true`.
    ///
    /// A scenario where this method will be run is after pressing the close button on a native
    /// window, which allows you to ask the user whether they want to do something before exiting.
    /// See the example at <https://github.com/emilk/egui/blob/master/examples/confirm_exit/> for practical usage.
    ///
    /// It will _not_ be called on the web or when the window is forcefully closed.
    #[cfg(not(target_arch = "wasm32"))]
    #[doc(alias = "exit")]
    #[doc(alias = "quit")]
    fn on_close_event(&mut self) -> bool {
        true
    }

    /// Called once on shutdown, after [`Self::save`].
    ///
    /// If you need to abort an exit use [`Self::on_close_event`].
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

    /// If `true` a warm-up call to [`Self::update`] will be issued where
    /// `ctx.memory(|mem| mem.everything_is_visible())` will be set to `true`.
    ///
    /// This can help pre-caching resources loaded by different parts of the UI, preventing stutter later on.
    ///
    /// In this warm-up call, all painted shapes will be ignored.
    ///
    /// The default is `false`, and it is unlikely you will want to change this.
    fn warm_up_enabled(&self) -> bool {
        false
    }

    /// Called each time after the rendering the UI.
    ///
    /// Can be used to access pixel data with [`Frame::screenshot`]
    fn post_rendering(&mut self, _window_size_px: [u32; 2], _frame: &Frame) {}
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
/// Only a single native window is currently supported.
#[cfg(not(target_arch = "wasm32"))]
pub struct NativeOptions {
    /// Sets whether or not the window will always be on top of other windows at initialization.
    pub always_on_top: bool,

    /// Show window in maximized mode
    pub maximized: bool,

    /// On desktop: add window decorations (i.e. a frame around your app)?
    /// If false it will be difficult to move and resize the app.
    pub decorated: bool,

    /// Start in (borderless) fullscreen?
    ///
    /// Default: `false`.
    pub fullscreen: bool,

    /// On Mac: the window doesn't have a titlebar, but floating window buttons.
    ///
    /// See [winit's documentation][with_fullsize_content_view] for information on Mac-specific options.
    ///
    /// [with_fullsize_content_view]: https://docs.rs/winit/latest/x86_64-apple-darwin/winit/platform/macos/trait.WindowBuilderExtMacOS.html#tymethod.with_fullsize_content_view
    #[cfg(target_os = "macos")]
    pub fullsize_content: bool,

    /// On Windows: enable drag and drop support. Drag and drop can
    /// not be disabled on other platforms.
    ///
    /// See [winit's documentation][drag_and_drop] for information on why you
    /// might want to disable this on windows.
    ///
    /// [drag_and_drop]: https://docs.rs/winit/latest/x86_64-pc-windows-msvc/winit/platform/windows/trait.WindowBuilderExtWindows.html#tymethod.with_drag_and_drop
    pub drag_and_drop_support: bool,

    /// The application icon, e.g. in the Windows task bar or the alt-tab menu.
    ///
    /// The default icon is a white `e` on a black background (for "egui" or "eframe").
    /// If you prefer the OS default, set this to `None`.
    pub icon_data: Option<IconData>,

    /// The initial (inner) position of the native window in points (logical pixels).
    pub initial_window_pos: Option<egui::Pos2>,

    /// The initial inner size of the native window in points (logical pixels).
    pub initial_window_size: Option<egui::Vec2>,

    /// The minimum inner window size in points (logical pixels).
    pub min_window_size: Option<egui::Vec2>,

    /// The maximum inner window size in points (logical pixels).
    pub max_window_size: Option<egui::Vec2>,

    /// Should the app window be resizable?
    pub resizable: bool,

    /// On desktop: make the window transparent.
    ///
    /// You control the transparency with [`App::clear_color()`].
    /// You should avoid having a [`egui::CentralPanel`], or make sure its frame is also transparent.
    pub transparent: bool,

    /// On desktop: mouse clicks pass through the window, used for non-interactable overlays
    /// Generally you would use this in conjunction with always_on_top
    pub mouse_passthrough: bool,

    /// Whether grant focus when window initially opened. True by default.
    pub active: bool,

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
    /// When `true`, [`winit::platform::run_return::EventLoopExtRunReturn::run_return`] is used.
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

    /// The application id, used for determining the folder to persist the app to.
    ///
    /// On native the path is picked using [`crate::storage_dir`].
    ///
    /// If you don't set [`Self::app_id`], the title argument to [`crate::run_native`]
    /// will be used as app id instead.
    ///
    /// ### On Wayland
    /// On Wayland this sets the Application ID for the window.
    ///
    /// The application ID is used in several places of the compositor, e.g. for
    /// grouping windows of the same application. It is also important for
    /// connecting the configuration of a `.desktop` file with the window, by
    /// using the application ID as file name. This allows e.g. a proper icon
    /// handling under Wayland.
    ///
    /// See [Waylands XDG shell documentation][xdg-shell] for more information
    /// on this Wayland-specific option.
    ///
    /// [xdg-shell]: https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:set_app_id
    ///
    /// # Example
    /// ``` no_run
    /// fn main() -> eframe::Result<()> {
    ///
    ///     let mut options = eframe::NativeOptions::default();
    ///     // Set the application ID for Wayland only on Linux
    ///     #[cfg(target_os = "linux")]
    ///     {
    ///         options.app_id = Some("egui-example".to_string());
    ///     }
    ///
    ///     eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
    ///         egui::CentralPanel::default().show(ctx, |ui| {
    ///             ui.heading("My egui Application");
    ///         });
    ///     })
    /// }
    /// ```
    pub app_id: Option<String>,

    /// Controls whether or not the native window position and size will be
    /// persisted (only if the "persistence" feature is enabled).
    pub persist_window: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl Clone for NativeOptions {
    fn clone(&self) -> Self {
        Self {
            icon_data: self.icon_data.clone(),

            #[cfg(any(feature = "glow", feature = "wgpu"))]
            event_loop_builder: None, // Skip any builder callbacks if cloning

            #[cfg(any(feature = "glow", feature = "wgpu"))]
            window_builder: None, // Skip any builder callbacks if cloning

            #[cfg(feature = "wgpu")]
            wgpu_options: self.wgpu_options.clone(),

            app_id: self.app_id.clone(),

            ..*self
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for NativeOptions {
    fn default() -> Self {
        Self {
            always_on_top: false,
            maximized: false,
            decorated: true,
            fullscreen: false,

            #[cfg(target_os = "macos")]
            fullsize_content: false,

            // We set a default "egui" or "eframe" icon, which is usually more distinctive than the default OS icon.
            icon_data: Some(
                IconData::try_from_png_bytes(&include_bytes!("../../data/icon.png")[..]).unwrap(),
            ),

            drag_and_drop_support: true,
            initial_window_pos: None,
            initial_window_size: None,
            min_window_size: None,
            max_window_size: None,
            resizable: true,
            transparent: false,
            mouse_passthrough: false,

            active: true,

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

            app_id: None,

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
        #[cfg(feature = "glow")]
        return Self::Glow;

        #[cfg(not(feature = "glow"))]
        #[cfg(feature = "wgpu")]
        return Self::Wgpu;

        #[cfg(not(feature = "glow"))]
        #[cfg(not(feature = "wgpu"))]
        compile_error!("eframe: you must enable at least one of the rendering backend features: 'glow' or 'wgpu'");
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
/// allocate textures, and change settings (e.g. window size).
pub struct Frame {
    /// Information about the integration.
    pub(crate) info: IntegrationInfo,

    /// Where the app can issue commands back to the integration.
    pub(crate) output: backend::AppOutput,

    /// A place where you can store custom data in a way that persists when you restart the app.
    pub(crate) storage: Option<Box<dyn Storage>>,

    /// A reference to the underlying [`glow`] (OpenGL) context.
    #[cfg(feature = "glow")]
    pub(crate) gl: Option<std::sync::Arc<glow::Context>>,

    /// Can be used to manage GPU resources for custom rendering with WGPU using [`egui::PaintCallback`]s.
    #[cfg(feature = "wgpu")]
    pub(crate) wgpu_render_state: Option<egui_wgpu::RenderState>,

    /// If [`Frame::request_screenshot`] was called during a frame, this field will store the screenshot
    /// such that it can be retrieved during [`App::post_rendering`] with [`Frame::screenshot`]
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) screenshot: std::cell::Cell<Option<egui::ColorImage>>,

    /// Raw platform window handle
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_window_handle: RawWindowHandle,

    /// Raw platform display handle for window
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) raw_display_handle: RawDisplayHandle,
}

// Implementing `Clone` would violate the guarantees of `HasRawWindowHandle` and `HasRawDisplayHandle`.
#[cfg(not(target_arch = "wasm32"))]
assert_not_impl_any!(Frame: Clone);

#[allow(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
unsafe impl HasRawWindowHandle for Frame {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.raw_window_handle
    }
}

#[allow(unsafe_code)]
#[cfg(not(target_arch = "wasm32"))]
unsafe impl HasRawDisplayHandle for Frame {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.raw_display_handle
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

    /// Request the current frame's pixel data. Needs to be retrieved by calling [`Frame::screenshot`]
    /// during [`App::post_rendering`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn request_screenshot(&mut self) {
        self.output.screenshot_requested = true;
    }

    /// Cancel a request made with [`Frame::request_screenshot`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cancel_screenshot_request(&mut self) {
        self.output.screenshot_requested = false;
    }

    /// During [`App::post_rendering`], use this to retrieve the pixel data that was requested during
    /// [`App::update`] via [`Frame::request_screenshot`].
    ///
    /// Returns None if:
    /// * Called in [`App::update`]
    /// * [`Frame::request_screenshot`] wasn't called on this frame during [`App::update`]
    /// * The rendering backend doesn't support this feature (yet). Currently implemented for wgpu and glow, but not with wasm as target.
    /// * Wgpu's GL target is active (not yet supported)
    /// * Retrieving the data was unsuccessful in some way.
    ///
    /// See also [`egui::ColorImage::region`]
    ///
    /// ## Example generating a capture of everything within a square of 100 pixels located at the top left of the app and saving it with the [`image`](crates.io/crates/image) crate:
    /// ```
    /// struct MyApp;
    ///
    /// impl eframe::App for MyApp {
    ///     fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    ///         // In real code the app would render something here
    ///         frame.request_screenshot();
    ///         // Things that are added to the frame after the call to
    ///         // request_screenshot() will still be included.
    ///     }
    ///
    ///     fn post_rendering(&mut self, _window_size: [u32; 2], frame: &eframe::Frame) {
    ///         if let Some(screenshot) = frame.screenshot() {
    ///             let pixels_per_point = frame.info().native_pixels_per_point;
    ///             let region = egui::Rect::from_two_pos(
    ///                 egui::Pos2::ZERO,
    ///                 egui::Pos2{ x: 100., y: 100. },
    ///             );
    ///             let top_left_corner = screenshot.region(&region, pixels_per_point);
    ///             image::save_buffer(
    ///                 "top_left.png",
    ///                 top_left_corner.as_raw(),
    ///                 top_left_corner.width() as u32,
    ///                 top_left_corner.height() as u32,
    ///                 image::ColorType::Rgba8,
    ///             ).unwrap();
    ///         }
    ///     }
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn screenshot(&self) -> Option<egui::ColorImage> {
        self.screenshot.take()
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

    /// Tell `eframe` to close the desktop window.
    ///
    /// The window will not close immediately, but at the end of the this frame.
    ///
    /// Calling this will likely result in the app quitting, unless
    /// you have more code after the call to [`crate::run_native`].
    #[cfg(not(target_arch = "wasm32"))]
    #[doc(alias = "exit")]
    #[doc(alias = "quit")]
    pub fn close(&mut self) {
        log::debug!("eframe::Frame::close called");
        self.output.close = true;
    }

    /// Minimize or unminimize window. (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_minimized(&mut self, minimized: bool) {
        self.output.minimized = Some(minimized);
    }

    /// Bring the window into focus (native only). Has no effect on Wayland, or if the window is minimized or invisible.
    ///
    /// This method puts the window on top of other applications and takes input focus away from them,
    /// which, if unexpected, will disturb the user.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn focus(&mut self) {
        self.output.focus = Some(true);
    }

    /// If the window is unfocused, attract the user's attention (native only).
    ///
    /// Typically, this means that the window will flash on the taskbar, or bounce, until it is interacted with.
    ///
    /// When the window comes into focus, or if `None` is passed, the attention request will be automatically reset.
    ///
    /// See [winit's documentation][user_attention_details] for platform-specific effect details.
    ///
    /// [user_attention_details]: https://docs.rs/winit/latest/winit/window/enum.UserAttentionType.html
    #[cfg(not(target_arch = "wasm32"))]
    pub fn request_user_attention(&mut self, kind: egui::UserAttentionType) {
        self.output.attention = Some(kind);
    }

    /// Maximize or unmaximize window. (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_maximized(&mut self, maximized: bool) {
        self.output.maximized = Some(maximized);
    }

    /// Tell `eframe` to close the desktop window.
    #[cfg(not(target_arch = "wasm32"))]
    #[deprecated = "Renamed `close`"]
    pub fn quit(&mut self) {
        self.close();
    }

    /// Set the desired inner size of the window (in egui points).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_window_size(&mut self, size: egui::Vec2) {
        self.output.window_size = Some(size);
        self.info.window_info.size = size; // so that subsequent calls see the updated value
    }

    /// Set the desired title of the window.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_window_title(&mut self, title: &str) {
        self.output.window_title = Some(title.to_owned());
    }

    /// Set whether to show window decorations (i.e. a frame around you app).
    ///
    /// If false it will be difficult to move and resize the app.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_decorations(&mut self, decorated: bool) {
        self.output.decorated = Some(decorated);
    }

    /// Turn borderless fullscreen on/off (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        self.output.fullscreen = Some(fullscreen);
        self.info.window_info.fullscreen = fullscreen; // so that subsequent calls see the updated value
    }

    /// set the position of the outer window.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_window_pos(&mut self, pos: egui::Pos2) {
        self.output.window_pos = Some(pos);
        self.info.window_info.position = Some(pos); // so that subsequent calls see the updated value
    }

    /// When called, the native window will follow the
    /// movement of the cursor while the primary mouse button is down.
    ///
    /// Does not work on the web.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn drag_window(&mut self) {
        self.output.drag_window = true;
    }

    /// Set the visibility of the window.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_visible(&mut self, visible: bool) {
        self.output.visible = Some(visible);
    }

    /// On desktop: Set the window always on top.
    ///
    /// (Wayland desktop currently not supported)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_always_on_top(&mut self, always_on_top: bool) {
        self.output.always_on_top = Some(always_on_top);
    }

    /// On desktop: Set the window to be centered.
    ///
    /// (Wayland desktop currently not supported)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_centered(&mut self) {
        if let Some(monitor_size) = self.info.window_info.monitor_size {
            let inner_size = self.info.window_info.size;
            if monitor_size.x > 1.0 && monitor_size.y > 1.0 {
                let x = (monitor_size.x - inner_size.x) / 2.0;
                let y = (monitor_size.y - inner_size.y) / 2.0;
                self.set_window_pos(egui::Pos2 { x, y });
            }
        }
    }

    /// for integrations only: call once per frame
    #[cfg(any(feature = "glow", feature = "wgpu"))]
    pub(crate) fn take_app_output(&mut self) -> backend::AppOutput {
        std::mem::take(&mut self.output)
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

/// Information about the application's main window, if available.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct WindowInfo {
    /// Coordinates of the window's outer top left corner, relative to the top left corner of the first display.
    ///
    /// Unit: egui points (logical pixels).
    ///
    /// `None` = unknown.
    pub position: Option<egui::Pos2>,

    /// Are we in fullscreen mode?
    pub fullscreen: bool,

    /// Are we minimized?
    pub minimized: bool,

    /// Are we maximized?
    pub maximized: bool,

    /// Is the window focused and able to receive input?
    ///
    /// This should be the same as [`egui::InputState::focused`].
    pub focused: bool,

    /// Window inner size in egui points (logical pixels).
    pub size: egui::Vec2,

    /// Current monitor size in egui points (logical pixels)
    pub monitor_size: Option<egui::Vec2>,
}

/// Information about the URL.
///
/// Everything has been percent decoded (`%20` -> ` ` etc).
#[cfg(target_arch = "wasm32")]
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
    /// Use [`Self::query_map`] to get the parsed version of it.
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
    /// Information about the surrounding web environment.
    #[cfg(target_arch = "wasm32")]
    pub web_info: WebInfo,

    /// Does the OS use dark or light mode?
    ///
    /// `None` means "don't know".
    pub system_theme: Option<Theme>,

    /// Seconds of cpu usage (in seconds) of UI code on the previous frame.
    /// `None` if this is the first frame.
    pub cpu_usage: Option<f32>,

    /// The OS native pixels-per-point
    pub native_pixels_per_point: Option<f32>,

    /// The position and size of the native window.
    #[cfg(not(target_arch = "wasm32"))]
    pub window_info: WindowInfo,
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

// ----------------------------------------------------------------------------

/// You only need to look here if you are writing a backend for `epi`.
pub(crate) mod backend {
    /// Action that can be taken by the user app.
    #[derive(Clone, Debug, Default)]
    #[must_use]
    pub struct AppOutput {
        /// Set to `true` to close the native window (which often quits the app).
        #[cfg(not(target_arch = "wasm32"))]
        pub close: bool,

        /// Set to some size to resize the outer window (e.g. glium window) to this size.
        #[cfg(not(target_arch = "wasm32"))]
        pub window_size: Option<egui::Vec2>,

        /// Set to some string to rename the outer window (e.g. glium window) to this title.
        #[cfg(not(target_arch = "wasm32"))]
        pub window_title: Option<String>,

        /// Set to some bool to change window decorations.
        #[cfg(not(target_arch = "wasm32"))]
        pub decorated: Option<bool>,

        /// Set to some bool to change window fullscreen.
        #[cfg(not(target_arch = "wasm32"))] // TODO: implement fullscreen on web
        pub fullscreen: Option<bool>,

        /// Set to true to drag window while primary mouse button is down.
        #[cfg(not(target_arch = "wasm32"))]
        pub drag_window: bool,

        /// Set to some position to move the outer window (e.g. glium window) to this position
        #[cfg(not(target_arch = "wasm32"))]
        pub window_pos: Option<egui::Pos2>,

        /// Set to some bool to change window visibility.
        #[cfg(not(target_arch = "wasm32"))]
        pub visible: Option<bool>,

        /// Set to some bool to tell the window always on top.
        #[cfg(not(target_arch = "wasm32"))]
        pub always_on_top: Option<bool>,

        /// Set to some bool to minimize or unminimize window.
        #[cfg(not(target_arch = "wasm32"))]
        pub minimized: Option<bool>,

        /// Set to some bool to maximize or unmaximize window.
        #[cfg(not(target_arch = "wasm32"))]
        pub maximized: Option<bool>,

        /// Set to some bool to focus window.
        #[cfg(not(target_arch = "wasm32"))]
        pub focus: Option<bool>,

        /// Set to request a user's attention to the native window.
        #[cfg(not(target_arch = "wasm32"))]
        pub attention: Option<egui::UserAttentionType>,

        #[cfg(not(target_arch = "wasm32"))]
        pub screenshot_requested: bool,
    }
}
