use crate::{math::Rect, CtxRef, Ui};

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
    /// Used when moving
    Grab,
    Grabbing,
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
    pub ctx: CtxRef,

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

    /// The widget had keyboard focus and lost it,
    /// perhaps because the user pressed enter.
    /// This is often a signal to the user to the application
    /// to make use of the contents of the text field.
    pub lost_kb_focus: bool,
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            ctx: _,
            rect,
            sense,
            hovered,
            clicked,
            double_clicked,
            active,
            has_kb_focus,
            lost_kb_focus,
        } = self;
        f.debug_struct("Response")
            .field("rect", rect)
            .field("sense", sense)
            .field("hovered", hovered)
            .field("clicked", clicked)
            .field("double_clicked", double_clicked)
            .field("active", active)
            .field("has_kb_focus", has_kb_focus)
            .field("lost_kb_focus", lost_kb_focus)
            .finish()
    }
}

impl Response {
    /// Show this UI if the item was hovered (i.e. a tooltip).
    /// If you call this multiple times the tooltips will stack underneath the previous ones.
    pub fn on_hover_ui(self, add_contents: impl FnOnce(&mut Ui)) -> Self {
        if self.hovered {
            crate::containers::show_tooltip(&self.ctx, add_contents);
        }
        self
    }

    /// Show this text if the item was hovered (i.e. a tooltip).
    /// If you call this multiple times the tooltips will stack underneath the previous ones.
    pub fn on_hover_text(self, text: impl Into<String>) -> Self {
        self.on_hover_ui(|ui| {
            ui.add(crate::widgets::Label::new(text));
        })
    }

    #[deprecated = "Deprecated 2020-10-01: use `on_hover_text` instead."]
    pub fn tooltip_text(self, text: impl Into<String>) -> Self {
        self.on_hover_text(text)
    }
}

impl Response {
    /// A logical "or" operation.
    /// For instance `a.union(b).hovered` means "was either a or b hovered?".
    pub fn union(&self, other: Self) -> Self {
        assert!(self.ctx == other.ctx);
        Self {
            ctx: other.ctx,
            rect: self.rect.union(other.rect),
            sense: self.sense.union(other.sense),
            hovered: self.hovered || other.hovered,
            clicked: self.clicked || other.clicked,
            double_clicked: self.double_clicked || other.double_clicked,
            active: self.active || other.active,
            has_kb_focus: self.has_kb_focus || other.has_kb_focus,
            lost_kb_focus: self.lost_kb_focus || other.lost_kb_focus,
        }
    }
}

/// To summarize the response from many widgets you can use this pattern:
///
/// ```
/// use egui::*;
/// fn draw_vec2(ui: &mut Ui, v: &mut Vec2) -> Response {
///     ui.add(DragValue::f32(&mut v.x)) | ui.add(DragValue::f32(&mut v.y))
/// }
/// ```
///
/// Now `draw_vec2(ui, foo).hovered` is true if either `DragValue` were hovered.
impl std::ops::BitOr for Response {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

/// To summarize the response from many widgets you can use this pattern:
///
/// ```
/// # let mut ui = egui::Ui::__test();
/// # let (widget_a, widget_b, widget_c) = (egui::Label::new("a"), egui::Label::new("b"), egui::Label::new("c"));
/// let mut response = ui.add(widget_a);
/// response |= ui.add(widget_b);
/// response |= ui.add(widget_c);
/// if response.active { ui.label("You are interacting with one of the widgets"); }
/// ```
impl std::ops::BitOrAssign for Response {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
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
    /// Senses no clicks or drags (but everything senses mouse hover).
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
