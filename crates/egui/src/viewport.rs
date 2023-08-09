use std::fmt::Display;

use crate::Context;

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

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct ViewportBuilder {
    pub title: String,
    pub name: Option<String>,
    pub position: Option<(i32, i32)>,
    pub inner_size: Option<(u32, u32)>,
    pub fullscreen: bool,
    pub maximized: bool,
    pub resizable: bool,
    pub transparent: bool,
    pub decorations: bool,
    pub icon: Option<(u32, u32, Vec<u8>)>,
    pub active: bool,
    pub visible: bool,
    pub title_hidden: bool,
    pub titlebar_transparent: bool,
    pub fullsize_content_view: bool,
    pub min_inner_size: Option<(u32, u32)>,
    pub max_inner_size: Option<(u32, u32)>,
    pub drag_and_drop: bool,

    pub close_button: bool,
}

impl Default for ViewportBuilder {
    fn default() -> Self {
        Self {
            title: "Dummy EGUI Window".into(),
            name: None,
            position: None,
            inner_size: Some((300, 100)),
            fullscreen: false,
            maximized: false,
            resizable: true,
            transparent: false,
            decorations: true,
            icon: None,
            active: true,
            visible: true,
            title_hidden: false,
            titlebar_transparent: false,
            fullsize_content_view: false,
            min_inner_size: None,
            max_inner_size: None,
            drag_and_drop: true,
            close_button: true,
        }
    }
}

impl ViewportBuilder {
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    pub fn with_fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    pub fn with_window_icon(mut self, icon: Option<(u32, u32, Vec<u8>)>) -> Self {
        self.icon = icon;
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn with_title_hidden(mut self, title_hidden: bool) -> Self {
        self.title_hidden = title_hidden;
        self
    }

    pub fn with_titlebar_transparent(mut self, value: bool) -> Self {
        self.titlebar_transparent = value;
        self
    }

    pub fn with_fullsize_content_view(mut self, value: bool) -> Self {
        self.fullsize_content_view = value;
        self
    }

    pub fn with_inner_size(mut self, value: (u32, u32)) -> Self {
        self.inner_size = Some(value);
        self
    }

    pub fn with_min_inner_size(mut self, value: (u32, u32)) -> Self {
        self.min_inner_size = Some(value);
        self
    }

    pub fn with_max_inner_size(mut self, value: (u32, u32)) -> Self {
        self.max_inner_size = Some(value);
        self
    }

    pub fn with_drag_and_drop(mut self, value: bool) -> Self {
        self.drag_and_drop = value;
        self
    }

    pub fn with_position(mut self, value: (i32, i32)) -> Self {
        self.position = Some(value);
        self
    }
}

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ViewportCommand {
    Drag,
    InnerSize(u32, u32),
    /// Top, Bottom, Right, Left
    Resize(bool, bool, bool, bool),
}
