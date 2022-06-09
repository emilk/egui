use crate::{
    emath::{Align, Pos2, Rect, Vec2},
    menu, Context, CursorIcon, Id, LayerId, PointerButton, Sense, Ui, WidgetText,
    NUM_POINTER_BUTTONS,
};

// ----------------------------------------------------------------------------

/// The result of adding a widget to a [`Ui`].
///
/// A [`Response`] lets you know whether or not a widget is being hovered, clicked or dragged.
/// It also lets you easily show a tooltip on hover.
///
/// Whenever something gets added to a [`Ui`], a [`Response`] object is returned.
/// [`ui.add`] returns a [`Response`], as does [`ui.button`], and all similar shortcuts.
#[derive(Debug, Clone)]
pub struct Response {
    layer_id: LayerId,
    id: Id,
    rect: Rect,
    sense: Sense,
    interact_pointer_pos: Option<Pos2>,
    hover_pointer_pos: Option<Pos2>,
    pointer_delta: Vec2,
    enabled: bool,
    hovered: bool,
    pointer_pressed: [bool; NUM_POINTER_BUTTONS],
    pointer_down: [bool; NUM_POINTER_BUTTONS],
    clicked: [bool; NUM_POINTER_BUTTONS],
    // TODO: `released` for sliders
    double_clicked: [bool; NUM_POINTER_BUTTONS],
    triple_clicked: [bool; NUM_POINTER_BUTTONS],
    dragged: bool,
    drag_released: bool,
    is_pointer_button_down_on: bool,
    changed: bool,
    clicked_elsewhere: bool,
    has_focus: bool,
    gained_focus: bool,
    lost_focus: bool,
}

impl Response {
    /// Which layer the widget is part of.
    #[inline]
    pub fn layer_id(&self) -> &LayerId {
        &self.layer_id
    }

    /// The [`Id`] of the widget/area this response pertains.
    #[inline]
    pub fn id(&self) -> Id {
        self.id
    }

    /// The area of the screen this widget occupies.
    #[inline]
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// The senses (click and/or drag) that the widget was interested in (if any).
    #[inline]
    pub fn sense(&self) -> Sense {
        self.sense
    }

    /// `true` if there was a click *outside* this widget this frame.
    #[inline]
    pub fn clicked_elsewhere(&self) -> bool {
        self.clicked_elsewhere
    }

    /// This widget has the keyboard focus (i.e. is receiving key presses).
    #[inline]
    pub fn has_focus(&self) -> bool {
        self.has_focus
    }

    /// True if this widget has keyboard focus this frame, but didn't last frame.
    #[inline]
    pub fn gained_focus(&self) -> bool {
        self.gained_focus
    }

    /// The widget had keyboard focus and lost it,
    /// either because the user pressed tab or clicked somewhere else,
    /// or (in case of a [`crate::TextEdit`]) because the user pressed enter.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_text = String::new();
    /// # fn do_request(_: &str) {}
    /// let response = ui.text_edit_singleline(&mut my_text);
    /// if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
    ///     do_request(&my_text);
    /// }
    /// # });
    /// ```
    #[inline]
    pub fn lost_focus(&self) -> bool {
        self.lost_focus
    }

    /// Request that this widget get keyboard focus.
    pub fn request_focus(&self, ctx: &mut Context) {
        ctx.memory_mut().request_focus(self.id);
    }

    /// Surrender keyboard focus for this widget.
    pub fn surrender_focus(&self, ctx: &mut Context) {
        ctx.memory_mut().surrender_focus(self.id);
    }

    /// Whether this widget is being dragged by the given pointer button.
    #[inline]
    pub fn dragged_by(&self, button: PointerButton) -> bool {
        self.dragged && self.pointer_down[button as usize]
    }

    /// Did a drag on this widgets begin this frame?
    #[inline]
    pub fn drag_started(&self) -> bool {
        self.dragged && self.pointer_pressed.into_iter().any(|v| v)
    }

    /// If dragged, how many points were we dragged and in what direction?
    #[inline]
    pub fn drag_delta(&self) -> Vec2 {
        if self.dragged() {
            self.pointer_delta
        } else {
            Vec2::ZERO
        }
    }

