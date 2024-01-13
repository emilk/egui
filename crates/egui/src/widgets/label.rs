use std::sync::Arc;

use crate::{
    text_edit::{
        cursor_interaction::cursor_rect, paint_cursor_selection, CursorRange, TextCursorState,
    },
    *,
};

/// Static text.
///
/// Usually it is more convenient to use [`Ui::label`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// ui.label("Equivalent");
/// ui.add(egui::Label::new("Equivalent"));
/// ui.add(egui::Label::new("With Options").wrap(false));
/// ui.label(egui::RichText::new("With formatting").underline());
/// # });
/// ```
///
/// For full control of the text you can use [`crate::text::LayoutJob`]
/// as argument to [`Self::new`].
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Label {
    text: WidgetText,
    wrap: Option<bool>,
    truncate: bool,
    sense: Option<Sense>,
    selectable: Option<bool>,
}

impl Label {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            wrap: None,
            truncate: false,
            sense: None,
            selectable: None,
        }
    }

    pub fn text(&self) -> &str {
        self.text.text()
    }

    /// If `true`, the text will wrap to stay within the max width of the [`Ui`].
    ///
    /// Calling `wrap` will override [`Self::truncate`].
    ///
    /// By default [`Self::wrap`] will be `true` in vertical layouts
    /// and horizontal layouts with wrapping,
    /// and `false` on non-wrapping horizontal layouts.
    ///
    /// Note that any `\n` in the text will always produce a new line.
    ///
    /// You can also use [`crate::Style::wrap`].
    #[inline]
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = Some(wrap);
        self.truncate = false;
        self
    }

    /// If `true`, the text will stop at the max width of the [`Ui`],
    /// and what doesn't fit will be elided, replaced with `…`.
    ///
    /// If the text is truncated, the full text will be shown on hover as a tool-tip.
    ///
    /// Default is `false`, which means the text will expand the parent [`Ui`],
    /// or wrap if [`Self::wrap`] is set.
    ///
    /// Calling `truncate` will override [`Self::wrap`].
    #[inline]
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.wrap = None;
        self.truncate = truncate;
        self
    }

    /// Can the user select the text with the mouse?
    ///
    /// Overrides [`crate::style::Interaction::selectable_labels`].
    #[inline]
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = Some(selectable);
        self
    }

    /// Make the label respond to clicks and/or drags.
    ///
    /// By default, a label is inert and does not respond to click or drags.
    /// By calling this you can turn the label into a button of sorts.
    /// This will also give the label the hover-effect of a button, but without the frame.
    ///
    /// ```
    /// # use egui::{Label, Sense};
    /// # egui::__run_test_ui(|ui| {
    /// if ui.add(Label::new("click me").sense(Sense::click())).clicked() {
    ///     /* … */
    /// }
    /// # });
    /// ```
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = Some(sense);
        self
    }
}

