//! egui supports multiple viewports, corresponding to multiple native windows.
//!
//! Not all egui backends support multiple viewports, but `eframe` native does
//! (but not on web).
//!
//! You can spawn a new viewport using [`Context::show_viewport_deferred`] and [`Context::show_viewport_immediate`].
//! These needs to be called every frame the viewport should be visible.
//!
//! This is implemented by the native `eframe` backend, but not the web one.
//!
//! ## Viewport classes
//! The viewports form a tree of parent-child relationships.
//!
//! There are different classes of viewports.
//!
//! ### Root viewport
//! The root viewport is the original viewport, and cannot be closed without closing the application.
//!
//! ### Deferred viewports
//! These are created with [`Context::show_viewport_deferred`].
//! Deferred viewports take a closure that is called by the integration at a later time, perhaps multiple times.
//! Deferred viewports are repainted independently of the parent viewport.
//! This means communication with them needs to be done via channels, or `Arc/Mutex`.
//!
//! This is the most performant type of child viewport, though a bit more cumbersome to work with compared to immediate viewports.
//!
//! ### Immediate viewports
//! These are created with [`Context::show_viewport_immediate`].
//! Immediate viewports take a `FnOnce` closure, similar to other egui functions, and is called immediately.
//! This makes communication with them much simpler than with deferred viewports, but this simplicity comes at a cost: whenever the parent viewports needs to be repainted, so will the child viewport, and vice versa.
//! This means that if you have `N` viewports you are potentially doing `N` times as much CPU work. However, if all your viewports are showing animations, and thus are repainting constantly anyway, this doesn't matter.
//!
//! In short: immediate viewports are simpler to use, but can waste a lot of CPU time.
//!
//! ### Embedded viewports
//! These are not real, independent viewports, but is a fallback mode for when the integration does not support real viewports.
//! In your callback is called with [`ViewportClass::EmbeddedWindow`] it means the viewport is embedded inside of
//! a regular [`crate::Window`], trapped in the parent viewport.
//!
//!
//! ## Using the viewports
//! Only one viewport is active at any one time, identified with [`Context::viewport_id`].
//! You can modify the current (change the title, resize the window, etc) by sending
//! a [`ViewportCommand`] to it using [`Context::send_viewport_cmd`].
//! You can interact with other viewports using [`Context::send_viewport_cmd_to`].
//!
//! There is an example in <https://github.com/emilk/egui/tree/main/examples/multiple_viewports/src/main.rs>.
//!
//! You can find all available viewports in [`crate::RawInput::viewports`] and the active viewport in
//! [`crate::InputState::viewport`]:
//!
//! ```no_run
//! # let ctx = &egui::Context::default();
//! ctx.input(|i| {
//!     dbg!(&i.viewport()); // Current viewport
//!     dbg!(&i.raw.viewports); // All viewports
//! });
//! ```
//!
//! ## For integrations
//! * There is a [`crate::InputState::viewport`] with information about the current viewport.
//! * There is a [`crate::RawInput::viewports`] with information about all viewports.
//! * The repaint callback set by [`Context::set_request_repaint_callback`] points to which viewport should be repainted.
//! * [`crate::FullOutput::viewport_output`] is a list of viewports which should result in their own independent windows.
//! * To support immediate viewports you need to call [`Context::set_immediate_viewport_renderer`].
//! * If you support viewports, you need to call [`Context::set_embed_viewports`] with `false`, or all new viewports will be embedded (the default behavior).
//!
//! ## Future work
//! There are several more things related to viewports that we want to add.
//! Read more at <https://github.com/emilk/egui/issues/3556>.

use std::sync::Arc;

use epaint::{Pos2, Vec2};

use crate::{Context, Id, Ui};

// ----------------------------------------------------------------------------

/// The different types of viewports supported by egui.
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ViewportClass {
    /// The root viewport; i.e. the original window.
    #[default]
    Root,

    /// A viewport run independently from the parent viewport.
    ///
    /// This is the preferred type of viewport from a performance perspective.
    ///
    /// Create these with [`crate::Context::show_viewport_deferred`].
    Deferred,

    /// A viewport run inside the parent viewport.
    ///
    /// This is the easier type of viewport to use, but it is less performant
    /// at it requires both parent and child to repaint if any one of them needs repainting,
    /// which effectively produces double work for two viewports, and triple work for three viewports, etc.
    ///
    /// Create these with [`crate::Context::show_viewport_immediate`].
    Immediate,

    /// The fallback, when the egui integration doesn't support viewports,
    /// or [`crate::Context::embed_viewports`] is set to `true`.
    ///
    /// If you get this, it is because you are already wrapped in a [`crate::Window`]
    /// inside of the parent viewport.
    EmbeddedWindow,
}

// ----------------------------------------------------------------------------

