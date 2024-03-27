use std::sync::Arc;

use epaint::text::{cursor::*, Galley, LayoutJob};

use crate::{
    os::OperatingSystem,
    output::OutputEvent,
    text_selection::{
        text_cursor_state::cursor_rect,
        visuals::{paint_cursor, paint_text_selection},
        CCursorRange, CursorRange,
    },
    *,
};

use super::{TextEditOutput, TextEditState};

/// A text region that the user can edit the contents of.
///
/// See also [`Ui::text_edit_singleline`] and [`Ui::text_edit_multiline`].
///
/// Example:
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_string = String::new();
/// let response = ui.add(egui::TextEdit::singleline(&mut my_string));
/// if response.changed() {
///     // …
/// }
/// if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
///     // …
/// }
/// # });
/// ```
///
/// To fill an [`Ui`] with a [`TextEdit`] use [`Ui::add_sized`]:
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_string = String::new();
/// ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut my_string));
/// # });
/// ```
///
///
/// You can also use [`TextEdit`] to show text that can be selected, but not edited.
/// To do so, pass in a `&mut` reference to a `&str`, for instance:
///
/// ```
/// fn selectable_text(ui: &mut egui::Ui, mut text: &str) {
///     ui.add(egui::TextEdit::multiline(&mut text));
/// }
/// ```
///
/// ## Advanced usage
/// See [`TextEdit::show`].
///
/// ## Other
/// The background color of a [`TextEdit`] is [`Visuals::extreme_bg_color`].
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct TextEdit<'t> {
    text: &'t mut dyn TextBuffer,
    hint_text: WidgetText,
    id: Option<Id>,
    id_source: Option<Id>,
    font_selection: FontSelection,
    text_color: Option<Color32>,
    layouter: Option<&'t mut dyn FnMut(&Ui, &str, f32) -> Arc<Galley>>,
    password: bool,
    frame: bool,
    margin: Margin,
    multiline: bool,
    interactive: bool,
    desired_width: Option<f32>,
    desired_height_rows: usize,
    event_filter: EventFilter,
    cursor_at_end: bool,
    min_size: Vec2,
    align: Align2,
    clip_text: bool,
    char_limit: usize,
    return_key: KeyboardShortcut,
}

impl<'t> WidgetWithState for TextEdit<'t> {
    type State = TextEditState;
}

impl<'t> TextEdit<'t> {
    pub fn load_state(ctx: &Context, id: Id) -> Option<TextEditState> {
        TextEditState::load(ctx, id)
    }

    pub fn store_state(ctx: &Context, id: Id, state: TextEditState) {
        state.store(ctx, id);
    }
}

impl<'t> TextEdit<'t> {
    /// No newlines (`\n`) allowed. Pressing enter key will result in the [`TextEdit`] losing focus (`response.lost_focus`).
    pub fn singleline(text: &'t mut dyn TextBuffer) -> Self {
        Self {
            desired_height_rows: 1,
            multiline: false,
            clip_text: true,
            ..Self::multiline(text)
        }
    }

    /// A [`TextEdit`] for multiple lines. Pressing enter key will create a new line by default (can be changed with [`return_key`](TextEdit::return_key)).
    pub fn multiline(text: &'t mut dyn TextBuffer) -> Self {
        Self {
            text,
            hint_text: Default::default(),
            id: None,
            id_source: None,
            font_selection: Default::default(),
            text_color: None,
            layouter: None,
            password: false,
            frame: true,
            margin: Margin::symmetric(4.0, 2.0),
            multiline: true,
            interactive: true,
            desired_width: None,
            desired_height_rows: 4,
            event_filter: EventFilter {
                // moving the cursor is really important
                horizontal_arrows: true,
                vertical_arrows: true,
                tab: false, // tab is used to change focus, not to insert a tab character
                ..Default::default()
            },
            cursor_at_end: true,
            min_size: Vec2::ZERO,
            align: Align2::LEFT_TOP,
            clip_text: false,
            char_limit: usize::MAX,
            return_key: KeyboardShortcut::new(Modifiers::NONE, Key::Enter),
        }
    }