    /// If it is a good idea to show a tooltip, where is pointer?
    /// None if the pointer is outside the response area.
    #[inline]
    pub fn hover_pos(&self) -> Option<Pos2> {
        if self.hovered() {
            self.hover_pointer_pos
        } else {
            None
        }
    }

    /// Returns true if this widget was clicked this frame by the primary button.
    ///
    /// A click is registered when the mouse or touch is released within
    /// a certain amount of time and distance from when and where it was pressed.
    ///
    /// Note that the widget must be sensing clicks with [`Sense::click`].
    /// [`crate::Button`] senses clicks; [`crate::Label`] does not (unless you call [`crate::Label::sense`]).
    ///
    /// You can use [`Self::interact`] to sense more things *after* adding a widget.
    #[inline]
    pub fn clicked(&self) -> bool {
        self.clicked[PointerButton::Primary as usize]
    }

    /// Returns true if this widget was clicked this frame by the given button.
    #[inline]
    pub fn clicked_by(&self, button: PointerButton) -> bool {
        self.clicked[button as usize]
    }

    /// Returns true if this widget was clicked this frame by the secondary mouse button (e.g. the right mouse button).
    #[inline]
    pub fn secondary_clicked(&self) -> bool {
        self.clicked[PointerButton::Secondary as usize]
    }

    /// Returns true if this widget was clicked this frame by the middle mouse button.
    #[inline]
    pub fn middle_clicked(&self) -> bool {
        self.clicked[PointerButton::Middle as usize]
    }

    /// Returns true if this widget was double-clicked this frame by the primary button.
    #[inline]
    pub fn double_clicked(&self) -> bool {
        self.double_clicked[PointerButton::Primary as usize]
    }

    /// Returns true if this widget was triple-clicked this frame by the primary button.
    #[inline]
    pub fn triple_clicked(&self) -> bool {
        self.triple_clicked[PointerButton::Primary as usize]
    }

    /// Returns true if this widget was double-clicked this frame by the given button.
    #[inline]
    pub fn double_clicked_by(&self, button: PointerButton) -> bool {
        self.double_clicked[button as usize]
    }

    /// Returns true if this widget was triple-clicked this frame by the given button.
    #[inline]
    pub fn triple_clicked_by(&self, button: PointerButton) -> bool {
        self.triple_clicked[button as usize]
    }

    /// Was the widget enabled?
    /// If false, there was no interaction attempted
    /// and the widget should be drawn in a gray disabled look.
    #[inline]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// The pointer is hovering above this widget or the widget was clicked/tapped this frame.
    ///
    /// Note that this is slightly different from checking `response.rect().contains(pointer_pos)`.
    /// For one, the hover rectangle is slightly larger, by half of the current item spacing
    /// (to make it easier to click things). But `hovered` also checks that no other area
    /// is covering this response rectangle.
    #[inline]
    pub fn hovered(&self) -> bool {
        self.hovered
    }

    /// The widgets is being dragged.
    ///
    /// To find out which button(s), query [`crate::PointerState::button_down`]
    /// (`ui.input().pointer.button_down(…)`).
    ///
    /// Note that the widget must be sensing drags with [`Sense::drag`].
    /// [`crate::DragValue`] senses drags; [`crate::Label`] does not (unless you call [`crate::Label::sense`]).
    ///
    /// You can use [`Self::interact`] to sense more things *after* adding a widget.
    #[inline]
    pub fn dragged(&self) -> bool {
        self.dragged
    }

    /// The widget was being dragged, but now it has been released.
    #[inline]
    pub fn drag_released(&self) -> bool {
        self.drag_released
    }

    /// Where the pointer (mouse/touch) were when when this widget was clicked or dragged.
    /// `None` if the widget is not being interacted with.
    #[inline]
    pub fn interact_pointer_pos(&self) -> Option<Pos2> {
        self.interact_pointer_pos
    }

    /// Is the pointer button currently down on this widget?
    /// This is true if the pointer is pressing down or dragging a widget
    #[inline]
    pub fn is_pointer_button_down_on(&self) -> bool {
        self.is_pointer_button_down_on
    }

