//! egui supports multiple viewports, corresponding to multiple native windows.
//!
//! Viewports come in two flavors: "deferred" (the default) and "immediate".
//!
//! * Deferred viewports have callbacks that are called multiple
//!   times as the viewport receives events, or need repaitning.
//! * Immediate viewports are executed immediately with an [`FnOnce`] callback,
//!   locking the parent and child viewports together so that they both must update at the same time.

use std::sync::Arc;

use epaint::{ColorImage, Pos2, Vec2};

use crate::{Context, Id};

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
}

/// The user-code that shows the ui in the viewport, used for deferred viewports.
pub type ViewportUiCallback = dyn Fn(&Context) + Sync + Send;

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
#[derive(PartialEq, Eq, Clone)]
#[allow(clippy::option_option)]
pub struct ViewportBuilder {
    pub id: ViewportId,

    /// The title of the vieweport.
    /// `eframe` will use this as the title of the native window.
    pub title: Option<String>,

    /// This is wayland only. See [`Self::with_name`].
    pub name: Option<(String, String)>,

    pub position: Option<Option<Pos2>>,
    pub inner_size: Option<Option<Vec2>>,
    pub fullscreen: Option<bool>,
    pub maximized: Option<bool>,
    pub resizable: Option<bool>,
    pub transparent: Option<bool>,
    pub decorations: Option<bool>,
    pub icon: Option<Option<Arc<ColorImage>>>,
    pub active: Option<bool>,
    pub visible: Option<bool>,
    pub title_hidden: Option<bool>,
    pub titlebar_transparent: Option<bool>,
    pub fullsize_content_view: Option<bool>,
    pub min_inner_size: Option<Option<Vec2>>,
    pub max_inner_size: Option<Option<Vec2>>,
    pub drag_and_drop: Option<bool>,

    pub close_button: Option<bool>,
    pub minimize_button: Option<bool>,
    pub maximize_button: Option<bool>,

    pub hittest: Option<bool>,
}

impl ViewportBuilder {
    /// Default settings for a new child viewport.
    ///
    /// The given id must be unique for each viewport.
    pub fn new(id: ViewportId) -> Self {
        Self {
            id,
            title: None,
            name: None,
            position: None,
            inner_size: Some(Some(Vec2::new(300.0, 200.0))),
            fullscreen: None,
            maximized: None,
            resizable: Some(true),
            transparent: Some(true),
            decorations: Some(true),
            icon: None,
            active: Some(true),
            visible: Some(true),
            title_hidden: None,
            titlebar_transparent: None,
            fullsize_content_view: None,
            min_inner_size: None,
            max_inner_size: None,
            drag_and_drop: Some(true),
            close_button: Some(false), // We disable the close button by default because we haven't implemented closing of child viewports yet
            minimize_button: Some(true),
            maximize_button: Some(true),
            hittest: Some(true),
        }
    }

    /// Empty settings for everything.
    ///
    /// If used the first frame, backend-specific defaults will be used.
    /// When used on subsequent frames, the current settings will be kept.
    ///
    /// The given id must be unique for each viewport.
    pub fn empty(id: ViewportId) -> Self {
        Self {
            id,
            title: None,
            name: None,
            position: None,
            inner_size: None,
            fullscreen: None,
            maximized: None,
            resizable: None,
            transparent: None,
            decorations: None,
            icon: None,
            active: None,
            visible: None,
            title_hidden: None,
            titlebar_transparent: None,
            fullsize_content_view: None,
            min_inner_size: None,
            max_inner_size: None,
            drag_and_drop: None,
            close_button: None,
            minimize_button: None,
            maximize_button: None,
            hittest: None,
        }
    }

    /// Sets the initial title of the window in the title bar.
    ///
    /// Look at winit for more details
    pub fn with_title(&mut self, title: impl Into<String>) -> &mut Self {
        self.title = Some(title.into());
        self
    }

    /// Sets whether the window should have a border, a title bar, etc.
    ///
    /// The default is `true`.
    ///
    /// Look at winit for more details
    pub fn with_decorations(&mut self, decorations: bool) -> &mut Self {
        self.decorations = Some(decorations);
        self
    }

