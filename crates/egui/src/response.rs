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
// TODO(emilk): we should be using bit sets instead of so many bools
#[derive(Clone)]
pub struct Response {
    // CONTEXT:
    /// Used for optionally showing a tooltip and checking for more interactions.
    pub ctx: Context,

    // IN:
    /// Which layer the widget is part of.
    pub layer_id: LayerId,

    /// The [`Id`] of the widget/area this response pertains.
    pub id: Id,

    /// The area of the screen we are talking about.
    pub rect: Rect,

    /// The senses (click and/or drag) that the widget was interested in (if any).
    pub sense: Sense,

    /// Was the widget enabled?
    /// If `false`, there was no interaction attempted (not even hover).
    #[doc(hidden)]
    pub enabled: bool,

    // OUT:
    /// The pointer is hovering above this widget or the widget was clicked/tapped this frame.
    #[doc(hidden)]
    pub hovered: bool,

    /// The widget is highlighted via a call to [`Self::highlight`] or [`Context::highlight_widget`].
    #[doc(hidden)]
    pub highlighted: bool,

    /// The pointer clicked this thing this frame.
    #[doc(hidden)]
    pub clicked: [bool; NUM_POINTER_BUTTONS],

    // TODO(emilk): `released` for sliders
    /// The thing was double-clicked.
    #[doc(hidden)]
    pub double_clicked: [bool; NUM_POINTER_BUTTONS],

    /// The thing was triple-clicked.
    pub triple_clicked: [bool; NUM_POINTER_BUTTONS],

    /// The widgets is being dragged
    #[doc(hidden)]
    pub dragged: bool,

    /// The widget was being dragged, but now it has been released.
    #[doc(hidden)]
    pub drag_released: bool,

    /// Is the pointer button currently down on this widget?
    /// This is true if the pointer is pressing down or dragging a widget
    #[doc(hidden)]
    pub is_pointer_button_down_on: bool,

    /// Where the pointer (mouse/touch) were when when this widget was clicked or dragged.
    /// `None` if the widget is not being interacted with.
    #[doc(hidden)]
    pub interact_pointer_pos: Option<Pos2>,

    /// What the underlying data changed?
    ///
    /// e.g. the slider was dragged, text was entered in a [`TextEdit`](crate::TextEdit) etc.
    /// Always `false` for something like a [`Button`](crate::Button).
    #[doc(hidden)]
    pub changed: bool,
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            ctx: _,
            layer_id,
            id,
            rect,
            sense,
            enabled,
            hovered,
            highlighted,
            clicked,
            double_clicked,
            triple_clicked,
            dragged,
            drag_released,
            is_pointer_button_down_on,
            interact_pointer_pos,
            changed,
        } = self;
        f.debug_struct("Response")
            .field("layer_id", layer_id)
            .field("id", id)
            .field("rect", rect)
            .field("sense", sense)
            .field("enabled", enabled)
            .field("hovered", hovered)
            .field("highlighted", highlighted)
            .field("clicked", clicked)
            .field("double_clicked", double_clicked)
            .field("triple_clicked", triple_clicked)
            .field("dragged", dragged)
            .field("drag_released", drag_released)
            .field("is_pointer_button_down_on", is_pointer_button_down_on)
            .field("interact_pointer_pos", interact_pointer_pos)
            .field("changed", changed)
            .finish()
    }
}

impl Response {
    /// Returns true if this widget was clicked this frame by the primary button.
    ///
    /// A click is registered when the mouse or touch is released within
    /// a certain amount of time and distance from when and where it was pressed.
    ///
    /// Note that the widget must be sensing clicks with [`Sense::click`].
    /// [`crate::Button`] senses clicks; [`crate::Label`] does not (unless you call [`crate::Label::sense`]).
    ///
    /// You can use [`Self::interact`] to sense more things *after* adding a widget.
    #[inline(always)]
    pub fn clicked(&self) -> bool {
        self.clicked[PointerButton::Primary as usize]
    }