/// A unique identifier of a viewport.
///
/// This is returned by [`Context::viewport_id`] and [`Context::parent_viewport_id`].
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ViewportId(pub Id);

// We implement `PartialOrd` and `Ord` so we can use `ViewportId` in a `BTreeMap`,
// which allows predicatable iteration order, frame-to-frame.
impl PartialOrd for ViewportId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ViewportId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.value().cmp(&other.0.value())
    }
}

impl Default for ViewportId {
    #[inline]
    fn default() -> Self {
        Self::ROOT
    }
}

impl std::fmt::Debug for ViewportId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.short_debug_format().fmt(f)
    }
}

impl ViewportId {
    /// The `ViewportId` of the root viewport.
    pub const ROOT: Self = Self(Id::NULL);

    #[inline]
    pub fn from_hash_of(source: impl std::hash::Hash) -> Self {
        Self(Id::new(source))
    }
}

impl From<ViewportId> for Id {
    #[inline]
    fn from(id: ViewportId) -> Self {
        id.0
    }
}

impl nohash_hasher::IsEnabled for ViewportId {}

/// A fast hash set of [`ViewportId`].
pub type ViewportIdSet = nohash_hasher::IntSet<ViewportId>;

/// A fast hash map from [`ViewportId`] to `T`.
pub type ViewportIdMap<T> = nohash_hasher::IntMap<ViewportId, T>;

/// An order map from [`ViewportId`] to `T`.
pub type OrderedViewportIdMap<T> = std::collections::BTreeMap<ViewportId, T>;

// ----------------------------------------------------------------------------

/// Image data for an application icon.
///
/// Use a square image, e.g. 256x256 pixels.
/// You can use a transparent background.
#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct IconData {
    /// RGBA pixels, with separate/unmultiplied alpha.
    pub rgba: Vec<u8>,

    /// Image width. This should be a multiple of 4.
    pub width: u32,

    /// Image height. This should be a multiple of 4.
    pub height: u32,
}

impl IconData {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rgba.is_empty()
    }
}

impl std::fmt::Debug for IconData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IconData")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl From<IconData> for epaint::ColorImage {
    fn from(icon: IconData) -> Self {
        profiling::function_scope!();
        let IconData {
            rgba,
            width,
            height,
        } = icon;
        Self::from_rgba_premultiplied([width as usize, height as usize], &rgba)
    }
}

impl From<&IconData> for epaint::ColorImage {
    fn from(icon: &IconData) -> Self {
        profiling::function_scope!();
        let IconData {
            rgba,
            width,
            height,
        } = icon;
        Self::from_rgba_premultiplied([*width as usize, *height as usize], rgba)
    }
}

// ----------------------------------------------------------------------------

/// A pair of [`ViewportId`], used to identify a viewport and its parent.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ViewportIdPair {
    pub this: ViewportId,
    pub parent: ViewportId,
}

impl Default for ViewportIdPair {
    #[inline]
    fn default() -> Self {
        Self::ROOT
    }
}

impl ViewportIdPair {
    /// The `ViewportIdPair` of the root viewport, which is its own parent.
    pub const ROOT: Self = Self {
        this: ViewportId::ROOT,
        parent: ViewportId::ROOT,
    };

    #[inline]
    pub fn from_self_and_parent(this: ViewportId, parent: ViewportId) -> Self {
        Self { this, parent }
    }
}

/// The user-code that shows the ui in the viewport, used for deferred viewports.
pub type DeferredViewportUiCallback = dyn Fn(&mut Ui) + Sync + Send;

/// Render the given viewport, calling the given ui callback.
pub type ImmediateViewportRendererCallback = dyn for<'a> Fn(&Context, ImmediateViewport<'a>);

/// Control the building of a new egui viewport (i.e. native window).
///
/// See [`crate::viewport`] for how to build new viewports (native windows).
///
/// The fields are public, but you should use the builder pattern to set them,
/// and that's where you'll find the documentation too.
///
/// Since egui is immediate mode, `ViewportBuilder` is accumulative in nature.
/// Setting any option to `None` means "keep the current value",
/// or "Use the default" if it is the first call.
///
/// The default values are implementation defined, so you may want to explicitly
/// configure the size of the window, and what buttons are shown.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ViewportBuilder {
    /// The title of the viewport.
    /// `eframe` will use this as the title of the native window.
    pub title: Option<String>,

    /// This is wayland only. See [`Self::with_app_id`].
    pub app_id: Option<String>,

    /// The desired outer position of the window.
    pub position: Option<Pos2>,
    pub inner_size: Option<Vec2>,
    pub min_inner_size: Option<Vec2>,
    pub max_inner_size: Option<Vec2>,

    /// Whether clamp the window's size to monitor's size. The default is `true` on linux, otherwise it is `false`.
    ///
    /// Note: On some Linux systems, a window size larger than the monitor causes crashes
    pub clamp_size_to_monitor_size: Option<bool>,

    pub fullscreen: Option<bool>,
    pub maximized: Option<bool>,
    pub resizable: Option<bool>,
    pub transparent: Option<bool>,
    pub decorations: Option<bool>,
    pub icon: Option<Arc<IconData>>,
    pub active: Option<bool>,
    pub visible: Option<bool>,

    // macOS:
    pub fullsize_content_view: Option<bool>,
    pub movable_by_window_background: Option<bool>,
    pub title_shown: Option<bool>,
    pub titlebar_buttons_shown: Option<bool>,
    pub titlebar_shown: Option<bool>,
    pub has_shadow: Option<bool>,

    // windows:
    pub drag_and_drop: Option<bool>,
    pub taskbar: Option<bool>,

    pub close_button: Option<bool>,
    pub minimize_button: Option<bool>,
    pub maximize_button: Option<bool>,

    pub window_level: Option<WindowLevel>,

    pub mouse_passthrough: Option<bool>,

    // X11
    pub window_type: Option<X11WindowType>,
    pub override_redirect: Option<bool>,
}