    /// Build a [`TextEdit`] focused on code editing.
    /// By default it comes with:
    /// - monospaced font
    /// - focus lock (tab will insert a tab character instead of moving focus)
    pub fn code_editor(self) -> Self {
        self.font(TextStyle::Monospace).lock_focus(true)
    }

    /// Use if you want to set an explicit [`Id`] for this widget.
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// A source for the unique [`Id`], e.g. `.id_source("second_text_edit_field")` or `.id_source(loop_index)`.
    #[inline]
    pub fn id_source(mut self, id_source: impl std::hash::Hash) -> Self {
        self.id_source = Some(Id::new(id_source));
        self
    }

    /// Show a faint hint text when the text field is empty.
    ///
    /// If the hint text needs to be persisted even when the text field has input,
    /// the following workaround can be used:
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_string = String::new();
    /// # use egui::{ Color32, FontId };
    /// let text_edit = egui::TextEdit::multiline(&mut my_string)
    ///     .desired_width(f32::INFINITY);
    /// let output = text_edit.show(ui);
    /// let painter = ui.painter_at(output.response.rect);
    /// let text_color = Color32::from_rgba_premultiplied(100, 100, 100, 100);
    /// let galley = painter.layout(
    ///     String::from("Enter text"),
    ///     FontId::default(),
    ///     text_color,
    ///     f32::INFINITY
    /// );
    /// painter.galley(output.galley_pos, galley, text_color);
    /// # });
    /// ```
    #[inline]
    pub fn hint_text(mut self, hint_text: impl Into<WidgetText>) -> Self {
        self.hint_text = hint_text.into();
        self
    }

    /// If true, hide the letters from view and prevent copying from the field.
    #[inline]
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Pick a [`FontId`] or [`TextStyle`].
    #[inline]
    pub fn font(mut self, font_selection: impl Into<FontSelection>) -> Self {
        self.font_selection = font_selection.into();
        self
    }

    #[inline]
    pub fn text_color(mut self, text_color: Color32) -> Self {
        self.text_color = Some(text_color);
        self
    }

    #[inline]
    pub fn text_color_opt(mut self, text_color: Option<Color32>) -> Self {
        self.text_color = text_color;
        self
    }

    /// Override how text is being shown inside the [`TextEdit`].
    ///
    /// This can be used to implement things like syntax highlighting.
    ///
    /// This function will be called at least once per frame,
    /// so it is strongly suggested that you cache the results of any syntax highlighter
    /// so as not to waste CPU highlighting the same string every frame.
    ///
    /// The arguments is the enclosing [`Ui`] (so you can access e.g. [`Ui::fonts`]),
    /// the text and the wrap width.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_code = String::new();
    /// # fn my_memoized_highlighter(s: &str) -> egui::text::LayoutJob { Default::default() }
    /// let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
    ///     let mut layout_job: egui::text::LayoutJob = my_memoized_highlighter(string);
    ///     layout_job.wrap.max_width = wrap_width;
    ///     ui.fonts(|f| f.layout_job(layout_job))
    /// };
    /// ui.add(egui::TextEdit::multiline(&mut my_code).layouter(&mut layouter));
    /// # });
    /// ```
    #[inline]
    pub fn layouter(mut self, layouter: &'t mut dyn FnMut(&Ui, &str, f32) -> Arc<Galley>) -> Self {
        self.layouter = Some(layouter);

        self
    }

    /// Default is `true`. If set to `false` then you cannot interact with the text (neither edit or select it).
    ///
    /// Consider using [`Ui::add_enabled`] instead to also give the [`TextEdit`] a greyed out look.
    #[inline]
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Default is `true`. If set to `false` there will be no frame showing that this is editable text!
    #[inline]
    pub fn frame(mut self, frame: bool) -> Self {
        self.frame = frame;
        self
    }

    /// Set margin of text. Default is `Margin::symmetric(4.0, 2.0)`
    #[inline]
    pub fn margin(mut self, margin: impl Into<Margin>) -> Self {
        self.margin = margin.into();
        self
    }

