//! Platform-agnostic interface for writing apps using [`egui`] (epi = egui programming interface).
//!
//! `epi` provides interfaces for window management and serialization.
//!
//! Start by looking at the [`App`] trait, and implement [`App::update`].

#![warn(missing_docs)] // Let's keep `epi` well-documented.

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
    #[cfg(feature = "glow")]
    pub gl: Option<std::sync::Arc<glow::Context>>,

    /// Can be used to manage GPU resources for custom rendering with WGPU using
    /// [`egui::PaintCallback`]s.
    #[cfg(feature = "wgpu")]
    pub render_state: Option<egui_wgpu::RenderState>,
}

// ----------------------------------------------------------------------------

/// Implement this trait to write apps that can be compiled for both web/wasm and desktop/native using [`eframe`](https://github.com/emilk/egui/tree/master/eframe).
pub trait App {
    /// Called each time the UI needs repainting, which may be many times per second.
    ///
    /// Put your widgets into a [`egui::SidePanel`], [`egui::TopBottomPanel`], [`egui::CentralPanel`], [`egui::Window`] or [`egui::Area`].
    ///
    /// The [`egui::Context`] can be cloned and saved if you like.
    ///
    /// To force a repaint, call [`egui::Context::request_repaint`] at any time (e.g. from another thread).
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame);

    /// Called on shutdown, and perhaps at regular intervals. Allows you to save state.
    ///
    /// Only called when the "persistence" feature is enabled.
    ///
    /// On web the state is stored to "Local Storage".
    /// On native the path is picked using [`directories_next::ProjectDirs::data_dir`](https://docs.rs/directories-next/2.0.0/directories_next/struct.ProjectDirs.html#method.data_dir) which is:
    /// * Linux:   `/home/UserName/.local/share/APPNAME`
    /// * macOS:   `/Users/UserName/Library/Application Support/APPNAME`
    /// * Windows: `C:\Users\UserName\AppData\Roaming\APPNAME`
    ///
    /// where `APPNAME` is what is given to `eframe::run_native`.
    fn save(&mut self, _storage: &mut dyn Storage) {}

    /// Called before an exit that can be aborted.
    /// By returning `false` the exit will be aborted. To continue the exit return `true`.
    ///
    /// A scenario where this method will be run is after pressing the close button on a native
    /// window, which allows you to ask the user whether they want to do something before exiting.
    /// See the example at <https://github.com/emilk/egui/blob/master/examples/confirm_exit/> for practical usage.
    ///
    /// It will _not_ be called on the web or when the window is forcefully closed.
    fn on_exit_event(&mut self) -> bool {
        true
    }

    /// Called once on shutdown, after [`Self::save`].
    ///
    /// If you need to abort an exit use [`Self::on_exit_event`].
    ///
    /// To get a [`glow`] context you need to compile with the `glow` feature flag,
    /// and run eframe with the glow backend.
    #[cfg(feature = "glow")]
    fn on_exit(&mut self, _gl: Option<&glow::Context>) {}

    /// Called once on shutdown, after [`Self::save`].
    ///
    /// If you need to abort an exit use [`Self::on_exit_event`].
    #[cfg(not(feature = "glow"))]
    fn on_exit(&mut self) {}

    // ---------
    // Settings:

    /// Time between automatic calls to [`Self::save`]
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    /// The size limit of the web app canvas.
    ///
    /// By default the max size is [`egui::Vec2::INFINITY`], i.e. unlimited.
    ///
    /// A large canvas can lead to bad frame rates on some older browsers on some platforms
    /// (see <https://bugzilla.mozilla.org/show_bug.cgi?id=1010527#c0>).
    fn max_size_points(&self) -> egui::Vec2 {
        egui::Vec2::INFINITY
    }

    /// Background color for the app, e.g. what is sent to `gl.clearColor`.
    /// This is the background of your windows if you don't set a central panel.
    fn clear_color(&self, _visuals: &egui::Visuals) -> egui::Rgba {
        // NOTE: a bright gray makes the shadows of the windows look weird.
        // We use a bit of transparency so that if the user switches on the
        // `transparent()` option they get immediate results.
        egui::Color32::from_rgba_unmultiplied(12, 12, 12, 180).into()

        // _visuals.window_fill() would also be a natural choice
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

    /// Called each time after the rendering the UI.
    ///
    /// Can be used to access pixel data with `get_pixels`
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

    /// Specify wether or not hardware acceleration is preferred, required, or not.
    ///
    /// Default: [`HardwareAcceleration::Preferred`].
    pub hardware_acceleration: HardwareAcceleration,

    /// What rendering backend to use.
    pub renderer: Renderer,

    /// If the `dark-light` feature is enabled:
    ///
    /// Try to detect and follow the system preferred setting for dark vs light mode.
    ///
    /// By default, this is `true` on Mac and Windows, but `false` on Linux
    /// due to <https://github.com/frewsxcv/rust-dark-light/issues/17>.
    ///
    /// See also [`Self::default_theme`].
    pub follow_system_theme: bool,

    /// Which theme to use in case [`Self::follow_system_theme`] is `false`
    /// or the `dark-light` feature is disabled.
    ///
    /// Default: `Theme::Dark`.
    pub default_theme: Theme,
}