    /// Sets whether the window should be put into fullscreen upon creation.
    ///
    /// The default is `None`.
    ///
    /// Look at winit for more details
    /// This will use borderless
    pub fn with_fullscreen(&mut self, fullscreen: bool) -> &mut Self {
        self.fullscreen = Some(fullscreen);
        self
    }

    /// Request that the window is maximized upon creation.
    ///
    /// The default is `false`.
    ///
    /// Look at winit for more details
    pub fn with_maximized(&mut self, maximized: bool) -> &mut Self {
        self.maximized = Some(maximized);
        self
    }

    /// Sets whether the window is resizable or not.
    ///
    /// The default is `true`.
    ///
    /// Look at winit for more details
    pub fn with_resizable(&mut self, resizable: bool) -> &mut Self {
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
    pub fn with_transparent(&mut self, transparent: bool) -> &mut Self {
        self.transparent = Some(transparent);
        self
    }

    /// The icon needs to be wrapped in Arc because will be cloned every frame
    pub fn with_window_icon(&mut self, icon: Option<Arc<ColorImage>>) -> &mut Self {
        self.icon = Some(icon);
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
    pub fn with_active(&mut self, active: bool) -> &mut Self {
        self.active = Some(active);
        self
    }

    /// Sets whether the window will be initially visible or hidden.
    ///
    /// The default is to show the window.
    ///
    /// Look at winit for more details
    pub fn with_visible(&mut self, visible: bool) -> &mut Self {
        self.visible = Some(visible);
        self
    }

    /// Mac Os only
    /// Hides the window title.
    pub fn with_title_hidden(&mut self, title_hidden: bool) -> &mut Self {
        self.title_hidden = Some(title_hidden);
        self
    }

    /// Mac Os only
    /// Makes the titlebar transparent and allows the content to appear behind it.
    pub fn with_titlebar_transparent(&mut self, value: bool) -> &mut Self {
        self.titlebar_transparent = Some(value);
        self
    }

    /// Mac Os only
    /// Makes the window content appear behind the titlebar.
    pub fn with_fullsize_content_view(&mut self, value: bool) -> &mut Self {
        self.fullsize_content_view = Some(value);
        self
    }

    /// Requests the window to be of specific dimensions.
    ///
    /// If this is not set, some platform-specific dimensions will be used.
    ///
    /// Should be bigger then 0
    /// Look at winit for more details
    pub fn with_inner_size(&mut self, value: Option<Vec2>) -> &mut Self {
        self.inner_size = Some(value);
        self
    }

    /// Sets the minimum dimensions a window can have.
    ///
    /// If this is not set, the window will have no minimum dimensions (aside
    /// from reserved).
    ///
    /// Should be bigger then 0
    /// Look at winit for more details
    pub fn with_min_inner_size(&mut self, value: Option<Vec2>) -> &mut Self {
        self.min_inner_size = Some(value);
        self
    }

    /// Sets the maximum dimensions a window can have.
    ///
    /// If this is not set, the window will have no maximum or will be set to
    /// the primary monitor's dimensions by the platform.
    ///
    /// Should be bigger then 0
    /// Look at winit for more details
    pub fn with_max_inner_size(&mut self, value: Option<Vec2>) -> &mut Self {
        self.max_inner_size = Some(value);
        self
    }

    /// X11 not working!
    pub fn with_close_button(&mut self, value: bool) -> &mut Self {
        self.close_button = Some(value);
        self
    }

    /// X11 not working!
    pub fn with_minimize_button(&mut self, value: bool) -> &mut Self {
        self.minimize_button = Some(value);
        self
    }

    /// X11 not working!
    pub fn with_maximize_button(&mut self, value: bool) -> &mut Self {
        self.maximize_button = Some(value);
        self
    }

    /// This currently only work on windows to be disabled!
    pub fn with_drag_and_drop(&mut self, value: bool) -> &mut Self {
        self.drag_and_drop = Some(value);
        self
    }

    /// This will probably not work as expected!
    pub fn with_position(&mut self, value: Option<Pos2>) -> &mut Self {
        self.position = Some(value);
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
    pub fn with_name(&mut self, id: impl Into<String>, instance: impl Into<String>) -> &mut Self {
        self.name = Some((id.into(), instance.into()));
        self
    }

    /// Is not implemented for winit
    /// You should use `ViewportCommand::CursorHitTest` if you want to set this!
    #[deprecated]
    pub fn with_hittest(&mut self, value: bool) -> &mut Self {
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
                if let Some(position) = new_position {
                    commands.push(ViewportCommand::OuterPosition(position));
                }
            }
        }

        if let Some(new_inner_size) = new.inner_size {
            if Some(new_inner_size) != self.inner_size {
                self.inner_size = Some(new_inner_size);
                if let Some(inner_size) = new_inner_size {
                    commands.push(ViewportCommand::InnerSize(inner_size));
                }
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

        if let Some(new_icon) = new.icon.clone() {
            let eq = match &new_icon {
                Some(icon) => {
                    if let Some(self_icon) = &self.icon {
                        matches!(self_icon, Some(self_icon) if Arc::ptr_eq(icon, self_icon))
                    } else {
                        false
                    }
                }
                None => self.icon == Some(None),
            };

            if !eq {
                commands.push(ViewportCommand::WindowIcon(
                    new_icon.as_ref().map(|i| i.as_ref().clone()),
                ));
                self.icon = Some(new_icon);
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

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum WindowLevel {
    Normal,
    AlwaysOnBottom,
    AlwaysOnTop,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum IMEPurpose {
    Normal,
    Password,
    Terminal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum SystemTheme {
    Light,
    Dark,
    SystemDefault,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum CursorGrab {
    None,
    Confined,
    Locked,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum UserAttentionType {
    Informational,
    Critical,
}

/// You can send a [`ViewportCommand`] to the viewport with [`Context::viewport_command`].
///
/// All coordinates are in logical points.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ViewportCommand {
    /// Set the title
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
    MinInnerSize(Option<Vec2>),

    /// Should be bigger then 0
    MaxInnerSize(Option<Vec2>),

    /// Should be bigger then 0
    ResizeIncrements(Option<Vec2>),

    /// Begin resizing the viewport with the left mouse button until the button is released.
    ///
    /// There's no guarantee that this will work unless the left mouse button was pressed
    /// immediately before this function is called.
    BeginResize {
        top: bool,
        bottom: bool,
        right: bool,
        left: bool,
    },

    /// Can the window be resized?
    Resizable(bool),

    /// Set which window buttons are enabled
    EnableButtons {
        close: bool,
        minimized: bool,
        maximize: bool,
    },
    Minimized(bool),
    Maximized(bool),
    Fullscreen(bool),

    /// Show window decorations, i.e. the chrome around the content
    /// with the title bar, close buttons, resize handles, etc.
    Decorations(bool),

    WindowLevel(WindowLevel),
    WindowIcon(Option<ColorImage>),

    IMEPosition(Pos2),
    IMEAllowed(bool),
    IMEPurpose(IMEPurpose),

    RequestUserAttention(Option<UserAttentionType>),

    SetTheme(SystemTheme),

    ContentProtected(bool),

    /// Will probably not work as expected!
    CursorPosition(Pos2),

    CursorGrab(CursorGrab),

    CursorVisible(bool),

    CursorHitTest(bool),
}

#[derive(Clone)]
pub(crate) struct ViewportState {
    pub(crate) builder: ViewportBuilder,

    /// Id of us and our parent.
    pub(crate) ids: ViewportIdPair,

    /// Has this viewport been updated this frame?
    pub(crate) used: bool,

    /// The user-code that shows the GUI, used for deferred viewports.
    ///
    /// `None` for immediate viewports.
    pub(crate) viewport_ui_cb: Option<Arc<ViewportUiCallback>>,
}

/// Describes a viewport, i.e. a native window.
#[derive(Clone)]
pub struct ViewportOutput {
    /// Id of us and our parent.
    pub ids: ViewportIdPair,

    pub builder: ViewportBuilder,

    /// The user-code that shows the GUI, used for deferred viewports.
    ///
    /// `None` for immediate viewports and the ROOT viewport.
    pub viewport_ui_cb: Option<Arc<ViewportUiCallback>>,
}

impl ViewportOutput {
    pub fn id(&self) -> ViewportId {
        self.ids.this
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
