use std::sync::Arc;

use crate::{
    Align, Direction, FontSelection, Galley, Pos2, Response, Sense, Stroke, TextWrapMode, Ui,
    Widget, WidgetInfo, WidgetText, WidgetType, epaint, pos2, text_selection::LabelSelectionState,
};

/// Static text.
///
/// Usually it is more convenient to use [`Ui::label`].
///
/// ```
/// # use egui::TextWrapMode;
/// # egui::__run_test_ui(|ui| {
/// ui.label("Equivalent");
/// ui.add(egui::Label::new("Equivalent"));
/// ui.add(egui::Label::new("With Options").truncate());
/// ui.label(egui::RichText::new("With formatting").underline());
/// # });
/// ```
///
/// For full control of the text you can use [`crate::text::LayoutJob`]
/// as argument to [`Self::new`].
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Label {
    text: WidgetText,
    wrap_mode: Option<TextWrapMode>,
    sense: Option<Sense>,
    selectable: Option<bool>,
    halign: Option<Align>,
    show_tooltip_when_elided: bool,
}

impl Label {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            wrap_mode: None,
            sense: None,
            selectable: None,
            halign: None,
            show_tooltip_when_elided: true,
        }
    }

    pub fn text(&self) -> &str {
        self.text.text()
    }

    /// Set the wrap mode for the text.
    ///
    /// By default, [`crate::Ui::wrap_mode`] will be used, which can be overridden with [`crate::Style::wrap_mode`].
    ///
    /// Note that any `\n` in the text will always produce a new line.
    #[inline]
    pub fn wrap_mode(mut self, wrap_mode: TextWrapMode) -> Self {
        self.wrap_mode = Some(wrap_mode);
        self
    }

    /// Set [`Self::wrap_mode`] to [`TextWrapMode::Wrap`].
    #[inline]
    pub fn wrap(mut self) -> Self {
        self.wrap_mode = Some(TextWrapMode::Wrap);

        self
    }

    /// Set [`Self::wrap_mode`] to [`TextWrapMode::Truncate`].
    #[inline]
    pub fn truncate(mut self) -> Self {
        self.wrap_mode = Some(TextWrapMode::Truncate);
        self
    }

    /// Set [`Self::wrap_mode`] to [`TextWrapMode::Extend`],
    /// disabling wrapping and truncating, and instead expanding the parent [`Ui`].
    #[inline]
    pub fn extend(mut self) -> Self {
        self.wrap_mode = Some(TextWrapMode::Extend);
        self
    }

    /// Sets the horizontal alignment of the Label to the given `Align` value.
    #[inline]
    pub fn halign(mut self, align: Align) -> Self {
        self.halign = Some(align);
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

    /// Show the full text when hovered, if the text was elided.
    ///
    /// By default, this is true.
    ///
    /// ```
    /// # use egui::{Label, Sense};
    /// # egui::__run_test_ui(|ui| {
    /// ui.add(Label::new("some text").show_tooltip_when_elided(false))
    ///     .on_hover_text("completely different text");
    /// # });
    /// ```
    #[inline]
    pub fn show_tooltip_when_elided(mut self, show: bool) -> Self {
        self.show_tooltip_when_elided = show;
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
            let allow_drag_to_select = ui.input(|i| !i.has_touch_screen());

            let mut select_sense = if allow_drag_to_select {
                Sense::click_and_drag()
            } else {
                Sense::click()
            };
            select_sense -= Sense::FOCUSABLE; // Don't move focus to labels with TAB key.

            sense |= select_sense;
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

        let valign = ui.text_valign();
        let mut layout_job = Arc::unwrap_or_clone(self.text.into_layout_job(
            ui.style(),
            FontSelection::Default,
            valign,
        ));

        let available_width = ui.available_width();

        let wrap_mode = self.wrap_mode.unwrap_or_else(|| ui.wrap_mode());
        if wrap_mode == TextWrapMode::Wrap
            && ui.layout().main_dir() == Direction::LeftToRight
            && ui.layout().main_wrap()
            && available_width.is_finite()
        {
            // On a wrapping horizontal layout we want text to start after the previous widget,
            // then continue on the line below! This will take some extra work:

            let cursor = ui.cursor();
            let first_row_indentation = available_width - ui.available_size_before_wrap().x;
            debug_assert!(
                first_row_indentation.is_finite(),
                "first row indentation is not finite: {first_row_indentation}"
            );

            layout_job.wrap.max_width = available_width;
            layout_job.first_row_min_height = cursor.height();
            layout_job.halign = Align::Min;
            layout_job.justify = false;
            if let Some(first_section) = layout_job.sections.first_mut() {
                first_section.leading_space = first_row_indentation;
            }
            let galley = ui.fonts_mut(|fonts| fonts.layout_job(layout_job));

            let pos = pos2(ui.max_rect().left(), ui.cursor().top());
            assert!(!galley.rows.is_empty(), "Galleys are never empty");
            // collect a response from many rows:
            let rect = galley.rows[0]
                .rect_without_leading_space()
                .translate(pos.to_vec2());
            let mut response = ui.allocate_rect(rect, sense);
            response.intrinsic_size = Some(galley.intrinsic_size());
            for placed_row in galley.rows.iter().skip(1) {
                let rect = placed_row.rect().translate(pos.to_vec2());
                response |= ui.allocate_rect(rect, sense);
            }
            (pos, galley, response)
        } else {
            // Apply wrap_mode, but don't overwrite anything important
            // the user may have set manually on the layout_job:
            match wrap_mode {
                TextWrapMode::Extend => {
                    layout_job.wrap.max_width = f32::INFINITY;
                }
                TextWrapMode::Wrap => {
                    layout_job.wrap.max_width = available_width;
                }
                TextWrapMode::Truncate => {
                    layout_job.wrap.max_width = available_width;
                    layout_job.wrap.max_rows = 1;
                    layout_job.wrap.break_anywhere = true;
                }
            }

            if ui.is_grid() {
                // TODO(emilk): remove special Grid hacks like these
                layout_job.halign = Align::LEFT;
                layout_job.justify = false;
            } else {
                layout_job.halign = self
                    .halign
                    .unwrap_or_else(|| ui.layout().horizontal_placement());
                layout_job.justify = ui.layout().horizontal_justify();
            }

            let galley = ui.fonts_mut(|fonts| fonts.layout_job(layout_job));
            let (rect, mut response) = ui.allocate_exact_size(galley.size(), sense);
            response.intrinsic_size = Some(galley.intrinsic_size());
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
        // Interactive = the uses asked to sense interaction.
        // We DON'T want to have the color respond just because the text is selectable;
        // the cursor is enough to communicate that.
        let interactive = self.sense.is_some_and(|sense| sense != Sense::hover());

        let selectable = self.selectable;
        let show_tooltip_when_elided = self.show_tooltip_when_elided;

        let (galley_pos, galley, mut response) = self.layout_in_ui(ui);
        response
            .widget_info(|| WidgetInfo::labeled(WidgetType::Label, ui.is_enabled(), galley.text()));

        if ui.is_rect_visible(response.rect) {
            if show_tooltip_when_elided && galley.elided {
                // Keep the sections and text, but reset everything else (especially wrapping):
                let job = crate::text::LayoutJob {
                    sections: galley.job.sections.clone(),
                    text: galley.job.text.clone(),
                    ..crate::text::LayoutJob::default()
                };
                // Show the full (non-elided) text on hover:
                response = response.on_hover_text(job);
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

            let selectable = selectable.unwrap_or_else(|| ui.style().interaction.selectable_labels);
            if selectable {
                LabelSelectionState::label_text_selection(
                    ui,
                    &response,
                    galley_pos,
                    galley,
                    response_color,
                    underline,
                );
            } else {
                ui.painter().add(
                    epaint::TextShape::new(galley_pos, galley, response_color)
                        .with_underline(underline),
                );
            }
        }

        response
    }
}
