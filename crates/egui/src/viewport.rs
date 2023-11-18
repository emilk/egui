//! egui supports multiple viewports, corresponding to multiple native windows.
//!
//! Not all egui backends support multiple viewports, but `eframe` native does
//! (but not on web).
//!
//! You can spawn a new viewport using [`Context::show_viewport`] and [`Context::show_viewport_immediate`].
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
//! These are created with [`Context::show_viewport`].
//! Deferred viewports take a closure that is called by the integration at a later time, perhaps multiple times.
//! Deferred viewports are repainted independenantly of the parent viewport.
//! This means communication with them need to done via channels, or `Arc/Mutex`.
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
//! These are not real, independenant viewports, but is a fallback mode for when the integration does not support real viewports. In your callback is called with [`ViewportClass::Embedded`] it means you need to create an [`crate::Window`] to wrap your ui in, which will then be embedded in the parent viewport, unable to escape it.
//!
//!
//! ## Using the viewports
//! Only one viewport is active at any one time, identified with [`Context::viewport_id`].
//! You can modify the current (change the title, resize the window, etc) by sending
//! a [`ViewportCommand`] to it using [`Context::send_viewport_cmd`].
//! You can interact with other viewports using [`Context::send_viewport_cmd_to`].
//!
//! There is an example in <https://github.com/emilk/egui/tree/master/examples/multiple_viewports/src/main.rs>.
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

use epaint::{ColorImage, Pos2, Vec2};

use crate::{Context, Id};

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
    /// Create these with [`crate::Context::show_viewport`].
    Deferred,

    /// A viewport run inside the parent viewport.
    ///
    /// This is the easier type of viewport to use, but it is less performant
    /// at it requires both parent and child to repaint if any one of them needs repainting,
    /// which efficvely produce double work for two viewports, and triple work for three viewports, etc.
    ///
    /// Create these with [`crate::Context::show_viewport_immediate`].
    Immediate,

    /// The fallback, when the egui integration doesn't support viewports,
    /// or [`crate::Context::embed_viewports`] is set to `true`.
    Embedded,
}

// ----------------------------------------------------------------------------

/// A unique identifier of a viewport.
///
/// This is returned by [`Context::viewport_id`] and [`Context::parent_viewport_id`].
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ViewportId(pub Id);

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
pub type DeferredViewportUiCallback = dyn Fn(&Context) + Sync + Send;

/// Render the given viewport, calling the given ui callback.
pub type ImmediateViewportRendererCallback = dyn for<'a> Fn(&Context, ImmediateViewport<'a>);

/// Control the building of a new egui viewport (i.e. native window).
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
#[allow(clippy::option_option)]
pub struct ViewportBuilder {
    /// The title of the vieweport.
    /// `eframe` will use this as the title of the native window.
    pub title: Option<String>,

    /// This is wayland only. See [`Self::with_name`].
    pub name: Option<(String, String)>,

    pub position: Option<Pos2>,
    pub inner_size: Option<Vec2>,
    pub min_inner_size: Option<Vec2>,
    pub max_inner_size: Option<Vec2>,

    pub fullscreen: Option<bool>,
    pub maximized: Option<bool>,
    pub resizable: Option<bool>,
    pub transparent: Option<bool>,
    pub decorations: Option<bool>,
    pub icon: Option<Arc<ColorImage>>,
    pub active: Option<bool>,
    pub visible: Option<bool>,
    pub title_hidden: Option<bool>,
    pub titlebar_transparent: Option<bool>,
    pub fullsize_content_view: Option<bool>,
    pub drag_and_drop: Option<bool>,

    pub close_button: Option<bool>,
    pub minimize_button: Option<bool>,
    pub maximize_button: Option<bool>,