#[cfg(not(target_arch = "wasm32"))]
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
            vsync: true,
            multisampling: 0,
            depth_buffer: 0,
            stencil_buffer: 0,
            hardware_acceleration: HardwareAcceleration::Preferred,
            renderer: Renderer::default(),
            follow_system_theme: cfg!(target_os = "macos") || cfg!(target_os = "windows"),
            default_theme: Theme::Dark,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeOptions {
    /// The theme used by the system.
    #[cfg(feature = "dark-light")]
    pub fn system_theme(&self) -> Option<Theme> {
        if self.follow_system_theme {
            crate::profile_scope!("dark_light::detect");
            match dark_light::detect() {
                dark_light::Mode::Dark => Some(Theme::Dark),
                dark_light::Mode::Light => Some(Theme::Light),
            }
        } else {
            None
        }
    }

    /// The theme used by the system.
    #[cfg(not(feature = "dark-light"))]
    pub fn system_theme(&self) -> Option<Theme> {
        None
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

    /// Which version of WebGl context to select
    ///
    /// Default: [`WebGlContextOption::BestFirst`].
    pub webgl_context_option: WebGlContextOption,
}

#[cfg(target_arch = "wasm32")]
impl Default for WebOptions {
    fn default() -> Self {
        Self {
            follow_system_theme: true,
            default_theme: Theme::Dark,
            webgl_context_option: WebGlContextOption::BestFirst,
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

/// `WebGl` Context options
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

/// Image data for an application icon.
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
pub struct Frame {
    /// Information about the integration.
    #[doc(hidden)]
    pub info: IntegrationInfo,

    /// Where the app can issue commands back to the integration.
    #[doc(hidden)]
    pub output: backend::AppOutput,

    /// A place where you can store custom data in a way that persists when you restart the app.
    #[doc(hidden)]
    pub storage: Option<Box<dyn Storage>>,

    /// A reference to the underlying [`glow`] (OpenGL) context.
    #[cfg(feature = "glow")]
    #[doc(hidden)]
    pub gl: Option<std::sync::Arc<glow::Context>>,

    /// Can be used to manage GPU resources for custom rendering with WGPU using
    /// [`egui::PaintCallback`]s.
    #[cfg(feature = "wgpu")]
    pub render_state: Option<egui_wgpu::RenderState>,
}

impl Frame {
    /// True if you are in a web environment.
    pub fn is_web(&self) -> bool {
        self.info.web_info.is_some()
    }

    /// Information about the integration.
    pub fn info(&self) -> IntegrationInfo {
        self.info.clone()
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
    /// and run eframe with the glow backend.
    #[cfg(feature = "glow")]
    pub fn gl(&self) -> Option<&std::sync::Arc<glow::Context>> {
        self.gl.as_ref()
    }

    /// Signal the app to stop/exit/quit the app (only works for native apps, not web apps).
    /// The framework will not quit immediately, but at the end of the this frame.
    pub fn quit(&mut self) {
        self.output.quit = true;
    }

    /// Set the desired inner size of the window (in egui points).
    pub fn set_window_size(&mut self, size: egui::Vec2) {
        self.output.window_size = Some(size);
    }

    /// Set the desired title of the window.
    pub fn set_window_title(&mut self, title: &str) {
        self.output.window_title = Some(title.to_owned());
    }

    /// Set whether to show window decorations (i.e. a frame around you app).
    /// If false it will be difficult to move and resize the app.
    pub fn set_decorations(&mut self, decorated: bool) {
        self.output.decorated = Some(decorated);
    }

    /// set the position of the outer window
    pub fn set_window_pos(&mut self, pos: egui::Pos2) {
        self.output.window_pos = Some(pos);
    }

    /// When called, the native window will follow the
    /// movement of the cursor while the primary mouse button is down.
    ///
    /// Does not work on the web.
    pub fn drag_window(&mut self) {
        self.output.drag_window = true;
    }

    /// Set the visibility of the window.
    pub fn set_visible(&mut self, visible: bool) {
        self.output.visible = Some(visible);
    }

    /// for integrations only: call once per frame
    #[doc(hidden)]
    pub fn take_app_output(&mut self) -> backend::AppOutput {
        std::mem::take(&mut self.output)
    }
}

/// Information about the web environment (if applicable).
#[derive(Clone, Debug)]
pub struct WebInfo {
    /// Information about the URL.
    pub location: Location,
}

/// Information about the application's main window, if available.
#[derive(Clone, Debug)]
pub struct WindowInfo {
    /// Coordinates of the window's outer top left corner, relative to the top left corner of the first display.
    /// Unit: egui points (logical pixels).
    pub position: egui::Pos2,

    /// Window inner size in egui points (logical pixels).
    pub size: egui::Vec2,
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
    /// If the app is running in a Web context, this returns information about the environment.
    pub web_info: Option<WebInfo>,

    /// Does the OS use dark or light mode?
    ///
    /// `None` means "don't know".
    pub system_theme: Option<Theme>,

    /// Seconds of cpu usage (in seconds) of UI code on the previous frame.
    /// `None` if this is the first frame.
    pub cpu_usage: Option<f32>,

    /// The OS native pixels-per-point
    pub native_pixels_per_point: Option<f32>,

    /// Window-specific geometry information, if provided by the platform.
    pub window_info: Option<WindowInfo>,
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
#[doc(hidden)]
pub mod backend {
    /// Action that can be taken by the user app.
    #[derive(Clone, Debug, Default)]
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

        /// Set to some position to move the outer window (e.g. glium window) to this position
        pub window_pos: Option<egui::Pos2>,

        /// Set to some bool to change window visibility.
        pub visible: Option<bool>,
    }
}