    /// Set to 0.0 to keep as small as possible.
    /// Set to [`f32::INFINITY`] to take up all available space (i.e. disable automatic word wrap).
    #[inline]
    pub fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
    }

    /// Set the number of rows to show by default.
    /// The default for singleline text is `1`.
    /// The default for multiline text is `4`.
    #[inline]
    pub fn desired_rows(mut self, desired_height_rows: usize) -> Self {
        self.desired_height_rows = desired_height_rows;
        self
    }

    /// When `false` (default), pressing TAB will move focus
    /// to the next widget.
    ///
    /// When `true`, the widget will keep the focus and pressing TAB
    /// will insert the `'\t'` character.
    #[inline]
    pub fn lock_focus(mut self, tab_will_indent: bool) -> Self {
        self.event_filter.tab = tab_will_indent;
        self
    }

    /// When `true` (default), the cursor will initially be placed at the end of the text.
    ///
    /// When `false`, the cursor will initially be placed at the beginning of the text.
    #[inline]
    pub fn cursor_at_end(mut self, b: bool) -> Self {
        self.cursor_at_end = b;
        self
    }

    /// When `true` (default), overflowing text will be clipped.
    ///
    /// When `false`, widget width will expand to make all text visible.
    ///
    /// This only works for singleline [`TextEdit`].
    #[inline]
    pub fn clip_text(mut self, b: bool) -> Self {
        // always show everything in multiline
        if !self.multiline {
            self.clip_text = b;
        }
        self
    }

    /// Sets the limit for the amount of characters can be entered
    ///
    /// This only works for singleline [`TextEdit`]
    #[inline]
    pub fn char_limit(mut self, limit: usize) -> Self {
        self.char_limit = limit;
        self
    }

    /// Set the horizontal align of the inner text.
    #[inline]
    pub fn horizontal_align(mut self, align: Align) -> Self {
        self.align.0[0] = align;
        self
    }

    /// Set the vertical align of the inner text.
    #[inline]
    pub fn vertical_align(mut self, align: Align) -> Self {
        self.align.0[1] = align;
        self
    }

    /// Set the minimum size of the [`TextEdit`].
    #[inline]
    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    /// Set the return key combination.
    ///
    /// This combination will cause a newline on multiline,
    /// whereas on singleline it will cause the widget to lose focus.
    #[inline]
    pub fn return_key(mut self, return_key: KeyboardShortcut) -> Self {
        self.return_key = return_key;
        self
    }
}

// ----------------------------------------------------------------------------

impl<'t> Widget for TextEdit<'t> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

impl<'t> TextEdit<'t> {
    /// Show the [`TextEdit`], returning a rich [`TextEditOutput`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_string = String::new();
    /// let output = egui::TextEdit::singleline(&mut my_string).show(ui);
    /// if let Some(text_cursor_range) = output.cursor_range {
    ///     use egui::TextBuffer as _;
    ///     let selected_chars = text_cursor_range.as_sorted_char_range();
    ///     let selected_text = my_string.char_range(selected_chars);
    ///     ui.label("Selected text: ");
    ///     ui.monospace(selected_text);
    /// }
    /// # });
    /// ```
    pub fn show(self, ui: &mut Ui) -> TextEditOutput {
        let is_mutable = self.text.is_mutable();
        let frame = self.frame;
        let interactive = self.interactive;
        let where_to_put_background = ui.painter().add(Shape::Noop);

        let margin = self.margin;
        let available = ui.available_rect_before_wrap();
        let max_rect = margin.shrink_rect(available);
        let mut content_ui = ui.child_ui(max_rect, *ui.layout());

        let mut output = self.show_content(&mut content_ui);

        let id = output.response.id;
        let frame_rect = margin.expand_rect(output.response.rect);
        ui.allocate_space(frame_rect.size());
        if interactive {
            output.response |= ui.interact(frame_rect, id, Sense::click());
        }
        if output.response.clicked() && !output.response.lost_focus() {
            ui.memory_mut(|mem| mem.request_focus(output.response.id));
        }

        if frame {
            let visuals = ui.style().interact(&output.response);
            let frame_rect = frame_rect.expand(visuals.expansion);
            let shape = if is_mutable {
                if output.response.has_focus() {
                    epaint::RectShape::new(
                        frame_rect,
                        visuals.rounding,
                        ui.visuals().extreme_bg_color,
                        ui.visuals().selection.stroke,
                    )
                } else {
                    epaint::RectShape::new(
                        frame_rect,
                        visuals.rounding,
                        ui.visuals().extreme_bg_color,
                        visuals.bg_stroke, // TODO(emilk): we want to show something here, or a text-edit field doesn't "pop".
                    )
                }
            } else {
                let visuals = &ui.style().visuals.widgets.inactive;
                epaint::RectShape::stroke(
                    frame_rect,
                    visuals.rounding,
                    visuals.bg_stroke, // TODO(emilk): we want to show something here, or a text-edit field doesn't "pop".
                )
            };

            ui.painter().set(where_to_put_background, shape);
        }

        output
    }