    pub hittest: Option<bool>,
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
    /// If this is `true`, writing colors with alpha values different than
    /// `1.0` will produce a transparent window. On some platforms this
    /// is more of a hint for the system and you'd still have the alpha
    /// buffer.
    ///
    /// The default is `false`.
    /// If this is not working is because the graphic context dozen't support transparency,
    /// you will need to set the transparency in the eframe!
    #[inline]
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = Some(transparent);
        self
    }

    /// The icon needs to be wrapped in Arc because will be cloned every frame
    #[inline]
    pub fn with_window_icon(mut self, icon: impl Into<Arc<ColorImage>>) -> Self {
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

    /// Hides the window title.
    ///
    /// Mac Os only.
    #[inline]
    pub fn with_title_hidden(mut self, title_hidden: bool) -> Self {
        self.title_hidden = Some(title_hidden);
        self
    }

    /// Makes the titlebar transparent and allows the content to appear behind it.
    ///
    /// Mac Os only.
    #[inline]
    pub fn with_titlebar_transparent(mut self, value: bool) -> Self {
        self.titlebar_transparent = Some(value);
        self
    }

    /// Makes the window content appear behind the titlebar.
    ///
    /// Mac Os only.
    #[inline]
    pub fn with_fullsize_content_view(mut self, value: bool) -> Self {
        self.fullsize_content_view = Some(value);
        self
    }

    /// Requests the window to be of specific dimensions.
    ///
    /// If this is not set, some platform-specific dimensions will be used.
    ///
    /// Should be bigger then 0
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
    /// Should be bigger then 0
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
    /// Should be bigger then 0
    /// Look at winit for more details
    #[inline]
    pub fn with_max_inner_size(mut self, size: impl Into<Vec2>) -> Self {
        self.max_inner_size = Some(size.into());
        self
    }

    /// X11 not working!
    #[inline]
    pub fn with_close_button(mut self, value: bool) -> Self {
        self.close_button = Some(value);
        self
    }

    /// X11 not working!
    #[inline]
    pub fn with_minimize_button(mut self, value: bool) -> Self {
        self.minimize_button = Some(value);
        self
    }

    /// X11 not working!
    #[inline]
    pub fn with_maximize_button(mut self, value: bool) -> Self {
        self.maximize_button = Some(value);
        self
    }

    /// This currently only work on windows to be disabled!
    #[inline]
    pub fn with_drag_and_drop(mut self, value: bool) -> Self {
        self.drag_and_drop = Some(value);
        self
    }

    /// This will probably not work as expected!
    #[inline]
    pub fn with_position(mut self, pos: impl Into<Pos2>) -> Self {
        self.position = Some(pos.into());
        self
    }

    /// This is wayland only!
    /// Build window with the given name.
    ///
    /// The `general` name sets an application ID, which should match the `.desktop`
    /// file distributed with your program. The `instance` is a `no-op`.
    ///
    /// For details about application ID conventions, see the
    /// [Desktop Entry Spec](https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#desktop-file-id)
    #[inline]
    pub fn with_name(mut self, id: impl Into<String>, instance: impl Into<String>) -> Self {
        self.name = Some((id.into(), instance.into()));
        self
    }

    /// Is not implemented for winit
    /// You should use `ViewportCommand::CursorHitTest` if you want to set this!
    #[deprecated]
    #[inline]
    pub fn with_hittest(mut self, value: bool) -> Self {
        self.hittest = Some(value);
        self
    }

    /// Update this `ViewportBuilder` with a delta,
    /// returning a list of commands and a bool intdicating if the window needs to be recreated.
    pub fn patch(&mut self, new: &ViewportBuilder) -> (Vec<ViewportCommand>, bool) {
        let mut commands = Vec::new();

        if let Some(new_title) = &new.title {
            if Some(new_title) != self.title.as_ref() {
                self.title = Some(new_title.clone());
                commands.push(ViewportCommand::Title(new_title.clone()));
            }
        }

        if let Some(new_position) = new.position {
            if Some(new_position) != self.position {
                self.position = Some(new_position);
                commands.push(ViewportCommand::OuterPosition(new_position));
            }
        }

        if let Some(new_inner_size) = new.inner_size {
            if Some(new_inner_size) != self.inner_size {
                self.inner_size = Some(new_inner_size);
                commands.push(ViewportCommand::InnerSize(new_inner_size));
            }
        }

        if let Some(new_min_inner_size) = new.min_inner_size {
            if Some(new_min_inner_size) != self.min_inner_size {
                self.min_inner_size = Some(new_min_inner_size);
                commands.push(ViewportCommand::MinInnerSize(new_min_inner_size));
            }
        }

        if let Some(new_max_inner_size) = new.max_inner_size {
            if Some(new_max_inner_size) != self.max_inner_size {
                self.max_inner_size = Some(new_max_inner_size);
                commands.push(ViewportCommand::MaxInnerSize(new_max_inner_size));
            }
        }

        if let Some(new_fullscreen) = new.fullscreen {
            if Some(new_fullscreen) != self.fullscreen {
                self.fullscreen = Some(new_fullscreen);
                commands.push(ViewportCommand::Fullscreen(new_fullscreen));
            }
        }

        if let Some(new_maximized) = new.maximized {
            if Some(new_maximized) != self.maximized {
                self.maximized = Some(new_maximized);
                commands.push(ViewportCommand::Maximized(new_maximized));
            }
        }

        if let Some(new_resizable) = new.resizable {
            if Some(new_resizable) != self.resizable {
                self.resizable = Some(new_resizable);
                commands.push(ViewportCommand::Resizable(new_resizable));
            }
        }

        if let Some(new_transparent) = new.transparent {
            if Some(new_transparent) != self.transparent {
                self.transparent = Some(new_transparent);
                commands.push(ViewportCommand::Transparent(new_transparent));
            }
        }

        if let Some(new_decorations) = new.decorations {
            if Some(new_decorations) != self.decorations {
                self.decorations = Some(new_decorations);
                commands.push(ViewportCommand::Decorations(new_decorations));
            }
        }

        if let Some(new_icon) = &new.icon {
            let is_new = match &self.icon {
                Some(existing) => !Arc::ptr_eq(new_icon, existing),
                None => true,
            };

            if is_new {
                commands.push(ViewportCommand::WindowIcon(Some(new_icon.clone())));
                self.icon = Some(new_icon.clone());
            }
        }

        if let Some(new_visible) = new.visible {
            if Some(new_visible) != self.active {
                self.visible = Some(new_visible);
                commands.push(ViewportCommand::Visible(new_visible));
            }
        }

        if let Some(new_hittest) = new.hittest {
            if Some(new_hittest) != self.hittest {
                self.hittest = Some(new_hittest);
                commands.push(ViewportCommand::CursorHitTest(new_hittest));
            }
        }

        // TODO: Implement compare for windows buttons

        let mut recreate_window = false;

        if let Some(new_active) = new.active {
            if Some(new_active) != self.active {
                self.active = Some(new_active);
                recreate_window = true;
            }
        }

        if let Some(new_close_button) = new.close_button {
            if Some(new_close_button) != self.close_button {
                self.close_button = Some(new_close_button);
                recreate_window = true;
            }
        }

        if let Some(new_minimize_button) = new.minimize_button {
            if Some(new_minimize_button) != self.minimize_button {
                self.minimize_button = Some(new_minimize_button);
                recreate_window = true;
            }
        }

        if let Some(new_maximized_button) = new.maximize_button {
            if Some(new_maximized_button) != self.maximize_button {
                self.maximize_button = Some(new_maximized_button);
                recreate_window = true;
            }
        }

        if let Some(new_title_hidden) = new.title_hidden {
            if Some(new_title_hidden) != self.title_hidden {
                self.title_hidden = Some(new_title_hidden);
                recreate_window = true;
            }
        }

        if let Some(new_titlebar_transparent) = new.titlebar_transparent {
            if Some(new_titlebar_transparent) != self.titlebar_transparent {
                self.titlebar_transparent = Some(new_titlebar_transparent);
                recreate_window = true;
            }
        }

        if let Some(new_fullsize_content_view) = new.fullsize_content_view {
            if Some(new_fullsize_content_view) != self.fullsize_content_view {
                self.fullsize_content_view = Some(new_fullsize_content_view);
                recreate_window = true;
            }
        }

        (commands, recreate_window)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum WindowLevel {
    Normal,
    AlwaysOnBottom,
    AlwaysOnTop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum IMEPurpose {
    Normal,
    Password,
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum SystemTheme {
    Light,
    Dark,
    SystemDefault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum CursorGrab {
    None,
    Confined,
    Locked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ResizeDirection {
    North,
    South,
    West,
    NorthEast,
    SouthEast,
    NorthWest,
    SouthWest,
}

/// You can send a [`ViewportCommand`] to the viewport with [`Context::send_viewport_cmd`].
///
/// All coordinates are in logical points.
///
/// This is essentially a way to diff [`ViewportBuilder`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ViewportCommand {
    /// Request this viewport to be closed.
    ///
    /// For the root viewport, this usually results in the application shutting down.
    /// For other viewports, the [`crate::ViewportInfo::close_requested`] flag will be set.
    Close,

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

    /// Should be bigger then 0
    InnerSize(Vec2),

    /// Should be bigger then 0
    MinInnerSize(Vec2),

    /// Should be bigger then 0
    MaxInnerSize(Vec2),

    /// Should be bigger then 0
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

    /// The the window icon.
    WindowIcon(Option<Arc<ColorImage>>),

    IMEPosition(Pos2),
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

    CursorHitTest(bool),

    /// Take a screenshot.
    ///
    /// The results are returned in `crate::Event::Screenshot`.
    Screenshot,
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
}

/// Describes a viewport, i.e. a native window.
#[derive(Clone)]
pub struct ViewportOutput {
    /// Id of our parent viewport.
    pub parent: ViewportId,

    /// What type of viewport are we?
    ///
    /// This will never be [`ViewportClass::Embedded`],
    /// since those don't result in real viewports.
    pub class: ViewportClass,

    /// The window attrbiutes such as title, position, size, etc.
    pub builder: ViewportBuilder,

    /// The user-code that shows the GUI, used for deferred viewports.
    ///
    /// `None` for immediate viewports and the ROOT viewport.
    pub viewport_ui_cb: Option<Arc<DeferredViewportUiCallback>>,

    /// Commands to change the viewport, e.g. window title and size.
    pub commands: Vec<ViewportCommand>,

    /// Schedulare a repaint of this viewport after this delay.
    ///
    /// It is preferably to instead install a [`Context::set_request_repaint_callback`],
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
        self.builder.patch(&builder);
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
    pub viewport_ui_cb: Box<dyn FnOnce(&Context) + 'a>,
}