    /// Returns true if this widget was clicked this frame by the given button.
    pub fn clicked_by(&self, button: PointerButton) -> bool {
        self.clicked[button as usize]
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

    /// Returns true if this widget was triple-clicked this frame by the primary button.
    pub fn triple_clicked(&self) -> bool {
        self.triple_clicked[PointerButton::Primary as usize]
    }

    /// Returns true if this widget was double-clicked this frame by the given button.
    pub fn double_clicked_by(&self, button: PointerButton) -> bool {
        self.double_clicked[button as usize]
    }

    /// Returns true if this widget was triple-clicked this frame by the given button.
    pub fn triple_clicked_by(&self, button: PointerButton) -> bool {
        self.triple_clicked[button as usize]
    }

    /// `true` if there was a click *outside* this widget this frame.
    pub fn clicked_elsewhere(&self) -> bool {
        // We do not use self.clicked(), because we want to catch all clicks within our frame,
        // even if we aren't clickable (or even enabled).
        // This is important for windows and such that should close then the user clicks elsewhere.
        self.ctx.input(|i| {
            let pointer = &i.pointer;

            if pointer.any_click() {
                // We detect clicks/hover on a "interact_rect" that is slightly larger than
                // self.rect. See Context::interact.
                // This means we can be hovered and clicked even though `!self.rect.contains(pos)` is true,
                // hence the extra complexity here.
                if self.hovered() {
                    false
                } else if let Some(pos) = pointer.interact_pos() {
                    !self.rect.contains(pos)
                } else {
                    false // clicked without a pointer, weird
                }
            } else {
                false
            }
        })
    }

    /// Was the widget enabled?
    /// If false, there was no interaction attempted
    /// and the widget should be drawn in a gray disabled look.
    #[inline(always)]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// The pointer is hovering above this widget or the widget was clicked/tapped this frame.
    ///
    /// Note that this is slightly different from checking `response.rect.contains(pointer_pos)`.
    /// For one, the hover rectangle is slightly larger, by half of the current item spacing
    /// (to make it easier to click things). But `hovered` also checks that no other area
    /// is covering this response rectangle.
    #[inline(always)]
    pub fn hovered(&self) -> bool {
        self.hovered
    }

    /// The widget is highlighted via a call to [`Self::highlight`] or [`Context::highlight_widget`].
    #[doc(hidden)]
    pub fn highlighted(&self) -> bool {
        self.highlighted
    }

    /// This widget has the keyboard focus (i.e. is receiving key presses).
    ///
    /// This function only returns true if the UI as a whole (e.g. window)
    /// also has the keyboard focus. That makes this function suitable
    /// for style choices, e.g. a thicker border around focused widgets.
    pub fn has_focus(&self) -> bool {
        self.ctx.input(|i| i.raw.has_focus) && self.ctx.memory(|mem| mem.has_focus(self.id))
    }

    /// True if this widget has keyboard focus this frame, but didn't last frame.
    pub fn gained_focus(&self) -> bool {
        self.ctx.memory(|mem| mem.gained_focus(self.id))
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
    /// if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
    ///     do_request(&my_text);
    /// }
    /// # });
    /// ```
    pub fn lost_focus(&self) -> bool {
        self.ctx.memory(|mem| mem.lost_focus(self.id))
    }

    /// Request that this widget get keyboard focus.
    pub fn request_focus(&self) {
        self.ctx.memory_mut(|mem| mem.request_focus(self.id));
    }

    /// Surrender keyboard focus for this widget.
    pub fn surrender_focus(&self) {
        self.ctx.memory_mut(|mem| mem.surrender_focus(self.id));
    }

    /// The widgets is being dragged.
    ///
    /// To find out which button(s), query [`crate::PointerState::button_down`]
    /// (`ui.input(|i| i.pointer.button_down(…))`).
    ///
    /// Note that the widget must be sensing drags with [`Sense::drag`].
    /// [`crate::DragValue`] senses drags; [`crate::Label`] does not (unless you call [`crate::Label::sense`]).
    ///
    /// You can use [`Self::interact`] to sense more things *after* adding a widget.
    #[inline(always)]
    pub fn dragged(&self) -> bool {
        self.dragged
    }

    pub fn dragged_by(&self, button: PointerButton) -> bool {
        self.dragged() && self.ctx.input(|i| i.pointer.button_down(button))
    }

    /// Did a drag on this widgets begin this frame?
    pub fn drag_started(&self) -> bool {
        self.dragged && self.ctx.input(|i| i.pointer.any_pressed())
    }

    /// Did a drag on this widgets by the button begin this frame?
    pub fn drag_started_by(&self, button: PointerButton) -> bool {
        self.drag_started() && self.ctx.input(|i| i.pointer.button_pressed(button))
    }

    /// The widget was being dragged, but now it has been released.
    pub fn drag_released(&self) -> bool {
        self.drag_released
    }

    /// The widget was being dragged by the button, but now it has been released.
    pub fn drag_released_by(&self, button: PointerButton) -> bool {
        self.drag_released() && self.ctx.input(|i| i.pointer.button_released(button))
    }

    /// If dragged, how many points were we dragged and in what direction?
    pub fn drag_delta(&self) -> Vec2 {
        if self.dragged() {
            self.ctx.input(|i| i.pointer.delta())
        } else {
            Vec2::ZERO
        }
    }

    /// Where the pointer (mouse/touch) were when when this widget was clicked or dragged.
    /// `None` if the widget is not being interacted with.
    pub fn interact_pointer_pos(&self) -> Option<Pos2> {
        self.interact_pointer_pos
    }

    /// If it is a good idea to show a tooltip, where is pointer?
    /// None if the pointer is outside the response area.
    pub fn hover_pos(&self) -> Option<Pos2> {
        if self.hovered() {
            self.ctx.input(|i| i.pointer.hover_pos())
        } else {
            None
        }
    }

    /// Is the pointer button currently down on this widget?
    /// This is true if the pointer is pressing down or dragging a widget
    #[inline(always)]
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
    #[inline(always)]
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
    #[inline(always)]
    pub fn mark_changed(&mut self) {
        self.changed = true;
    }

    /// Show this UI if the widget was hovered (i.e. a tooltip).
    ///
    /// The text will not be visible if the widget is not enabled.
    /// For that, use [`Self::on_disabled_hover_ui`] instead.
    ///
    /// If you call this multiple times the tooltips will stack underneath the previous ones.
    #[doc(alias = "tooltip")]
    pub fn on_hover_ui(self, add_contents: impl FnOnce(&mut Ui)) -> Self {
        if self.should_show_hover_ui() {
            crate::containers::show_tooltip_for(
                &self.ctx,
                self.id.with("__tooltip"),
                &self.rect,
                add_contents,
            );
        }
        self
    }

    /// Show this UI when hovering if the widget is disabled.
    pub fn on_disabled_hover_ui(self, add_contents: impl FnOnce(&mut Ui)) -> Self {
        if !self.enabled && self.ctx.rect_contains_pointer(self.layer_id, self.rect) {
            crate::containers::show_tooltip_for(
                &self.ctx,
                self.id.with("__tooltip"),
                &self.rect,
                add_contents,
            );
        }
        self
    }

    /// Like `on_hover_ui`, but show the ui next to cursor.
    pub fn on_hover_ui_at_pointer(self, add_contents: impl FnOnce(&mut Ui)) -> Self {
        if self.should_show_hover_ui() {
            crate::containers::show_tooltip_at_pointer(
                &self.ctx,
                self.id.with("__tooltip"),
                add_contents,
            );
        }
        self
    }

    /// Was the tooltip open last frame?
    pub fn is_tooltip_open(&self) -> bool {
        crate::popup::was_tooltip_open_last_frame(&self.ctx, self.id.with("__tooltip"))
    }

    fn should_show_hover_ui(&self) -> bool {
        if self.ctx.memory(|mem| mem.everything_is_visible()) {
            return true;
        }

        if !self.hovered || !self.ctx.input(|i| i.pointer.has_pointer()) {
            return false;
        }

        if self.ctx.style().interaction.show_tooltips_only_when_still {
            // We only show the tooltip when the mouse pointer is still,
            // but once shown we keep showing it until the mouse leaves the parent.

            if !self.ctx.input(|i| i.pointer.is_still()) && !self.is_tooltip_open() {
                // wait for mouse to stop
                self.ctx.request_repaint();
                return false;
            }
        }

        // We don't want tooltips of things while we are dragging them,
        // but we do want tooltips while holding down on an item on a touch screen.
        if self
            .ctx
            .input(|i| i.pointer.any_down() && i.pointer.has_moved_too_much_for_a_click)
        {
            return false;
        }

        true
    }

    /// Like `on_hover_text`, but show the text next to cursor.
    #[doc(alias = "tooltip")]
    pub fn on_hover_text_at_pointer(self, text: impl Into<WidgetText>) -> Self {
        self.on_hover_ui_at_pointer(|ui| {
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
    pub fn on_hover_text(self, text: impl Into<WidgetText>) -> Self {
        self.on_hover_ui(|ui| {
            ui.add(crate::widgets::Label::new(text));
        })
    }

    /// Highlight this widget, to make it look like it is hovered, even if it isn't.
    ///
    /// The highlight takes on frame to take effect if you call this after the widget has been fully rendered.
    ///
    /// See also [`Context::highlight_widget`].
    pub fn highlight(mut self) -> Self {
        self.ctx.highlight_widget(self.id);
        self.highlighted = true;
        self
    }

    /// Show this text when hovering if the widget is disabled.
    pub fn on_disabled_hover_text(self, text: impl Into<WidgetText>) -> Self {
        self.on_disabled_hover_ui(|ui| {
            ui.add(crate::widgets::Label::new(text));
        })
    }

    /// When hovered, use this icon for the mouse cursor.
    pub fn on_hover_cursor(self, cursor: CursorIcon) -> Self {
        if self.hovered() {
            self.ctx.set_cursor_icon(cursor);
        }
        self
    }

    /// When hovered or dragged, use this icon for the mouse cursor.
    pub fn on_hover_and_drag_cursor(self, cursor: CursorIcon) -> Self {
        if self.hovered() || self.dragged() {
            self.ctx.set_cursor_icon(cursor);
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
    #[must_use]
    pub fn interact(&self, sense: Sense) -> Self {
        self.ctx.interact_with_hovered(
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
    ///             response.scroll_to_me(Some(egui::Align::Center));
    ///         }
    ///     }
    /// });
    /// # });
    /// ```
    pub fn scroll_to_me(&self, align: Option<Align>) {
        self.ctx.frame_state_mut(|state| {
            state.scroll_target[0] = Some((self.rect.x_range(), align));
            state.scroll_target[1] = Some((self.rect.y_range(), align));
        });
    }

    /// For accessibility.
    ///
    /// Call after interacting and potential calls to [`Self::mark_changed`].
    pub fn widget_info(&self, make_info: impl Fn() -> crate::WidgetInfo) {
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
            self.output_event(event);
        } else {
            #[cfg(feature = "accesskit")]
            self.ctx.accesskit_node_builder(self.id, |builder| {
                self.fill_accesskit_node_from_widget_info(builder, make_info());
            });
        }
    }

    pub fn output_event(&self, event: crate::output::OutputEvent) {
        #[cfg(feature = "accesskit")]
        self.ctx.accesskit_node_builder(self.id, |builder| {
            self.fill_accesskit_node_from_widget_info(builder, event.widget_info().clone());
        });
        self.ctx.output_mut(|o| o.events.push(event));
    }

    #[cfg(feature = "accesskit")]
    pub(crate) fn fill_accesskit_node_common(&self, builder: &mut accesskit::NodeBuilder) {
        builder.set_bounds(accesskit::Rect {
            x0: self.rect.min.x.into(),
            y0: self.rect.min.y.into(),
            x1: self.rect.max.x.into(),
            y1: self.rect.max.y.into(),
        });
        if self.sense.focusable {
            builder.add_action(accesskit::Action::Focus);
        }
        if self.sense.click && builder.default_action_verb().is_none() {
            builder.set_default_action_verb(accesskit::DefaultActionVerb::Click);
        }
    }

    #[cfg(feature = "accesskit")]
    fn fill_accesskit_node_from_widget_info(
        &self,
        builder: &mut accesskit::NodeBuilder,
        info: crate::WidgetInfo,
    ) {
        use crate::WidgetType;
        use accesskit::{CheckedState, Role};

        self.fill_accesskit_node_common(builder);
        builder.set_role(match info.typ {
            WidgetType::Label => Role::StaticText,
            WidgetType::Link => Role::Link,
            WidgetType::TextEdit => Role::TextField,
            WidgetType::Button | WidgetType::ImageButton | WidgetType::CollapsingHeader => {
                Role::Button
            }
            WidgetType::Checkbox => Role::CheckBox,
            WidgetType::RadioButton => Role::RadioButton,
            WidgetType::SelectableLabel => Role::ToggleButton,
            WidgetType::ComboBox => Role::PopupButton,
            WidgetType::Slider => Role::Slider,
            WidgetType::DragValue => Role::SpinButton,
            WidgetType::ColorButton => Role::ColorWell,
            WidgetType::Other => Role::Unknown,
        });
        if let Some(label) = info.label {
            builder.set_name(label);
        }
        if let Some(value) = info.current_text_value {
            builder.set_value(value);
        }
        if let Some(value) = info.value {
            builder.set_numeric_value(value);
        }
        if let Some(selected) = info.selected {
            builder.set_checked_state(if selected {
                CheckedState::True
            } else {
                CheckedState::False
            });
        }
    }

    /// Associate a label with a control for accessibility.
    ///
    /// # Example
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut text = "Arthur".to_string();
    /// ui.horizontal(|ui| {
    ///     let label = ui.label("Your name: ");
    ///     ui.text_edit_singleline(&mut text).labelled_by(label.id);
    /// });
    /// # });
    /// ```
    pub fn labelled_by(self, id: Id) -> Self {
        #[cfg(feature = "accesskit")]
        self.ctx.accesskit_node_builder(self.id, |builder| {
            builder.push_labelled_by(id.accesskit_id());
        });
        #[cfg(not(feature = "accesskit"))]
        {
            let _ = id;
        }

        self
    }

    /// Response to secondary clicks (right-clicks) by showing the given menu.
    ///
    /// ```
    /// # use egui::{Label, Sense};
    /// # egui::__run_test_ui(|ui| {
    /// let response = ui.add(Label::new("Right-click me!").sense(Sense::click()));
    /// response.context_menu(|ui| {
    ///     if ui.button("Close the menu").clicked() {
    ///         ui.close_menu();
    ///     }
    /// });
    /// # });
    /// ```
    ///
    /// See also: [`Ui::menu_button`] and [`Ui::close_menu`].
    pub fn context_menu(self, add_contents: impl FnOnce(&mut Ui)) -> Self {
        menu::context_menu(&self, add_contents);
        self
    }
}

impl Response {
    /// A logical "or" operation.
    /// For instance `a.union(b).hovered` means "was either a or b hovered?".
    ///
    /// The resulting [`Self::id`] will come from the first (`self`) argument.
    pub fn union(&self, other: Self) -> Self {
        assert!(self.ctx == other.ctx);
        crate::egui_assert!(
            self.layer_id == other.layer_id,
            "It makes no sense to combine Responses from two different layers"
        );
        Self {
            ctx: other.ctx,
            layer_id: self.layer_id,
            id: self.id,
            rect: self.rect.union(other.rect),
            sense: self.sense.union(other.sense),
            enabled: self.enabled || other.enabled,
            hovered: self.hovered || other.hovered,
            highlighted: self.highlighted || other.highlighted,
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
            interact_pointer_pos: self.interact_pointer_pos.or(other.interact_pointer_pos),
            changed: self.changed || other.changed,
        }
    }
}

impl Response {
    /// Returns a response with a modified [`Self::rect`].
    pub fn with_new_rect(self, rect: Rect) -> Self {
        Self { rect, ..self }
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
/// inner_resp.response.on_hover_text("You hovered the horizontal layout");
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