    fn show_content(self, ui: &mut Ui) -> TextEditOutput {
        let TextEdit {
            text,
            hint_text,
            id,
            id_source,
            font_selection,
            text_color,
            layouter,
            password,
            frame: _,
            margin,
            multiline,
            interactive,
            desired_width,
            desired_height_rows,
            event_filter,
            cursor_at_end,
            min_size,
            align,
            clip_text,
            char_limit,
            return_key,
        } = self;

        let text_color = text_color
            .or(ui.visuals().override_text_color)
            // .unwrap_or_else(|| ui.style().interact(&response).text_color()); // too bright
            .unwrap_or_else(|| ui.visuals().widgets.inactive.text_color());

        let prev_text = text.as_str().to_owned();

        let font_id = font_selection.resolve(ui.style());
        let row_height = ui.fonts(|f| f.row_height(&font_id));
        const MIN_WIDTH: f32 = 24.0; // Never make a [`TextEdit`] more narrow than this.
        let available_width = ui.available_width().at_least(MIN_WIDTH);
        let desired_width = desired_width.unwrap_or_else(|| ui.spacing().text_edit_width);
        let wrap_width = if ui.layout().horizontal_justify() {
            available_width
        } else {
            desired_width.min(available_width)
        };

        let font_id_clone = font_id.clone();
        let mut default_layouter = move |ui: &Ui, text: &str, wrap_width: f32| {
            let text = mask_if_password(password, text);
            let layout_job = if multiline {
                LayoutJob::simple(text, font_id_clone.clone(), text_color, wrap_width)
            } else {
                LayoutJob::simple_singleline(text, font_id_clone.clone(), text_color)
            };
            ui.fonts(|f| f.layout_job(layout_job))
        };

        let layouter = layouter.unwrap_or(&mut default_layouter);

        let mut galley = layouter(ui, text.as_str(), wrap_width);

        let desired_width = if clip_text {
            wrap_width // visual clipping with scroll in singleline input.
        } else {
            galley.size().x.max(wrap_width)
        };
        let desired_height = (desired_height_rows.at_least(1) as f32) * row_height;
        let at_least = min_size - margin.sum();
        let desired_size =
            vec2(desired_width, galley.size().y.max(desired_height)).at_least(at_least);

        let (auto_id, rect) = ui.allocate_space(desired_size);

        let id = id.unwrap_or_else(|| {
            if let Some(id_source) = id_source {
                ui.make_persistent_id(id_source)
            } else {
                auto_id // Since we are only storing the cursor a persistent Id is not super important
            }
        });
        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();

        // On touch screens (e.g. mobile in `eframe` web), should
        // dragging select text, or scroll the enclosing [`ScrollArea`] (if any)?
        // Since currently copying selected text in not supported on `eframe` web,
        // we prioritize touch-scrolling:
        let allow_drag_to_select =
            ui.input(|i| !i.has_touch_screen()) || ui.memory(|mem| mem.has_focus(id));

        let sense = if interactive {
            if allow_drag_to_select {
                Sense::click_and_drag()
            } else {
                Sense::click()
            }
        } else {
            Sense::hover()
        };
        let mut response = ui.interact(rect, id, sense);
        let text_clip_rect = rect;
        let painter = ui.painter_at(text_clip_rect.expand(1.0)); // expand to avoid clipping cursor

        if interactive {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                if response.hovered() && text.is_mutable() {
                    ui.output_mut(|o| o.mutable_text_under_cursor = true);
                }

                // TODO(emilk): drag selected text to either move or clone (ctrl on windows, alt on mac)

                let singleline_offset = vec2(state.singleline_offset, 0.0);
                let cursor_at_pointer =
                    galley.cursor_from_pos(pointer_pos - response.rect.min + singleline_offset);

                if ui.visuals().text_cursor_preview
                    && response.hovered()
                    && ui.input(|i| i.pointer.is_moving())
                {
                    // preview:
                    let cursor_rect =
                        cursor_rect(response.rect.min, &galley, &cursor_at_pointer, row_height);
                    paint_cursor(&painter, ui.visuals(), cursor_rect);
                }

                let is_being_dragged = ui.ctx().is_being_dragged(response.id);
                let did_interact = state.cursor.pointer_interaction(
                    ui,
                    &response,
                    cursor_at_pointer,
                    &galley,
                    is_being_dragged,
                );

                if did_interact {
                    ui.memory_mut(|mem| mem.request_focus(response.id));
                }
            }
        }