impl Label {
    /// Do layout and position the galley in the ui, without painting it or adding widget info.
    pub fn layout_in_ui(self, ui: &mut Ui) -> (Pos2, Arc<Galley>, Response) {
        let selectable = self
            .selectable
            .unwrap_or_else(|| ui.style().interaction.selectable_labels);

        let mut sense = self.sense.unwrap_or_else(|| {
            if ui.memory(|mem| mem.options.screen_reader) {
                // We only want to focus labels if the screen reader is on.
                Sense::focusable_noninteractive()
            } else {
                Sense::hover()
            }
        });

        if selectable {
            // On touch screens (e.g. mobile in `eframe` web), should
            // dragging select text, or scroll the enclosing [`ScrollArea`] (if any)?
            // Since currently copying selected text in not supported on `eframe` web,
            // we prioritize touch-scrolling:
            let allow_drag_to_select = ui.input(|i| !i.any_touches());

            let select_sense = if allow_drag_to_select {
                Sense::click_and_drag()
            } else {
                Sense::click()
            };

            sense = sense.union(select_sense);
        }

        if let WidgetText::Galley(galley) = self.text {
            // If the user said "use this specific galley", then just use it:
            let (rect, response) = ui.allocate_exact_size(galley.size(), sense);
            let pos = match galley.job.halign {
                Align::LEFT => rect.left_top(),
                Align::Center => rect.center_top(),
                Align::RIGHT => rect.right_top(),
            };
            return (pos, galley, response);
        }

        let valign = ui.layout().vertical_align();
        let mut layout_job = self
            .text
            .into_layout_job(ui.style(), FontSelection::Default, valign);

        let truncate = self.truncate;
        let wrap = !truncate && self.wrap.unwrap_or_else(|| ui.wrap_text());
        let available_width = ui.available_width();

        if wrap
            && ui.layout().main_dir() == Direction::LeftToRight
            && ui.layout().main_wrap()
            && available_width.is_finite()
        {
            // On a wrapping horizontal layout we want text to start after the previous widget,
            // then continue on the line below! This will take some extra work:

            let cursor = ui.cursor();
            let first_row_indentation = available_width - ui.available_size_before_wrap().x;
            egui_assert!(first_row_indentation.is_finite());

            layout_job.wrap.max_width = available_width;
            layout_job.first_row_min_height = cursor.height();
            layout_job.halign = Align::Min;
            layout_job.justify = false;
            if let Some(first_section) = layout_job.sections.first_mut() {
                first_section.leading_space = first_row_indentation;
            }
            let galley = ui.fonts(|fonts| fonts.layout_job(layout_job));

            let pos = pos2(ui.max_rect().left(), ui.cursor().top());
            assert!(!galley.rows.is_empty(), "Galleys are never empty");
            // collect a response from many rows:
            let rect = galley.rows[0].rect.translate(vec2(pos.x, pos.y));
            let mut response = ui.allocate_rect(rect, sense);
            for row in galley.rows.iter().skip(1) {
                let rect = row.rect.translate(vec2(pos.x, pos.y));
                response |= ui.allocate_rect(rect, sense);
            }
            (pos, galley, response)
        } else {
            if truncate {
                layout_job.wrap.max_width = available_width;
                layout_job.wrap.max_rows = 1;
                layout_job.wrap.break_anywhere = true;
            } else if wrap {
                layout_job.wrap.max_width = available_width;
            } else {
                layout_job.wrap.max_width = f32::INFINITY;
            };

            if ui.is_grid() {
                // TODO(emilk): remove special Grid hacks like these
                layout_job.halign = Align::LEFT;
                layout_job.justify = false;
            } else {
                layout_job.halign = ui.layout().horizontal_placement();
                layout_job.justify = ui.layout().horizontal_justify();
            };

            let galley = ui.fonts(|fonts| fonts.layout_job(layout_job));
            let (rect, response) = ui.allocate_exact_size(galley.size(), sense);
            let galley_pos = match galley.job.halign {
                Align::LEFT => rect.left_top(),
                Align::Center => rect.center_top(),
                Align::RIGHT => rect.right_top(),
            };
            (galley_pos, galley, response)
        }
    }
}

impl Widget for Label {
    fn ui(self, ui: &mut Ui) -> Response {
        let interactive = self.sense.map_or(false, |sense| sense != Sense::hover());
        let selectable = self.selectable;

        let (galley_pos, galley, mut response) = self.layout_in_ui(ui);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, galley.text()));

        if ui.is_rect_visible(response.rect) {
            if galley.elided {
                // Show the full (non-elided) text on hover:
                response = response.on_hover_text(galley.text());
            }

            let response_color = if interactive {
                ui.style().interact(&response).text_color()
            } else {
                ui.style().visuals.text_color()
            };

            let underline = if response.has_focus() || response.highlighted() {
                Stroke::new(1.0, response_color)
            } else {
                Stroke::NONE
            };

            ui.painter().add(
                epaint::TextShape::new(galley_pos, galley.clone(), response_color)
                    .with_underline(underline),
            );

            let selectable = selectable.unwrap_or_else(|| ui.style().interaction.selectable_labels);
            if selectable {
                text_selection(ui, &response, galley_pos, &galley);
            }
        }

        response
    }
}

