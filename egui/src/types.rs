use crate::{math::Rect, CtxRef, Id, LayerId, Ui, lerp};

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
    /// Used for optionally showing a tooltip and checking for more interactions.
    pub ctx: CtxRef,

    // IN:
    /// Which layer the widget is part of.
    pub layer_id: LayerId,

    /// The `Id` of the widget/area this response pertains.
    pub id: Id,

    /// The area of the screen we are talking about.
    pub rect: Rect,

    /// The senses (click or drag) that the widget is interested in (if any).
    pub sense: Sense,

    // OUT:
    /// The mouse is hovering above this.
    pub hovered: bool,

    /// The mouse clicked this thing this frame.
    pub clicked: bool,

    /// The thing was double-clicked.
    pub double_clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it).
    pub active: bool,

    /// This widget has the keyboard focus (i.e. is receiving key pressed).
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
            layer_id,
            id,
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
            .field("layer_id", layer_id)
            .field("id", id)
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

    /// Check for more interactions (e.g. sense clicks on a `Response` returned from a label).
    ///
    /// ```
    /// # let mut ui = egui::Ui::__test();
    /// let response = ui.label("hello");
    /// assert!(!response.clicked); // labels don't sense clicks
    /// let response = response.interact(egui::Sense::click());
    /// if response.clicked { /* … */ }
    /// ```
    pub fn interact(&self, sense: Sense) -> Self {
        self.ctx
            .interact_with_hovered(self.layer_id, self.id, self.rect, sense, self.hovered)
    }

    /// Move the scroll to this UI.
    /// The scroll centering is based on the `center_factor`:
    /// 0.0f - at the top, 0.5f - at the middle, 1.0f - at the bottom.
    pub fn scroll_to_me(&self, center_factor: f32) {
        let scroll_target = lerp(self.rect.y_range(), center_factor);

        let mut frame_state = self.ctx.frame_state();
        frame_state.set_scroll_target(Some(scroll_target));
        frame_state.set_scroll_target_center_factor(center_factor);
    }
}

impl Response {
    /// A logical "or" operation.
    /// For instance `a.union(b).hovered` means "was either a or b hovered?".
    pub fn union(&self, other: Self) -> Self {
        assert!(self.ctx == other.ctx);
        debug_assert_eq!(
            self.layer_id, other.layer_id,
            "It makes no sense to combine Responses from two different layers"
        );
        Self {
            ctx: other.ctx,
            layer_id: self.layer_id,
            id: self.id,
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
    /// Senses no clicks or drags. Only senses mouse hover.
    pub fn hover() -> Self {
        Self {
            click: false,
            drag: false,
        }
    }

    #[deprecated = "Use hover()"]
    pub fn nothing() -> Self {
        Sense::hover()
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
