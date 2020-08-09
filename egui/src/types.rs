use std::sync::Arc;

use crate::{math::Rect, Context, Ui};

// ----------------------------------------------------------------------------

/// What Egui emits each frame.
/// The backend should use this.
#[derive(Clone, Default)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Output {
    /// Set the cursor to this icon.
    pub cursor_icon: CursorIcon,

    /// If set, open this url.
    pub open_url: Option<String>,

    /// Response to Event::Copy or Event::Cut. Ignore if empty.
    pub copied_text: String,

    /// Set to `true` to request another repaint right after this one.
    /// This is only used in reactive backends (i.e. backends where we repaint on new input).
    /// For instance, you may want to set this to `true` while there is an animation.
    pub needs_repaint: bool,
}

#[derive(Clone, Copy)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize))]
// #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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

/// The result of an interaction.
///
/// For instance, this lets you know whether or not a widget has been clicked this frame.
#[derive(Clone, Copy, Debug)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct InteractInfo {
    /// The senses (click or drag) that the widget is interested in (if any).
    pub sense: Sense,

    /// The mouse is hovering above this thing
    pub hovered: bool,

    /// The mouse pressed this thing ealier, and now released on this thing too.
    pub clicked: bool,

    pub double_clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it or holding it)
    pub active: bool,

    /// This widget has the keyboard focus (i.e. is receiving key pressed)
    pub has_kb_focus: bool,

    /// The region of the screen we are talking about
    pub rect: Rect,
}

impl InteractInfo {
    pub fn nothing() -> Self {
        Self {
            sense: Sense::nothing(),
            hovered: false,
            clicked: false,
            double_clicked: false,
            active: false,
            has_kb_focus: false,
            rect: Rect::nothing(),
        }
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            sense: self.sense.union(other.sense),
            hovered: self.hovered || other.hovered,
            clicked: self.clicked || other.clicked,
            double_clicked: self.double_clicked || other.double_clicked,
            active: self.active || other.active,
            has_kb_focus: self.has_kb_focus || other.has_kb_focus,
            rect: self.rect.union(other.rect),
        }
    }
}

// ----------------------------------------------------------------------------

/// The result of adding a widget to an `Ui`.
///
/// This lets you know whether or not a widget has been clicked this frame.
/// It also lets you easily show a tooltip on hover.
pub struct GuiResponse {
    /// The senses (click or drag) that the widget is interested in (if any).
    pub sense: Sense,

    /// The mouse is hovering above this
    pub hovered: bool,

    /// The mouse clicked this thing this frame
    pub clicked: bool,

    pub double_clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it)
    pub active: bool,

    /// This widget has the keyboard focus (i.e. is receiving key pressed)
    pub has_kb_focus: bool,

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
            sense: self.sense,
            hovered: self.hovered,
            clicked: self.clicked,
            double_clicked: self.double_clicked,
            active: self.active,
            has_kb_focus: self.has_kb_focus,
            rect: self.rect,
        }
    }
}

// ----------------------------------------------------------------------------

/// What sort of interaction is a widget sensitive to?
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize))]
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

    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            click: self.click | other.click,
            drag: self.drag | other.drag,
        }
    }
}