        if interactive && response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Text);
        }

        let mut cursor_range = None;
        let prev_cursor_range = state.cursor.range(&galley);
        if interactive && ui.memory(|mem| mem.has_focus(id)) {
            ui.memory_mut(|mem| mem.set_focus_lock_filter(id, event_filter));

            let default_cursor_range = if cursor_at_end {
                CursorRange::one(galley.end())
            } else {
                CursorRange::default()
            };

            let (changed, new_cursor_range) = events(
                ui,
                &mut state,
                text,
                &mut galley,
                layouter,
                id,
                wrap_width,
                multiline,
                password,
                default_cursor_range,
                char_limit,
                event_filter,
                return_key,
            );

            if changed {
                response.mark_changed();
            }
            cursor_range = Some(new_cursor_range);
        }

        let mut galley_pos = align
            .align_size_within_rect(galley.size(), response.rect)
            .intersect(response.rect) // limit pos to the response rect area
            .min;
        let align_offset = response.rect.left() - galley_pos.x;

        // Visual clipping for singleline text editor with text larger than width
        if clip_text && align_offset == 0.0 {
            let cursor_pos = match (cursor_range, ui.memory(|mem| mem.has_focus(id))) {
                (Some(cursor_range), true) => galley.pos_from_cursor(&cursor_range.primary).min.x,
                _ => 0.0,
            };

            let mut offset_x = state.singleline_offset;
            let visible_range = offset_x..=offset_x + desired_size.x;

            if !visible_range.contains(&cursor_pos) {
                if cursor_pos < *visible_range.start() {
                    offset_x = cursor_pos;
                } else {
                    offset_x = cursor_pos - desired_size.x;
                }
            }

            offset_x = offset_x
                .at_most(galley.size().x - desired_size.x)
                .at_least(0.0);

            state.singleline_offset = offset_x;
            galley_pos -= vec2(offset_x, 0.0);
        } else {
            state.singleline_offset = align_offset;
        }

        let selection_changed = if let (Some(cursor_range), Some(prev_cursor_range)) =
            (cursor_range, prev_cursor_range)
        {
            prev_cursor_range.as_ccursor_range() != cursor_range.as_ccursor_range()
        } else {
            false
        };

        if ui.is_rect_visible(rect) {
            painter.galley(galley_pos, galley.clone(), text_color);

            if text.as_str().is_empty() && !hint_text.is_empty() {
                let hint_text_color = ui.visuals().weak_text_color();
                let galley = if multiline {
                    hint_text.into_galley(ui, Some(true), desired_size.x, font_id)
                } else {
                    hint_text.into_galley(ui, Some(false), f32::INFINITY, font_id)
                };
                painter.galley(response.rect.min, galley, hint_text_color);
            }

            if ui.memory(|mem| mem.has_focus(id)) {
                if let Some(cursor_range) = state.cursor.range(&galley) {
                    // We paint the cursor on top of the text, in case
                    // the text galley has backgrounds (as e.g. `code` snippets in markup do).
                    paint_text_selection(
                        &painter,
                        ui.visuals(),
                        galley_pos,
                        &galley,
                        &cursor_range,
                        None,
                    );

                    let primary_cursor_rect =
                        cursor_rect(galley_pos, &galley, &cursor_range.primary, row_height);

                    let is_fully_visible = ui.clip_rect().contains_rect(rect); // TODO(emilk): remove this HACK workaround for https://github.com/emilk/egui/issues/1531
                    if (response.changed || selection_changed) && !is_fully_visible {
                        // Scroll to keep primary cursor in view:
                        ui.scroll_to_rect(primary_cursor_rect, None);
                    }

                    if text.is_mutable() {
                        paint_cursor(&painter, ui.visuals(), primary_cursor_rect);

                        if interactive {
                            // For IME, so only set it when text is editable and visible!
                            ui.ctx().output_mut(|o| {
                                o.ime = Some(crate::output::IMEOutput {
                                    rect,
                                    cursor_rect: primary_cursor_rect,
                                });
                            });
                        }
                    }
                }
            }
        }

        state.clone().store(ui.ctx(), id);

        if response.changed {
            response.widget_info(|| {
                WidgetInfo::text_edit(
                    mask_if_password(password, prev_text.as_str()),
                    mask_if_password(password, text.as_str()),
                )
            });
        } else if selection_changed {
            let cursor_range = cursor_range.unwrap();
            let char_range =
                cursor_range.primary.ccursor.index..=cursor_range.secondary.ccursor.index;
            let info = WidgetInfo::text_selection_changed(
                char_range,
                mask_if_password(password, text.as_str()),
            );
            response.output_event(OutputEvent::TextSelectionChanged(info));
        } else {
            response.widget_info(|| {
                WidgetInfo::text_edit(
                    mask_if_password(password, prev_text.as_str()),
                    mask_if_password(password, text.as_str()),
                )
            });
        }

        #[cfg(feature = "accesskit")]
        {
            let role = if password {
                accesskit::Role::PasswordInput
            } else if multiline {
                accesskit::Role::MultilineTextInput
            } else {
                accesskit::Role::TextInput
            };

            crate::text_selection::accesskit_text::update_accesskit_for_text_widget(
                ui.ctx(),
                id,
                cursor_range,
                role,
                galley_pos,
                &galley,
            );
        }

        TextEditOutput {
            response,
            galley,
            galley_pos,
            text_clip_rect,
            state,
            cursor_range,
        }
    }
}

