use crate::math::{lerp, Align, Rect};
use crate::{CtxRef, Id, LayerId, Sense, Ui};

// ----------------------------------------------------------------------------

/// The result of adding a widget to a [`Ui`].
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

    /// The senses (click and/or drag) that the widget was interested in (if any).
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
    /// If you want to do an action when a user presses enter in a text field,
    /// use this.
    ///
    /// ```
    /// # let mut ui = egui::Ui::__test();
    /// # let mut my_text = String::new();
    /// # fn do_request(_: &str) {}
    /// if ui.text_edit_singleline(&mut my_text).lost_kb_focus {
    ///     do_request(&my_text);
    /// }
    /// ```
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
        if self.hovered || self.ctx.memory().everything_is_visible() {
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

    /// Move the scroll to this UI with the specified alignment.
    ///
    /// ```
    /// # use egui::Align;
    /// # let mut ui = &mut egui::Ui::__test();
    /// egui::ScrollArea::auto_sized().show(ui, |ui| {
    ///     for i in 0..1000 {
    ///         let response = ui.button(format!("Button {}", i));
    ///         if response.clicked {
    ///             response.scroll_to_me(Align::Center);
    ///         }
    ///     }
    /// });
    /// ```
    pub fn scroll_to_me(&self, align: Align) {
        let scroll_target = lerp(self.rect.y_range(), align.to_factor());
        self.ctx.frame_state().scroll_target = Some((scroll_target, align));
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