    /// What the underlying data changed?
    ///
    /// e.g. the slider was dragged, text was entered in a [`TextEdit`](crate::TextEdit) etc.
    /// Always `false` for something like a [`Button`](crate::Button).
    ///
    /// Can sometimes be `true` even though the data didn't changed
    /// (e.g. if the user entered a character and erased it the same frame).
    ///
    /// This is not set if the *view* of the data was changed.
    /// For instance, moving the cursor in a [`TextEdit`](crate::TextEdit) does not set this to `true`.
    #[inline]
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Report the data shown by this widget changed.
    ///
    /// This must be called by widgets that represent some mutable data,
    /// e.g. checkboxes, sliders etc.
    ///
    /// This should be called when the *content* changes, but not when the view does.
    /// So we call this when the text of a [`crate::TextEdit`], but not when the cursors changes.
    #[inline]
    pub fn mark_changed(&mut self) {
        self.changed = true;
    }

    #[inline]
    pub(crate) fn set_changed(&mut self, changed: bool) {
        self.changed = changed;
    }

    /// Show this UI if the widget was hovered (i.e. a tooltip).
    ///
    /// The text will not be visible if the widget is not enabled.
    /// For that, use [`Self::on_disabled_hover_ui`] instead.
    ///
    /// If you call this multiple times the tooltips will stack underneath the previous ones.
    #[doc(alias = "tooltip")]
    pub fn on_hover_ui(self, ctx: &mut Context, add_contents: impl FnOnce(&mut Ui<'_>)) -> Self {
        if self.should_show_hover_ui(ctx) {
            crate::containers::show_tooltip_for(
                ctx,
                self.id.with("__tooltip"),
                self.rect,
                add_contents,
            );
        }
        self
    }

    /// Show this UI when hovering if the widget is disabled.
    pub fn on_disabled_hover_ui(
        self,
        ctx: &mut Context,
        add_contents: impl FnOnce(&mut Ui<'_>),
    ) -> Self {
        if !self.enabled && ctx.rect_contains_pointer(self.layer_id, self.rect) {
            crate::containers::show_tooltip_for(
                ctx,
                self.id.with("__tooltip"),
                self.rect,
                add_contents,
            );
        }
        self
    }

    /// Like `on_hover_ui`, but show the ui next to cursor.
    pub fn on_hover_ui_at_pointer(
        self,
        ctx: &mut Context,
        add_contents: impl FnOnce(&mut Ui<'_>),
    ) -> Self {
        if self.should_show_hover_ui(ctx) {
            crate::containers::show_tooltip_at_pointer(
                ctx,
                self.id.with("__tooltip"),
                add_contents,
            );
        }
        self
    }

    fn should_show_hover_ui(&self, ctx: &mut Context) -> bool {
        if ctx.memory().everything_is_visible() {
            return true;
        }

        if !self.hovered || !ctx.input().pointer.has_pointer() {
            return false;
        }

        if ctx.style().interaction.show_tooltips_only_when_still && !ctx.input().pointer.is_still()
        {
            // wait for mouse to stop
            ctx.request_repaint();
            return false;
        }

        // We don't want tooltips of things while we are dragging them,
        // but we do want tooltips while holding down on an item on a touch screen.
        if ctx.input().pointer.any_down() && ctx.input().pointer.has_moved_too_much_for_a_click {
            return false;
        }

        true
    }

    /// Like `on_hover_text`, but show the text next to cursor.
    #[doc(alias = "tooltip")]
    pub fn on_hover_text_at_pointer(self, ctx: &mut Context, text: impl Into<WidgetText>) -> Self {
        self.on_hover_ui_at_pointer(ctx, |ui| {
            ui.add(crate::widgets::Label::new(text));
        })
    }

    /// Show this text if the widget was hovered (i.e. a tooltip).
    ///
    /// The text will not be visible if the widget is not enabled.
    /// For that, use [`Self::on_disabled_hover_text`] instead.
    ///
    /// If you call this multiple times the tooltips will stack underneath the previous ones.
    #[doc(alias = "tooltip")]
    pub fn on_hover_text(self, ctx: &mut Context, text: impl Into<WidgetText>) -> Self {
        self.on_hover_ui(ctx, |ui| {
            ui.add(crate::widgets::Label::new(text));
        })
    }

    /// Show this text when hovering if the widget is disabled.
    pub fn on_disabled_hover_text(self, ctx: &mut Context, text: impl Into<WidgetText>) -> Self {
        self.on_disabled_hover_ui(ctx, |ui| {
            ui.add(crate::widgets::Label::new(text));
        })
    }

    /// When hovered, use this icon for the mouse cursor.
    pub fn on_hover_cursor(self, ctx: &mut Context, cursor: CursorIcon) -> Self {
        if self.hovered() {
            ctx.output_mut().cursor_icon = cursor;
        }
        self
    }

    /// Check for more interactions (e.g. sense clicks on a [`Response`] returned from a label).
    ///
    /// Note that this call will not add any hover-effects to the widget, so when possible
    /// it is better to give the widget a [`Sense`] instead, e.g. using [`crate::Label::sense`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let response = ui.label("hello");
    /// assert!(!response.clicked()); // labels don't sense clicks by default
    /// let response = response.interact(egui::Sense::click());
    /// if response.clicked() { /* … */ }
    /// # });
    /// ```
    pub fn interact(&self, ctx: &mut Context, sense: Sense) -> Response {
        ctx.interact_with_hovered(
            self.layer_id,
            self.id,
            self.rect,
            sense,
            self.enabled,
            self.hovered,
        )
    }

    /// Adjust the scroll position until this UI becomes visible.
    ///
    /// If `align` is `None`, it'll scroll enough to bring the UI into view.
    ///
    /// See also: [`Ui::scroll_to_cursor`], [`Ui::scroll_to_rect`]. [`Ui::scroll_with_delta`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// egui::ScrollArea::vertical().show(ui, |ui| {
    ///     for i in 0..1000 {
    ///         let response = ui.button("Scroll to me");
    ///         if response.clicked() {
    ///             response.scroll_to_me(ui, Some(egui::Align::Center));
    ///         }
    ///     }
    /// });
    /// # });
    /// ```
    pub fn scroll_to_me(&self, ctx: &mut Context, align: Option<Align>) {
        ctx.frame_state_mut().scroll_target[0] = Some((self.rect.x_range(), align));
        ctx.frame_state_mut().scroll_target[1] = Some((self.rect.y_range(), align));
    }

    /// For accessibility.
    ///
    /// Call after interacting and potential calls to [`Self::mark_changed`].
    pub fn widget_info(&mut self, ctx: &mut Context, make_info: impl Fn() -> crate::WidgetInfo) {
        use crate::output::OutputEvent;
        let event = if self.clicked() {
            Some(OutputEvent::Clicked(make_info()))
        } else if self.double_clicked() {
            Some(OutputEvent::DoubleClicked(make_info()))
        } else if self.triple_clicked() {
            Some(OutputEvent::TripleClicked(make_info()))
        } else if self.gained_focus() {
            Some(OutputEvent::FocusGained(make_info()))
        } else if self.changed {
            Some(OutputEvent::ValueChanged(make_info()))
        } else {
            None
        };
        if let Some(event) = event {
            ctx.output_mut().events.push(event);
        }
    }

    /// Response to secondary clicks (right-clicks) by showing the given menu.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let response = ui.label("Right-click me!");
    /// response.context_menu(|ui| {
    ///     if ui.button("Close the menu").clicked() {
    ///         ui.close_menu();
    ///     }
    /// });
    /// # });
    /// ```
    ///
    /// See also: [`Ui::menu_button`] and [`Ui::close_menu`].
    pub fn context_menu(self, ctx: &mut Context, add_contents: impl FnOnce(&mut Ui<'_>)) -> Self {
        menu::context_menu(ctx, &mut self, add_contents);
        self
    }
}

impl Response {
    /// A logical "or" operation.
    /// For instance `a.union(b).hovered` means "was either a or b hovered?".
    ///
    /// The resulting [`Self::id`] will come from the first (`self`) argument.
    pub fn union(&self, other: Self) -> Self {
        crate::egui_assert!(
            self.layer_id == other.layer_id,
            "It makes no sense to combine Responses from two different layers"
        );
        Self {
            layer_id: self.layer_id,
            id: self.id,
            rect: self.rect.union(other.rect),
            sense: self.sense.union(other.sense),
            interact_pointer_pos: self.interact_pointer_pos.or(other.interact_pointer_pos),
            hover_pointer_pos: self.hover_pointer_pos.or(other.hover_pointer_pos),
            pointer_delta: self.pointer_delta,
            enabled: self.enabled || other.enabled,
            hovered: self.hovered || other.hovered,
            pointer_pressed: [
                self.pointer_pressed[0] || other.pointer_pressed[0],
                self.pointer_pressed[1] || other.pointer_pressed[1],
                self.pointer_pressed[2] || other.pointer_pressed[2],
                self.pointer_pressed[3] || other.pointer_pressed[3],
                self.pointer_pressed[4] || other.pointer_pressed[4],
            ],
            pointer_down: [
                self.pointer_down[0] || other.pointer_down[0],
                self.pointer_down[1] || other.pointer_down[1],
                self.pointer_down[2] || other.pointer_down[2],
                self.pointer_down[3] || other.pointer_down[3],
                self.pointer_down[4] || other.pointer_down[4],
            ],
            clicked: [
                self.clicked[0] || other.clicked[0],
                self.clicked[1] || other.clicked[1],
                self.clicked[2] || other.clicked[2],
                self.clicked[3] || other.clicked[3],
                self.clicked[4] || other.clicked[4],
            ],
            double_clicked: [
                self.double_clicked[0] || other.double_clicked[0],
                self.double_clicked[1] || other.double_clicked[1],
                self.double_clicked[2] || other.double_clicked[2],
                self.double_clicked[3] || other.double_clicked[3],
                self.double_clicked[4] || other.double_clicked[4],
            ],
            triple_clicked: [
                self.triple_clicked[0] || other.triple_clicked[0],
                self.triple_clicked[1] || other.triple_clicked[1],
                self.triple_clicked[2] || other.triple_clicked[2],
                self.triple_clicked[3] || other.triple_clicked[3],
                self.triple_clicked[4] || other.triple_clicked[4],
            ],
            dragged: self.dragged || other.dragged,
            drag_released: self.drag_released || other.drag_released,
            is_pointer_button_down_on: self.is_pointer_button_down_on
                || other.is_pointer_button_down_on,
            changed: self.changed || other.changed,
            clicked_elsewhere: self.clicked_elsewhere || other.clicked_elsewhere,
            has_focus: self.has_focus || other.has_focus,
            gained_focus: self.gained_focus || other.gained_focus,
            lost_focus: self.lost_focus || other.lost_focus,
        }
    }
}

/// To summarize the response from many widgets you can use this pattern:
///
/// ```
/// use egui::*;
/// fn draw_vec2(ui: &mut Ui, v: &mut Vec2) -> Response {
///     ui.add(DragValue::new(&mut v.x)) | ui.add(DragValue::new(&mut v.y))
/// }
/// ```
///
/// Now `draw_vec2(ui, foo).hovered` is true if either [`DragValue`](crate::DragValue) were hovered.
impl std::ops::BitOr for Response {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

/// To summarize the response from many widgets you can use this pattern:
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let (widget_a, widget_b, widget_c) = (egui::Label::new("a"), egui::Label::new("b"), egui::Label::new("c"));
/// let mut response = ui.add(widget_a);
/// response |= ui.add(widget_b);
/// response |= ui.add(widget_c);
/// if response.hovered() { ui.label("You hovered at least one of the widgets"); }
/// # });
/// ```
impl std::ops::BitOrAssign for Response {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

// ----------------------------------------------------------------------------

/// Returned when we wrap some ui-code and want to return both
/// the results of the inner function and the ui as a whole, e.g.:
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// let inner_resp = ui.horizontal(|ui| {
///     ui.label("Blah blah");
///     42
/// });
/// inner_resp.response.on_hover_text(ui.ctx, "You hovered the horizontal layout");
/// assert_eq!(inner_resp.inner, 42);
/// # });
/// ```
#[derive(Debug)]
pub struct InnerResponse<R> {
    /// What the user closure returned.
    pub inner: R,
    /// The response of the area.
    pub response: Response,
}

impl<R> InnerResponse<R> {
    #[inline]
    pub fn new(inner: R, response: Response) -> Self {
        Self { inner, response }
    }
}