fn mask_if_password(is_password: bool, text: &str) -> String {
    fn mask_password(text: &str) -> String {
        std::iter::repeat(epaint::text::PASSWORD_REPLACEMENT_CHAR)
            .take(text.chars().count())
            .collect::<String>()
    }

    if is_password {
        mask_password(text)
    } else {
        text.to_owned()
    }
}

// ----------------------------------------------------------------------------

/// Check for (keyboard) events to edit the cursor and/or text.
#[allow(clippy::too_many_arguments)]
fn events(
    ui: &crate::Ui,
    state: &mut TextEditState,
    text: &mut dyn TextBuffer,
    galley: &mut Arc<Galley>,
    layouter: &mut dyn FnMut(&Ui, &str, f32) -> Arc<Galley>,
    id: Id,
    wrap_width: f32,
    multiline: bool,
    password: bool,
    default_cursor_range: CursorRange,
    char_limit: usize,
    event_filter: EventFilter,
    return_key: KeyboardShortcut,
) -> (bool, CursorRange) {
    let os = ui.ctx().os();

    let mut cursor_range = state.cursor.range(galley).unwrap_or(default_cursor_range);

    // We feed state to the undoer both before and after handling input
    // so that the undoer creates automatic saves even when there are no events for a while.
    state.undoer.lock().feed_state(
        ui.input(|i| i.time),
        &(cursor_range.as_ccursor_range(), text.as_str().to_owned()),
    );

    let copy_if_not_password = |ui: &Ui, text: String| {
        if !password {
            ui.ctx().copy_text(text);
        }
    };

    let mut any_change = false;

    let events = ui.input(|i| i.filtered_events(&event_filter));
    for event in &events {
        let did_mutate_text = match event {
            // First handle events that only changes the selection cursor, not the text:
            event if cursor_range.on_event(os, event, galley, id) => None,

            Event::Copy => {
                if cursor_range.is_empty() {
                    copy_if_not_password(ui, text.as_str().to_owned());
                } else {
                    copy_if_not_password(ui, cursor_range.slice_str(text.as_str()).to_owned());
                }
                None
            }
            Event::Cut => {
                if cursor_range.is_empty() {
                    copy_if_not_password(ui, text.take());
                    Some(CCursorRange::default())
                } else {
                    copy_if_not_password(ui, cursor_range.slice_str(text.as_str()).to_owned());
                    Some(CCursorRange::one(text.delete_selected(&cursor_range)))
                }
            }
            Event::Paste(text_to_insert) => {
                if !text_to_insert.is_empty() {
                    let mut ccursor = text.delete_selected(&cursor_range);

                    text.insert_text_at(&mut ccursor, text_to_insert, char_limit);

                    Some(CCursorRange::one(ccursor))
                } else {
                    None
                }
            }
            Event::Text(text_to_insert) => {
                // Newlines are handled by `Key::Enter`.
                if !text_to_insert.is_empty() && text_to_insert != "\n" && text_to_insert != "\r" {
                    let mut ccursor = text.delete_selected(&cursor_range);

                    text.insert_text_at(&mut ccursor, text_to_insert, char_limit);

                    Some(CCursorRange::one(ccursor))
                } else {
                    None
                }
            }
            Event::Key {
                key: Key::Tab,
                pressed: true,
                modifiers,
                ..
            } if multiline => {
                let mut ccursor = text.delete_selected(&cursor_range);
                if modifiers.shift {
                    // TODO(emilk): support removing indentation over a selection?
                    text.decrease_indentation(&mut ccursor);
                } else {
                    text.insert_text_at(&mut ccursor, "\t", char_limit);
                }
                Some(CCursorRange::one(ccursor))
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if *key == return_key.logical_key
                && modifiers.matches_logically(return_key.modifiers) =>
            {
                if multiline {
                    let mut ccursor = text.delete_selected(&cursor_range);
                    text.insert_text_at(&mut ccursor, "\n", char_limit);
                    // TODO(emilk): if code editor, auto-indent by same leading tabs, + one if the lines end on an opening bracket
                    Some(CCursorRange::one(ccursor))
                } else {
                    ui.memory_mut(|mem| mem.surrender_focus(id)); // End input with enter
                    break;
                }
            }
            Event::Key {
                key: Key::Z,
                pressed: true,
                modifiers,
                ..
            } if modifiers.matches_logically(Modifiers::COMMAND) => {
                if let Some((undo_ccursor_range, undo_txt)) = state
                    .undoer
                    .lock()
                    .undo(&(cursor_range.as_ccursor_range(), text.as_str().to_owned()))
                {
                    text.replace_with(undo_txt);
                    Some(*undo_ccursor_range)
                } else {
                    None
                }
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if (modifiers.matches_logically(Modifiers::COMMAND) && *key == Key::Y)
                || (modifiers.matches_logically(Modifiers::SHIFT | Modifiers::COMMAND)
                    && *key == Key::Z) =>
            {
                if let Some((redo_ccursor_range, redo_txt)) = state
                    .undoer
                    .lock()
                    .redo(&(cursor_range.as_ccursor_range(), text.as_str().to_owned()))
                {
                    text.replace_with(redo_txt);
                    Some(*redo_ccursor_range)
                } else {
                    None
                }
            }

            Event::Key {
                modifiers,
                key,
                pressed: true,
                ..
            } => check_for_mutating_key_press(os, &mut cursor_range, text, galley, modifiers, *key),

            Event::CompositionStart => {
                state.has_ime = true;
                None
            }

            Event::CompositionUpdate(text_mark) => {
                // empty prediction can be produced when user press backspace
                // or escape during ime. We should clear current text.
                if text_mark != "\n" && text_mark != "\r" && state.has_ime {
                    let mut ccursor = text.delete_selected(&cursor_range);
                    let start_cursor = ccursor;
                    if !text_mark.is_empty() {
                        text.insert_text_at(&mut ccursor, text_mark, char_limit);
                    }
                    state.ime_cursor_range = cursor_range;
                    Some(CCursorRange::two(start_cursor, ccursor))
                } else {
                    None
                }
            }

            Event::CompositionEnd(prediction) => {
                // CompositionEnd only characters may be typed into TextEdit without trigger CompositionStart first,
                // so do not check `state.has_ime = true` in the following statement.
                if prediction != "\n" && prediction != "\r" {
                    state.has_ime = false;
                    let mut ccursor;
                    if !prediction.is_empty()
                        && cursor_range.secondary.ccursor.index
                            == state.ime_cursor_range.secondary.ccursor.index
                    {
                        ccursor = text.delete_selected(&cursor_range);
                        text.insert_text_at(&mut ccursor, prediction, char_limit);
                    } else {
                        ccursor = cursor_range.primary.ccursor;
                    }
                    Some(CCursorRange::one(ccursor))
                } else {
                    None
                }
            }

            _ => None,
        };

        if let Some(new_ccursor_range) = did_mutate_text {
            any_change = true;

            // Layout again to avoid frame delay, and to keep `text` and `galley` in sync.
            *galley = layouter(ui, text.as_str(), wrap_width);

            // Set cursor_range using new galley:
            cursor_range = CursorRange {
                primary: galley.from_ccursor(new_ccursor_range.primary),
                secondary: galley.from_ccursor(new_ccursor_range.secondary),
            };
        }
    }

    state.cursor.set_range(Some(cursor_range));

    state.undoer.lock().feed_state(
        ui.input(|i| i.time),
        &(cursor_range.as_ccursor_range(), text.as_str().to_owned()),
    );

    (any_change, cursor_range)
}

// ----------------------------------------------------------------------------

/// Returns `Some(new_cursor)` if we did mutate `text`.
fn check_for_mutating_key_press(
    os: OperatingSystem,
    cursor_range: &mut CursorRange,
    text: &mut dyn TextBuffer,
    galley: &Galley,
    modifiers: &Modifiers,
    key: Key,
) -> Option<CCursorRange> {
    match key {
        Key::Backspace => {
            let ccursor = if modifiers.mac_cmd {
                text.delete_paragraph_before_cursor(galley, cursor_range)
            } else if let Some(cursor) = cursor_range.single() {
                if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    text.delete_previous_word(cursor.ccursor)
                } else {
                    text.delete_previous_char(cursor.ccursor)
                }
            } else {
                text.delete_selected(cursor_range)
            };
            Some(CCursorRange::one(ccursor))
        }

        Key::Delete if !modifiers.shift || os != OperatingSystem::Windows => {
            let ccursor = if modifiers.mac_cmd {
                text.delete_paragraph_after_cursor(galley, cursor_range)
            } else if let Some(cursor) = cursor_range.single() {
                if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    text.delete_next_word(cursor.ccursor)
                } else {
                    text.delete_next_char(cursor.ccursor)
                }
            } else {
                text.delete_selected(cursor_range)
            };
            let ccursor = CCursor {
                prefer_next_row: true,
                ..ccursor
            };
            Some(CCursorRange::one(ccursor))
        }

        Key::H if modifiers.ctrl => {
            let ccursor = text.delete_previous_char(cursor_range.primary.ccursor);
            Some(CCursorRange::one(ccursor))
        }

        Key::K if modifiers.ctrl => {
            let ccursor = text.delete_paragraph_after_cursor(galley, cursor_range);
            Some(CCursorRange::one(ccursor))
        }

        Key::U if modifiers.ctrl => {
            let ccursor = text.delete_paragraph_before_cursor(galley, cursor_range);
            Some(CCursorRange::one(ccursor))
        }

        Key::W if modifiers.ctrl => {
            let ccursor = if let Some(cursor) = cursor_range.single() {
                text.delete_previous_word(cursor.ccursor)
            } else {
                text.delete_selected(cursor_range)
            };
            Some(CCursorRange::one(ccursor))
        }

        _ => None,
    }
}
