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

    /// If `true`, Egui or a user is indicating that the UI needs immediate repaint (e.g. on the next frame).
    /// This happens for instance when there is an animation, or if a user has called `Context::request_repaint()`.
    /// Don't set this manually, but call `Context::request_repaint()` instead.
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

/// The result of adding a widget to an `Ui`.
///
/// This lets you know whether or not a widget has been clicked this frame.
/// It also lets you easily show a tooltip on hover.
#[derive(Clone)]
pub struct Response {
    // CONTEXT:
    /// Used for optionally showing a tooltip
    pub ctx: Arc<Context>,

    // IN:
    /// The area of the screen we are talking about
    pub rect: Rect,

    /// The senses (click or drag) that the widget is interested in (if any).
    pub sense: Sense,

    // OUT:
    /// The mouse is hovering above this
    pub hovered: bool,

    /// The mouse clicked this thing this frame
    pub clicked: bool,

    /// The thing was double-clicked
    pub double_clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it)
    pub active: bool,

    /// This widget has the keyboard focus (i.e. is receiving key pressed)
    pub has_kb_focus: bool,
}

impl Response {
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

impl Response {
    pub fn union(self, other: Self) -> Self {
        assert!(Arc::ptr_eq(&self.ctx, &other.ctx));
        Self {
            ctx: self.ctx,
            rect: self.rect.union(other.rect),
            sense: self.sense.union(other.sense),
            hovered: self.hovered || other.hovered,
            clicked: self.clicked || other.clicked,
            double_clicked: self.double_clicked || other.double_clicked,
            active: self.active || other.active,
            has_kb_focus: self.has_kb_focus || other.has_kb_focus,
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
