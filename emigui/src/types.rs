use std::sync::Arc;

use serde_derive::Serialize;

use crate::{math::Rect, Context, Ui};

// ----------------------------------------------------------------------------

#[derive(Clone, Default, Serialize)]
pub struct Output {
    pub cursor_icon: CursorIcon,

    /// If set, open this url.
    pub open_url: Option<String>,

    /// Response to Event::Copy or Event::Cut. Ignore if empty.
    pub copied_text: String,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorIcon {
    Default,
    /// Pointing hand, used for e.g. web links
    PointingHand,
    ResizeHorizontal,
    ResizeNeSw,
    ResizeNwSe,
    ResizeVertical,
    Text,
}

impl Default for CursorIcon {
    fn default() -> Self {
        Self::Default
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Serialize)]
pub struct InteractInfo {
    /// The mouse is hovering above this thing
    pub hovered: bool,

    /// The mouse pressed this thing ealier, and now released on this thing too.
    pub clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it or holding it)
    pub active: bool,

    /// The region of the screen we are talking about
    pub rect: Rect,
}

impl InteractInfo {
    pub fn nothing() -> Self {
        Self {
            hovered: false,
            clicked: false,
            active: false,
            rect: Rect::nothing(),
        }
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            hovered: self.hovered || other.hovered,
            clicked: self.clicked || other.clicked,
            active: self.active || other.active,
            rect: self.rect.union(other.rect),
        }
    }
}

// ----------------------------------------------------------------------------

// TODO: rename GuiResponse
pub struct GuiResponse {
    /// The mouse is hovering above this
    pub hovered: bool,

    /// The mouse clicked this thing this frame
    pub clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it)
    pub active: bool,

    /// The area of the screen we are talking about
    pub rect: Rect,

    /// Used for optionally showing a tooltip
    pub ctx: Arc<Context>,
}

impl GuiResponse {
    /// Show some stuff if the item was hovered
    pub fn tooltip(&mut self, add_contents: impl FnOnce(&mut Ui)) -> &mut Self {
        if self.hovered {
            crate::containers::show_tooltip(&self.ctx, add_contents);
        }
        self
    }

    /// Show this text if the item was hovered
    pub fn tooltip_text(&mut self, text: impl Into<String>) -> &mut Self {
        self.tooltip(|popup| {
            popup.add(crate::widgets::Label::new(text));
        })
    }
}

impl Into<InteractInfo> for GuiResponse {
    fn into(self) -> InteractInfo {
        InteractInfo {
            hovered: self.hovered,
            clicked: self.clicked,
            active: self.active,
            rect: self.rect,
        }
    }
}

// ----------------------------------------------------------------------------

/// What sort of interaction is a widget sensitive to?
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sense {
    /// buttons, sliders, windows ...
    pub click: bool,

    /// sliders, windows, scroll bars, scroll areas ...
    pub drag: bool,
}

impl Sense {
    pub fn nothing() -> Self {
        Self {
            click: false,
            drag: false,
        }
    }

    pub fn click() -> Self {
        Self {
            click: true,
            drag: false,
        }
    }

    pub fn drag() -> Self {
        Self {
            click: false,
            drag: true,
        }
    }

    /// e.g. a slider or window
    pub fn click_and_drag() -> Self {
        Self {
            click: true,
            drag: true,
        }
    }
}
