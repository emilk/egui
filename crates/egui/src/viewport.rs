use std::{fmt::Display, sync::Arc};

use epaint::Pos2;

use crate::{Context, Id};

/// This is used to send a command to a specific viewport
///
/// This is returned by `Context::get_viewport_id` and `Context::get_parent_viewport_id`
#[derive(Default, Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct ViewportId(pub(crate) u64);

impl Display for ViewportId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ViewportId {
    /// This will return the `ViewportId` of the main viewport
    pub const MAIN: Self = Self(0);
}

/// This is used to render an async viewport
pub type ViewportRender = dyn Fn(&Context) + Sync + Send;

pub type ViewportRenderSyncCallback = dyn for<'a> Fn(&Context, ViewportBuilder, ViewportId, ViewportId, Box<dyn FnOnce(&Context) + 'a>)
    + Send
    + Sync;

/// The filds in this struct should not be change directly, but is not problem tho!
/// Every thing is wrapped in Option<> indicates that thing should not be changed!
#[derive(PartialEq, Eq, Clone)]
#[allow(clippy::option_option)]
pub struct ViewportBuilder {
    pub id: Id,
    pub title: String,
    pub name: Option<(String, String)>,
    pub position: Option<Option<Pos2>>,
    pub inner_size: Option<Option<Pos2>>,
    pub fullscreen: Option<bool>,
    pub maximized: Option<bool>,
    pub minimized: Option<bool>,
    pub resizable: Option<bool>,
    pub transparent: Option<bool>,
    pub decorations: Option<bool>,
    pub icon: Option<Option<Arc<(u32, u32, Vec<u8>)>>>,
    pub active: Option<bool>,
    pub visible: Option<bool>,
    pub title_hidden: Option<bool>,
    pub titlebar_transparent: Option<bool>,
    pub fullsize_content_view: Option<bool>,
    pub min_inner_size: Option<Option<Pos2>>,
    pub max_inner_size: Option<Option<Pos2>>,
    pub drag_and_drop: Option<bool>,

    pub close_button: Option<bool>,
    pub minimize_button: Option<bool>,
    pub maximize_button: Option<bool>,

    pub hittest: Option<bool>,
}

impl ViewportBuilder {
    pub fn new(id: impl Into<Id>) -> Self {
        Self {
            id: id.into(),
            title: "Dummy egui viewport".into(),
            name: None,
            position: None,
            inner_size: Some(Some(Pos2::new(300.0, 200.0))),
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
            drag_and_drop: None,
            close_button: None,
            minimized: Some(false),
            minimize_button: Some(true),
            maximize_button: Some(true),
            hittest: Some(true),
        }
    }
}

impl ViewportBuilder {
    pub fn empty(id: impl Into<Id>) -> Self {
        Self {
            id: id.into(),
            title: "Dummy egui viewport".into(),
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
            minimized: None,
            minimize_button: None,
            maximize_button: None,
            hittest: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = Some(decorations);
        self
    }

    pub fn with_fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = Some(fullscreen);
        self
    }

    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = Some(maximized);
        self
    }

    pub fn with_mimimized(mut self, minimized: bool) -> Self {
        self.minimized = Some(minimized);
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = Some(resizable);
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = Some(transparent);
        self
    }

    /// The icon needs to be wrapped in Arc because will be copied every frame
    pub fn with_window_icon(mut self, icon: Option<Arc<(u32, u32, Vec<u8>)>>) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    pub fn with_title_hidden(mut self, title_hidden: bool) -> Self {
        self.title_hidden = Some(title_hidden);
        self
    }

    pub fn with_titlebar_transparent(mut self, value: bool) -> Self {
        self.titlebar_transparent = Some(value);
        self
    }

    pub fn with_fullsize_content_view(mut self, value: bool) -> Self {
        self.fullsize_content_view = Some(value);
        self
    }

    /// Should be bigger then 0
    pub fn with_inner_size(mut self, value: Option<Pos2>) -> Self {
        self.inner_size = Some(value);
        self
    }

    /// Should be bigger then 0
    pub fn with_min_inner_size(mut self, value: Option<Pos2>) -> Self {
        self.min_inner_size = Some(value);
        self
    }

    /// Should be bigger then 0
    pub fn with_max_inner_size(mut self, value: Option<Pos2>) -> Self {
        self.max_inner_size = Some(value);
        self
    }

    pub fn with_close_button(mut self, value: bool) -> Self {
        self.close_button = Some(value);
        self
    }

    pub fn with_minimize_button(mut self, value: bool) -> Self {
        self.minimize_button = Some(value);
        self
    }

    pub fn with_maximize_button(mut self, value: bool) -> Self {
        self.maximize_button = Some(value);
        self
    }

    /// This currently only work on windows to be disabled!
    pub fn with_drag_and_drop(mut self, value: bool) -> Self {
        self.drag_and_drop = Some(value);
        self
    }

    pub fn with_position(mut self, value: Option<Pos2>) -> Self {
        self.position = Some(value);
        self
    }

    pub fn with_name(mut self, id: impl Into<String>, instance: impl Into<String>) -> Self {
        self.name = Some((id.into(), instance.into()));
        self
    }

    /// Is not implemented for winit
    /// You should use `ViewportCommand::CursorHitTest` if you want to set this!
    pub fn with_hittest(mut self, value: bool) -> Self {
        self.hittest = Some(value);
        self
    }
}

/// You can send a `ViewportCommand` to the viewport with `Context::viewport_command`
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ViewportCommand {
    Title(String),
    Transparent(bool),
    Visible(bool),
    Drag,
    OuterPosition(Pos2),

    /// Should be bigger then 0
    InnerSize(Pos2),

    /// Should be bigger then 0
    MinInnerSize(Option<Pos2>),

    /// Should be bigger then 0
    MaxInnerSize(Option<Pos2>),
    ResizeIncrements(Option<Pos2>),

    /// Top, Bottom, Right, Left
    Resize(bool, bool, bool, bool),
    Resizable(bool),
    EnableButtons {
        close: bool,
        minimized: bool,
        maximize: bool,
    },
    Minimized(bool),
    Maximized(bool),
    Fullscreen(bool),
    Decorations(bool),

    /// 0 = Normal, 1 = AlwaysOnBottom, 2 = AlwaysOnTop
    WindowLevel(u8),
    WindowIcon(Option<(Vec<u8>, u32, u32)>),
    IMEPosition(Pos2),
    IMEAllowed(bool),

    /// 0 = Normal, 1 = Password, 2 = Terminal
    IMEPurpose(u8),

    /// 0 = Informational, 1 = Critical
    RequestUserAttention(Option<u8>),

    /// 0 = Light, 1 = Dark
    SetTheme(Option<u8>),

    ContentProtected(bool),

    CursorPosition(Pos2),

    /// 0 = None, 1 = Confined, 2 = Locked
    CursorGrab(u8),

    CursorVisible(bool),

    CursorHitTest(bool),
}