impl ViewportBuilder {
    /// Sets the initial title of the window in the title bar.
    ///
    /// Look at winit for more details
    #[inline]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets whether the window should have a border, a title bar, etc.
    ///
    /// The default is `true`.
    ///
    /// Look at winit for more details
    #[inline]
    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = Some(decorations);
        self
    }

    /// Sets whether the window should be put into fullscreen upon creation.
    ///
    /// The default is `None`.
    ///
    /// Look at winit for more details
    /// This will use borderless
    #[inline]
    pub fn with_fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = Some(fullscreen);
        self
    }

    /// Request that the window is maximized upon creation.
    ///
    /// The default is `false`.
    ///
    /// Look at winit for more details
    #[inline]
    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = Some(maximized);
        self
    }

    /// Sets whether the window is resizable or not.
    ///
    /// The default is `true`.
    ///
    /// Look at winit for more details
    #[inline]
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = Some(resizable);
        self
    }

    /// Sets whether the background of the window should be transparent.
    ///
    /// You should avoid having a [`crate::CentralPanel`], or make sure its frame is also transparent.
    ///
    /// In `eframe` you control the transparency with `eframe::App::clear_color()`.
    ///
    /// If this is `true`, writing colors with alpha values different than
    /// `1.0` will produce a transparent window. On some platforms this
    /// is more of a hint for the system and you'd still have the alpha
    /// buffer.
    ///
    /// The default is `false`.
    /// If this is not working, it's because the graphic context doesn't support transparency,
    /// you will need to set the transparency in the eframe!
    ///
    /// ## Platform-specific
    ///
    /// **macOS:** When using this feature to create an overlay-like UI, you likely want to combine this with [`Self::with_has_shadow`] set to `false` in order to avoid ghosting artifacts.
    #[inline]
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = Some(transparent);
        self
    }

    /// The application icon, e.g. in the Windows task bar or the alt-tab menu.
    ///
    /// The default icon is a white `e` on a black background (for "egui" or "eframe").
    /// If you prefer the OS default, set this to `IconData::default()`.
    #[inline]
    pub fn with_icon(mut self, icon: impl Into<Arc<IconData>>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Whether the window will be initially focused or not.
    ///
    /// The window should be assumed as not focused by default
    ///
    /// ## Platform-specific:
    ///
    /// **Android / iOS / X11 / Wayland / Orbital:** Unsupported.
    ///
    /// Look at winit for more details
    #[inline]
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    /// Sets whether the window will be initially visible or hidden.
    ///
    /// The default is to show the window.
    ///
    /// Look at winit for more details
    #[inline]
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    /// macOS: Makes the window content appear behind the titlebar.
    ///
    /// You often want to combine this with [`Self::with_titlebar_shown`]
    /// and [`Self::with_title_shown`].
    #[inline]
    pub fn with_fullsize_content_view(mut self, value: bool) -> Self {
        self.fullsize_content_view = Some(value);
        self
    }

    /// macOS: Set to `true` to allow the window to be moved by dragging the background.
    /// Enabling this feature can result in unexpected behavior with draggable UI widgets such as sliders.
    #[inline]
    pub fn with_movable_by_background(mut self, value: bool) -> Self {
        self.movable_by_window_background = Some(value);
        self
    }

    /// macOS: Set to `false` to hide the window title.
    #[inline]
    pub fn with_title_shown(mut self, title_shown: bool) -> Self {
        self.title_shown = Some(title_shown);
        self
    }

    /// macOS: Set to `false` to hide the titlebar button (close, minimize, maximize)
    #[inline]
    pub fn with_titlebar_buttons_shown(mut self, titlebar_buttons_shown: bool) -> Self {
        self.titlebar_buttons_shown = Some(titlebar_buttons_shown);
        self
    }

    /// macOS: Set to `false` to make the titlebar transparent, allowing the content to appear behind it.
    #[inline]
    pub fn with_titlebar_shown(mut self, shown: bool) -> Self {
        self.titlebar_shown = Some(shown);
        self
    }

    /// macOS: Set to `false` to make the window render without a drop shadow.
    ///
    /// The default is `true`.
    ///
    /// Disabling this feature can solve ghosting issues experienced if using [`Self::with_transparent`].
    ///
    /// Look at winit for more details
    #[inline]
    pub fn with_has_shadow(mut self, has_shadow: bool) -> Self {
        self.has_shadow = Some(has_shadow);
        self
    }

    /// windows: Whether show or hide the window icon in the taskbar.
    #[inline]
    pub fn with_taskbar(mut self, show: bool) -> Self {
        self.taskbar = Some(show);
        self
    }

    /// Requests the window to be of specific dimensions.
    ///
    /// If this is not set, some platform-specific dimensions will be used.
    ///
    /// Should be bigger than 0
    /// Look at winit for more details
    #[inline]
    pub fn with_inner_size(mut self, size: impl Into<Vec2>) -> Self {
        self.inner_size = Some(size.into());
        self
    }

    /// Sets the minimum dimensions a window can have.
    ///
    /// If this is not set, the window will have no minimum dimensions (aside
    /// from reserved).
    ///
    /// Should be bigger than 0
    /// Look at winit for more details
    #[inline]
    pub fn with_min_inner_size(mut self, size: impl Into<Vec2>) -> Self {
        self.min_inner_size = Some(size.into());
        self
    }

    /// Sets the maximum dimensions a window can have.
    ///
    /// If this is not set, the window will have no maximum or will be set to
    /// the primary monitor's dimensions by the platform.
    ///
    /// Should be bigger than 0
    /// Look at winit for more details
    #[inline]
    pub fn with_max_inner_size(mut self, size: impl Into<Vec2>) -> Self {
        self.max_inner_size = Some(size.into());
        self
    }

    /// Sets whether clamp the window's size to monitor's size. The default is `true` on linux, otherwise it is `false`.
    ///
    /// Note: On some Linux systems, a window size larger than the monitor causes crashes
    #[inline]
    pub fn with_clamp_size_to_monitor_size(mut self, value: bool) -> Self {
        self.clamp_size_to_monitor_size = Some(value);
        self
    }

    /// Does not work on X11.
    #[inline]
    pub fn with_close_button(mut self, value: bool) -> Self {
        self.close_button = Some(value);
        self
    }

    /// Does not work on X11.
    #[inline]
    pub fn with_minimize_button(mut self, value: bool) -> Self {
        self.minimize_button = Some(value);
        self
    }

    /// Does not work on X11.
    #[inline]
    pub fn with_maximize_button(mut self, value: bool) -> Self {
        self.maximize_button = Some(value);
        self
    }

    /// On Windows: enable drag and drop support. Drag and drop can
    /// not be disabled on other platforms.
    ///
    /// See [winit's documentation][drag_and_drop] for information on why you
    /// might want to disable this on windows.
    ///
    /// [drag_and_drop]: https://docs.rs/winit/latest/x86_64-pc-windows-msvc/winit/platform/windows/trait.WindowAttributesExtWindows.html#tymethod.with_drag_and_drop
    #[inline]
    pub fn with_drag_and_drop(mut self, value: bool) -> Self {
        self.drag_and_drop = Some(value);
        self
    }

    /// The initial "outer" position of the window,
    /// i.e. where the top-left corner of the frame/chrome should be.
    ///
    /// **`eframe` notes**:
    ///
    /// - **iOS:** Sets the top left coordinates of the window in the screen space coordinate system.
    /// - **Web:** Sets the top-left coordinates relative to the viewport. Doesn't account for CSS
    ///   [`transform`].
    /// - **Android / Wayland:** Unsupported.
    ///
    /// [`transform`]: https://developer.mozilla.org/en-US/docs/Web/CSS/transform
    #[inline]
    pub fn with_position(mut self, pos: impl Into<Pos2>) -> Self {
        self.position = Some(pos.into());
        self
    }

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
    /// The `app_id` should match the `.desktop` file distributed with your program.
    ///
    /// For details about application ID conventions, see the
    /// [Desktop Entry Spec](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#desktop-file-id)
    ///
    /// [xdg-shell]: https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:set_app_id
    ///
    /// ### eframe
    /// On eframe, the `app_id` of the root window is also used to determine
    /// the storage location of persistence files.
    #[inline]
    pub fn with_app_id(mut self, app_id: impl Into<String>) -> Self {
        self.app_id = Some(app_id.into());
        self
    }

    /// Control if window is always-on-top, always-on-bottom, or neither.
    ///
    /// For platform compatibility see [`crate::viewport::WindowLevel`] documentation
    #[inline]
    pub fn with_window_level(mut self, level: WindowLevel) -> Self {
        self.window_level = Some(level);
        self
    }

    /// This window is always on top
    ///
    /// For platform compatibility see [`crate::viewport::WindowLevel`] documentation
    #[inline]
    pub fn with_always_on_top(self) -> Self {
        self.with_window_level(WindowLevel::AlwaysOnTop)
    }

    /// On desktop: mouse clicks pass through the window, used for non-interactable overlays.
    ///
    /// Generally you would use this in conjunction with [`Self::with_transparent`]
    /// and [`Self::with_always_on_top`].
    #[inline]
    pub fn with_mouse_passthrough(mut self, value: bool) -> Self {
        self.mouse_passthrough = Some(value);
        self
    }

    /// ### On X11
    /// This sets the window type.
    /// Maps directly to [`_NET_WM_WINDOW_TYPE`](https://specifications.freedesktop.org/wm/1.5/ar01s05.html#id-1.6.7).
    #[inline]
    pub fn with_window_type(mut self, value: X11WindowType) -> Self {
        self.window_type = Some(value);
        self
    }

    /// ### On X11
    /// This sets the override-redirect flag. When this is set to true the window type should be specified.
    /// Maps directly to [`Override-redirect windows`](https://specifications.freedesktop.org/wm/1.5/ar01s02.html#id-1.3.13).
    #[inline]
    pub fn with_override_redirect(mut self, value: bool) -> Self {
        self.override_redirect = Some(value);
        self
    }

    /// Update this `ViewportBuilder` with a delta,
    /// returning a list of commands and a bool indicating if the window needs to be recreated.
    #[must_use]
    pub fn patch(&mut self, new_vp_builder: Self) -> (Vec<ViewportCommand>, bool) {
        #![expect(clippy::useless_let_if_seq)] // False positive

        let Self {
            title: new_title,
            app_id: new_app_id,
            position: new_position,
            inner_size: new_inner_size,
            min_inner_size: new_min_inner_size,
            max_inner_size: new_max_inner_size,
            clamp_size_to_monitor_size: new_clamp_size_to_monitor_size,
            fullscreen: new_fullscreen,
            maximized: new_maximized,
            resizable: new_resizable,
            transparent: new_transparent,
            decorations: new_decorations,
            icon: new_icon,
            active: new_active,
            visible: new_visible,
            drag_and_drop: new_drag_and_drop,
            fullsize_content_view: new_fullsize_content_view,
            movable_by_window_background: new_movable_by_window_background,
            title_shown: new_title_shown,
            titlebar_buttons_shown: new_titlebar_buttons_shown,
            titlebar_shown: new_titlebar_shown,
            has_shadow: new_has_shadow,
            close_button: new_close_button,
            minimize_button: new_minimize_button,
            maximize_button: new_maximize_button,
            window_level: new_window_level,
            mouse_passthrough: new_mouse_passthrough,
            taskbar: new_taskbar,
            window_type: new_window_type,
            override_redirect: new_override_redirect,
        } = new_vp_builder;

        let mut commands = Vec::new();

        if let Some(new_title) = new_title
            && Some(&new_title) != self.title.as_ref()
        {
            self.title = Some(new_title.clone());
            commands.push(ViewportCommand::Title(new_title));
        }

        if let Some(new_position) = new_position
            && Some(new_position) != self.position
        {
            self.position = Some(new_position);
            commands.push(ViewportCommand::OuterPosition(new_position));
        }

        if let Some(new_inner_size) = new_inner_size
            && Some(new_inner_size) != self.inner_size
        {
            self.inner_size = Some(new_inner_size);
            commands.push(ViewportCommand::InnerSize(new_inner_size));
        }

        if let Some(new_min_inner_size) = new_min_inner_size
            && Some(new_min_inner_size) != self.min_inner_size
        {
            self.min_inner_size = Some(new_min_inner_size);
            commands.push(ViewportCommand::MinInnerSize(new_min_inner_size));
        }

        if let Some(new_max_inner_size) = new_max_inner_size
            && Some(new_max_inner_size) != self.max_inner_size
        {
            self.max_inner_size = Some(new_max_inner_size);
            commands.push(ViewportCommand::MaxInnerSize(new_max_inner_size));
        }

        if let Some(new_fullscreen) = new_fullscreen
            && Some(new_fullscreen) != self.fullscreen
        {
            self.fullscreen = Some(new_fullscreen);
            commands.push(ViewportCommand::Fullscreen(new_fullscreen));
        }

        if let Some(new_maximized) = new_maximized
            && Some(new_maximized) != self.maximized
        {
            self.maximized = Some(new_maximized);
            commands.push(ViewportCommand::Maximized(new_maximized));
        }

        if let Some(new_resizable) = new_resizable
            && Some(new_resizable) != self.resizable
        {
            self.resizable = Some(new_resizable);
            commands.push(ViewportCommand::Resizable(new_resizable));
        }

        if let Some(new_transparent) = new_transparent
            && Some(new_transparent) != self.transparent
        {
            self.transparent = Some(new_transparent);
            commands.push(ViewportCommand::Transparent(new_transparent));
        }

        if let Some(new_decorations) = new_decorations
            && Some(new_decorations) != self.decorations
        {
            self.decorations = Some(new_decorations);
            commands.push(ViewportCommand::Decorations(new_decorations));
        }

        if let Some(new_icon) = new_icon {
            let is_new = match &self.icon {
                Some(existing) => !Arc::ptr_eq(&new_icon, existing),
                None => true,
            };

            if is_new {
                commands.push(ViewportCommand::Icon(Some(Arc::clone(&new_icon))));
                self.icon = Some(new_icon);
            }
        }

        if let Some(new_visible) = new_visible
            && Some(new_visible) != self.visible
        {
            self.visible = Some(new_visible);
            commands.push(ViewportCommand::Visible(new_visible));
        }

        if let Some(new_mouse_passthrough) = new_mouse_passthrough
            && Some(new_mouse_passthrough) != self.mouse_passthrough
        {
            self.mouse_passthrough = Some(new_mouse_passthrough);
            commands.push(ViewportCommand::MousePassthrough(new_mouse_passthrough));
        }

        if let Some(new_window_level) = new_window_level
            && Some(new_window_level) != self.window_level
        {
            self.window_level = Some(new_window_level);
            commands.push(ViewportCommand::WindowLevel(new_window_level));
        }

        // --------------------------------------------------------------
        // Things we don't have commands for require a full window recreation.
        // The reason we don't have commands for them is that `winit` doesn't support
        // changing them without recreating the window.

        let mut recreate_window = false;

        if new_clamp_size_to_monitor_size.is_some()
            && self.clamp_size_to_monitor_size != new_clamp_size_to_monitor_size
        {
            self.clamp_size_to_monitor_size = new_clamp_size_to_monitor_size;
            recreate_window = true;
        }

        if new_active.is_some() && self.active != new_active {
            self.active = new_active;
            recreate_window = true;
        }

        if new_app_id.is_some() && self.app_id != new_app_id {
            self.app_id = new_app_id;
            recreate_window = true;
        }

        if new_close_button.is_some() && self.close_button != new_close_button {
            self.close_button = new_close_button;
            recreate_window = true;
        }

        if new_minimize_button.is_some() && self.minimize_button != new_minimize_button {
            self.minimize_button = new_minimize_button;
            recreate_window = true;
        }

        if new_maximize_button.is_some() && self.maximize_button != new_maximize_button {
            self.maximize_button = new_maximize_button;
            recreate_window = true;
        }

        if new_title_shown.is_some() && self.title_shown != new_title_shown {
            self.title_shown = new_title_shown;
            recreate_window = true;
        }

        if new_titlebar_buttons_shown.is_some()
            && self.titlebar_buttons_shown != new_titlebar_buttons_shown
        {
            self.titlebar_buttons_shown = new_titlebar_buttons_shown;
            recreate_window = true;
        }

        if new_titlebar_shown.is_some() && self.titlebar_shown != new_titlebar_shown {
            self.titlebar_shown = new_titlebar_shown;
            recreate_window = true;
        }

        if new_has_shadow.is_some() && self.has_shadow != new_has_shadow {
            self.has_shadow = new_has_shadow;
            recreate_window = true;
        }

        if new_taskbar.is_some() && self.taskbar != new_taskbar {
            self.taskbar = new_taskbar;
            recreate_window = true;
        }

        if new_fullsize_content_view.is_some()
            && self.fullsize_content_view != new_fullsize_content_view
        {
            self.fullsize_content_view = new_fullsize_content_view;
            recreate_window = true;
        }

        if new_movable_by_window_background.is_some()
            && self.movable_by_window_background != new_movable_by_window_background
        {
            self.movable_by_window_background = new_movable_by_window_background;
            recreate_window = true;
        }

        if new_drag_and_drop.is_some() && self.drag_and_drop != new_drag_and_drop {
            self.drag_and_drop = new_drag_and_drop;
            recreate_window = true;
        }

        if new_window_type.is_some() && self.window_type != new_window_type {
            self.window_type = new_window_type;
            recreate_window = true;
        }

        if new_override_redirect.is_some() && self.override_redirect != new_override_redirect {
            self.override_redirect = new_override_redirect;
            recreate_window = true;
        }

        (commands, recreate_window)
    }
}

