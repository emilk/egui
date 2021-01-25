use crate::{
    math::{lerp, Align, Pos2, Rect},
    PointerButton, NUM_POINTER_BUTTONS,
};
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
    /// The pointer is hovering above this widget or the widget was clicked/tapped this frame.
    pub(crate) hovered: bool,

    /// The pointer clicked this thing this frame.
    pub(crate) clicked: [bool; NUM_POINTER_BUTTONS],

    /// The thing was double-clicked.
    pub(crate) double_clicked: [bool; NUM_POINTER_BUTTONS],

    /// The widgets is being dragged
    pub(crate) dragged: bool,

    /// The widget was being dragged, but now it has been released.
    pub(crate) drag_released: bool,

    /// Is the pointer button currently down on this widget?
    /// This is true if the pointer is pressing down or dragging a widget
    pub(crate) is_pointer_button_down_on: bool,

    /// Where the pointer (mouse/touch) were when when this widget was clicked or dragged.
    /// `None` if the widget is not being interacted with.
    pub(crate) interact_pointer_pos: Option<Pos2>,

    /// This widget has the keyboard focus (i.e. is receiving key pressed).
    pub(crate) has_kb_focus: bool,

    /// The widget had keyboard focus and lost it.
    pub(crate) lost_kb_focus: bool,
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
            dragged,
            drag_released,
            is_pointer_button_down_on,
            interact_pointer_pos,
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
            .field("dragged", dragged)
            .field("drag_released", drag_released)
            .field("is_pointer_button_down_on", is_pointer_button_down_on)
            .field("interact_pointer_pos", interact_pointer_pos)
            .field("has_kb_focus", has_kb_focus)
            .field("lost_kb_focus", lost_kb_focus)
            .finish()
    }
}

impl Response {
    /// Returns true if this widget was clicked this frame by the primary button.
    pub fn clicked(&self) -> bool {
        self.clicked[PointerButton::Primary as usize]
    }

    /// Returns true if this widget was clicked this frame by the secondary mouse button (e.g. the right mouse button).
    pub fn secondary_clicked(&self) -> bool {
        self.clicked[PointerButton::Secondary as usize]
    }

    /// Returns true if this widget was clicked this frame by the middle mouse button.
    pub fn middle_clicked(&self) -> bool {
        self.clicked[PointerButton::Middle as usize]
    }

    /// Returns true if this widget was double-clicked this frame by the primary button.
    pub fn double_clicked(&self) -> bool {
        self.double_clicked[PointerButton::Primary as usize]
    }

    /// The pointer is hovering above this widget or the widget was clicked/tapped this frame.
    pub fn hovered(&self) -> bool {
        self.hovered
    }

    /// The widget had keyboard focus and lost it,
    /// perhaps because the user pressed enter.
    /// If you want to do an action when a user presses enter in a text field,
    /// use this.
    ///
    /// ```
    /// # let mut ui = egui::Ui::__test();
    /// # let mut my_text = String::new();
    /// # fn do_request(_: &str) {}
    /// if ui.text_edit_singleline(&mut my_text).lost_kb_focus() {
    ///     do_request(&my_text);
    /// }
    /// ```
    pub fn lost_kb_focus(&self) -> bool {
        self.lost_kb_focus
    }

    /// The widgets is being dragged.
    ///
    /// To find out which button(s), query [`PointerState::button_down`]
    /// (`ui.input().pointer.button_down(…)`).
    pub fn dragged(&self) -> bool {
        self.dragged
    }

    /// The widget was being dragged, but now it has been released.
    pub fn drag_released(&self) -> bool {
        self.drag_released
    }

    /// Returns true if this widget was clicked this frame by the given button.
    pub fn clicked_by(&self, button: PointerButton) -> bool {
        self.clicked[button as usize]
    }

    /// Returns true if this widget was double-clicked this frame by the given button.
    pub fn double_clicked_by(&self, button: PointerButton) -> bool {
        self.double_clicked[button as usize]
    }

    /// Where the pointer (mouse/touch) were when when this widget was clicked or dragged.
    /// `None` if the widget is not being interacted with.
    pub fn interact_pointer_pos(&self) -> Option<Pos2> {
        self.interact_pointer_pos
    }

    /// Is the pointer button currently down on this widget?
    /// This is true if the pointer is pressing down or dragging a widget
    pub fn is_pointer_button_down_on(&self) -> bool {
        self.is_pointer_button_down_on
    }

    /// Show this UI if the item was hovered (i.e. a tooltip).
    /// If you call this multiple times the tooltips will stack underneath the previous ones.
    pub fn on_hover_ui(self, add_contents: impl FnOnce(&mut Ui)) -> Self {
        if (self.hovered() && self.ctx.input().pointer.tooltip_pos().is_some())
            || self.ctx.memory().everything_is_visible()
        {
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
    /// assert!(!response.clicked()); // labels don't sense clicks
    /// let response = response.interact(egui::Sense::click());
    /// if response.clicked() { /* … */ }
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
    ///         if response.clicked() {
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
            clicked: [
                self.clicked[0] || other.clicked[0],
                self.clicked[1] || other.clicked[1],
                self.clicked[2] || other.clicked[2],
            ],
            double_clicked: [
                self.double_clicked[0] || other.double_clicked[0],
                self.double_clicked[1] || other.double_clicked[1],
                self.double_clicked[2] || other.double_clicked[2],
            ],
            dragged: self.dragged || other.dragged,
            drag_released: self.drag_released || other.drag_released,
            is_pointer_button_down_on: self.is_pointer_button_down_on
                || other.is_pointer_button_down_on,
            interact_pointer_pos: self.interact_pointer_pos.or(other.interact_pointer_pos),
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
/// if response.hovered() { ui.label("You hovered at least one of the widgets"); }
/// ```
impl std::ops::BitOrAssign for Response {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}