/// Handle text selection state for a label or similar widget.
///
/// Make sure the widget senses to clicks and drags.
///
/// This should be called after painting the text, because this will also
/// paint the text cursor/selection on top.
pub fn text_selection(ui: &Ui, response: &Response, galley_pos: Pos2, galley: &Galley) {
    let mut cursor_state = LabelSelectionState::load(ui.ctx(), response.id);

    if response.hovered {
        ui.ctx().set_cursor_icon(CursorIcon::Text);
    } else if !cursor_state.is_empty() && ui.input(|i| i.pointer.any_pressed()) {
        // We clicked somewhere else - deselect this label.
        cursor_state = Default::default();
        LabelSelectionState::store(ui.ctx(), response.id, cursor_state);
    }

    let original_cursor = cursor_state.range(galley);

    if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
        let cursor_at_pointer = galley.cursor_from_pos(pointer_pos - galley_pos);
        cursor_state.pointer_interaction(ui, response, cursor_at_pointer, galley);
    }

    if let Some(mut cursor_range) = cursor_state.range(galley) {
        process_selection_key_events(ui, galley, response.id, &mut cursor_range);
        cursor_state.set_range(Some(cursor_range));
    }

    let cursor_range = cursor_state.range(galley);

    if let Some(cursor_range) = cursor_range {
        // We paint the cursor on top of the text, in case
        // the text galley has backgrounds (as e.g. `code` snippets in markup do).
        paint_cursor_selection(
            ui.visuals(),
            ui.painter(),
            galley_pos,
            galley,
            &cursor_range,
        );

        let selection_changed = original_cursor != Some(cursor_range);

        let is_fully_visible = ui.clip_rect().contains_rect(response.rect); // TODO: remove this HACK workaround for https://github.com/emilk/egui/issues/1531

        if selection_changed && !is_fully_visible {
            // Scroll to keep primary cursor in view:
            let row_height = estimate_row_height(galley);
            let primary_cursor_rect =
                cursor_rect(galley_pos, galley, &cursor_range.primary, row_height);
            ui.scroll_to_rect(primary_cursor_rect, None);
        }
    }

    #[cfg(feature = "accesskit")]
    text_edit::accesskit_text::update_accesskit_for_text_widget(
        ui.ctx(),
        response.id,
        cursor_range,
        accesskit::Role::StaticText,
        galley,
        galley_pos,
    );

    if !cursor_state.is_empty() {
        LabelSelectionState::store(ui.ctx(), response.id, cursor_state);
    }
}

fn estimate_row_height(galley: &Galley) -> f32 {
    if let Some(row) = galley.rows.first() {
        row.rect.height()
    } else {
        galley.size().y
    }
}

fn process_selection_key_events(
    ui: &Ui,
    galley: &Galley,
    widget_id: Id,
    cursor_range: &mut CursorRange,
) {
    let mut copy_text = None;

    ui.input(|i| {
        // NOTE: we have a lock on ui/ctx here,
        // so be careful to not call into `ui` or `ctx` again.

        for event in &i.events {
            match event {
                Event::Copy | Event::Cut => {
                    // This logic means we can select everything in an ellided label (including the `…`)
                    // and still copy the entire un-ellided text!
                    let everything_is_selected =
                        cursor_range.contains(&CursorRange::select_all(galley));

                    let copy_everything = cursor_range.is_empty() || everything_is_selected;

                    if copy_everything {
                        copy_text = Some(galley.text().to_owned());
                    } else {
                        copy_text = Some(cursor_range.slice_str(galley).to_owned());
                    }
                }

                event => {
                    cursor_range.on_event(event, galley, widget_id);
                }
            }
        }
    });

    if let Some(copy_text) = copy_text {
        ui.ctx().copy_text(copy_text);
    }
}

// ----------------------------------------------------------------------------

/// One state for all labels.
#[derive(Clone, Copy, Debug, Default)]
struct LabelSelectionState {
    /// Id of the (only) label with a selection, if any
    id: Option<Id>,

    /// The current selection, if any.
    selection: TextCursorState,
}

impl LabelSelectionState {
    fn load(ctx: &Context, id: Id) -> TextCursorState {
        ctx.data(|data| data.get_temp::<Self>(Id::NULL))
            .and_then(|state| (state.id == Some(id)).then_some(state.selection))
            .unwrap_or_default()
    }

    fn store(ctx: &Context, id: Id, selection: TextCursorState) {
        ctx.data_mut(|data| {
            data.insert_temp(
                Id::NULL,
                Self {
                    id: Some(id),
                    selection,
                },
            );
        });
    }
}