/// For winit platform compatibility, see [`winit::WindowLevel` documentation](https://docs.rs/winit/latest/winit/window/enum.WindowLevel.html#platform-specific)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum WindowLevel {
    #[default]
    Normal,
    AlwaysOnBottom,
    AlwaysOnTop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum X11WindowType {
    /// This is a normal, top-level window.
    #[default]
    Normal,

    /// A desktop feature. This can include a single window containing desktop icons with the same dimensions as the
    /// screen, allowing the desktop environment to have full control of the desktop, without the need for proxying
    /// root window clicks.
    Desktop,

    /// A dock or panel feature. Typically a Window Manager would keep such windows on top of all other windows.
    Dock,

    /// Toolbar windows. "Torn off" from the main application.
    Toolbar,

    /// Pinnable menu windows. "Torn off" from the main application.
    Menu,

    /// A small persistent utility window, such as a palette or toolbox.
    Utility,

    /// The window is a splash screen displayed as an application is starting up.
    Splash,

    /// This is a dialog window.
    Dialog,

    /// A dropdown menu that usually appears when the user clicks on an item in a menu bar.
    /// This property is typically used on override-redirect windows.
    DropdownMenu,

    /// A popup menu that usually appears when the user right clicks on an object.
    /// This property is typically used on override-redirect windows.
    PopupMenu,

    /// A tooltip window. Usually used to show additional information when hovering over an object with the cursor.
    /// This property is typically used on override-redirect windows.
    Tooltip,

    /// The window is a notification.
    /// This property is typically used on override-redirect windows.
    Notification,

    /// This should be used on the windows that are popped up by combo boxes.
    /// This property is typically used on override-redirect windows.
    Combo,

    /// This indicates the window is being dragged.
    /// This property is typically used on override-redirect windows.
    Dnd,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum IMEPurpose {
    #[default]
    Normal,
    Password,
    Terminal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum SystemTheme {
    #[default]
    SystemDefault,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum CursorGrab {
    #[default]
    None,
    Confined,
    Locked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ResizeDirection {
    North,
    South,
    East,
    West,
    NorthEast,
    SouthEast,
    NorthWest,
    SouthWest,
}

/// An output [viewport](crate::viewport)-command from egui to the backend, e.g. to change the window title or size.
///
/// You can send a [`ViewportCommand`] to the viewport with [`Context::send_viewport_cmd`].
///
/// See [`crate::viewport`] for how to build new viewports (native windows).
///
/// All coordinates are in logical points.
///
/// [`ViewportCommand`] is essentially a way to diff [`ViewportBuilder`]s.
///
/// Only commands specific to a viewport are part of [`ViewportCommand`].
/// Other commands should be put in [`crate::OutputCommand`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ViewportCommand {
    /// Request this viewport to be closed.
    ///
    /// For the root viewport, this usually results in the application shutting down.
    /// For other viewports, the [`crate::ViewportInfo::close_requested`] flag will be set.
    Close,

    /// Cancel the closing that was signaled by [`crate::ViewportInfo::close_requested`].
    CancelClose,

    /// Set the window title.
    Title(String),

    /// Turn the window transparent or not.
    Transparent(bool),

    /// Set the visibility of the window.
    Visible(bool),

    /// Moves the window with the left mouse button until the button is released.
    ///
    /// There's no guarantee that this will work unless the left mouse button was pressed
    /// immediately before this function is called.
    StartDrag,

    /// Set the outer position of the viewport, i.e. moves the window.
    OuterPosition(Pos2),

    /// Should be bigger than 0
    InnerSize(Vec2),

    /// Should be bigger than 0
    MinInnerSize(Vec2),

    /// Should be bigger than 0
    MaxInnerSize(Vec2),

    /// Should be bigger than 0
    ResizeIncrements(Option<Vec2>),

    /// Begin resizing the viewport with the left mouse button until the button is released.
    ///
    /// There's no guarantee that this will work unless the left mouse button was pressed
    /// immediately before this function is called.
    BeginResize(ResizeDirection),

    /// Can the window be resized?
    Resizable(bool),

    /// Set which window buttons are enabled
    EnableButtons {
        close: bool,
        minimized: bool,
        maximize: bool,
    },
    Minimized(bool),

    /// Maximize or unmaximize window.
    Maximized(bool),

    /// Turn borderless fullscreen on/off.
    Fullscreen(bool),

    /// Show window decorations, i.e. the chrome around the content
    /// with the title bar, close buttons, resize handles, etc.
    Decorations(bool),

    /// Set window to be always-on-top, always-on-bottom, or neither.
    WindowLevel(WindowLevel),

    /// The window icon.
    Icon(Option<Arc<IconData>>),

    /// Set the IME cursor editing area.
    IMERect(crate::Rect),
    IMEAllowed(bool),
    IMEPurpose(IMEPurpose),

    /// Bring the window into focus (native only).
    ///
    /// This command puts the window on top of other applications and takes input focus away from them,
    /// which, if unexpected, will disturb the user.
    ///
    /// Has no effect on Wayland, or if the window is minimized or invisible.
    Focus,

    /// If the window is unfocused, attract the user's attention (native only).
    ///
    /// Typically, this means that the window will flash on the taskbar, or bounce, until it is interacted with.
    ///
    /// When the window comes into focus, or if `None` is passed, the attention request will be automatically reset.
    ///
    /// See [winit's documentation][user_attention_details] for platform-specific effect details.
    ///
    /// [user_attention_details]: https://docs.rs/winit/latest/winit/window/enum.UserAttentionType.html
    RequestUserAttention(crate::UserAttentionType),

    SetTheme(SystemTheme),

    ContentProtected(bool),

    /// Will probably not work as expected!
    CursorPosition(Pos2),

    CursorGrab(CursorGrab),

    CursorVisible(bool),

    /// Enable mouse pass-through: mouse clicks pass through the window, used for non-interactable overlays.
    MousePassthrough(bool),

    /// Take a screenshot of the next frame after this.
    ///
    /// The results are returned in [`crate::Event::Screenshot`].
    Screenshot(crate::UserData),

    /// Request cut of the current selection
    ///
    /// This is equivalent to the system keyboard shortcut for cut (e.g. CTRL + X).
    RequestCut,

    /// Request a copy of the current selection.
    ///
    /// This is equivalent to the system keyboard shortcut for copy (e.g. CTRL + C).
    RequestCopy,

    /// Request a paste from the clipboard to the current focused `TextEdit` if any.
    ///
    /// This is equivalent to the system keyboard shortcut for paste (e.g. CTRL + V).
    RequestPaste,
}

impl ViewportCommand {
    /// Construct a command to center the viewport on the monitor, if possible.
    pub fn center_on_screen(ctx: &crate::Context) -> Option<Self> {
        ctx.input(|i| {
            let outer_rect = i.viewport().outer_rect?;
            let size = outer_rect.size();
            let monitor_size = i.viewport().monitor_size?;
            if 1.0 < monitor_size.x && 1.0 < monitor_size.y {
                let x = (monitor_size.x - size.x) / 2.0;
                let y = (monitor_size.y - size.y) / 2.0;
                Some(Self::OuterPosition([x, y].into()))
            } else {
                None
            }
        })
    }

    /// This command requires the parent viewport to repaint.
    pub fn requires_parent_repaint(&self) -> bool {
        self == &Self::Close
    }
}

// ----------------------------------------------------------------------------

/// Describes a viewport, i.e. a native window.
///
/// This is returned by [`crate::Context::run`] on each frame, and should be applied
/// by the integration.
#[derive(Clone)]
pub struct ViewportOutput {
    /// Id of our parent viewport.
    pub parent: ViewportId,

    /// What type of viewport are we?
    ///
    /// This will never be [`ViewportClass::EmbeddedWindow`],
    /// since those don't result in real viewports.
    pub class: ViewportClass,

    /// The window attributes such as title, position, size, etc.
    ///
    /// Use this when first constructing the native window.
    /// Also check for changes in it using [`ViewportBuilder::patch`],
    /// and apply them as needed.
    pub builder: ViewportBuilder,

    /// The user-code that shows the GUI, used for deferred viewports.
    ///
    /// `None` for immediate viewports and the ROOT viewport.
    pub viewport_ui_cb: Option<Arc<DeferredViewportUiCallback>>,

    /// Commands to change the viewport, e.g. window title and size.
    pub commands: Vec<ViewportCommand>,

    /// Schedule a repaint of this viewport after this delay.
    ///
    /// It is preferable to instead install a [`Context::set_request_repaint_callback`],
    /// but if you haven't, you can use this instead.
    ///
    /// If the duration is zero, schedule a repaint immediately.
    pub repaint_delay: std::time::Duration,
}

impl ViewportOutput {
    /// Add on new output.
    pub fn append(&mut self, newer: Self) {
        let Self {
            parent,
            class,
            builder,
            viewport_ui_cb,
            mut commands,
            repaint_delay,
        } = newer;

        self.parent = parent;
        self.class = class;
        let _ = self.builder.patch(builder); // we ignore the returned command, because `self.builder` will be the basis of a new patch
        self.viewport_ui_cb = viewport_ui_cb;
        self.commands.append(&mut commands);
        self.repaint_delay = self.repaint_delay.min(repaint_delay);
    }
}

/// Viewport for immediate rendering.
pub struct ImmediateViewport<'a> {
    /// Id of us and our parent.
    pub ids: ViewportIdPair,

    pub builder: ViewportBuilder,

    /// The user-code that shows the GUI.
    pub viewport_ui_cb: Box<dyn FnMut(&mut Ui) + 'a>,
}
