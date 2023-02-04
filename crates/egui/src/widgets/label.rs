use crate::{widget_text::WidgetTextGalley, *};

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
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Label {
    text: WidgetText,
    wrap: Option<bool>,
    sense: Option<Sense>,
}

impl Label {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            wrap: None,
            sense: None,
        }
    }

    pub fn text(&self) -> &str {
        self.text.text()
    }

    /// If `true`, the text will wrap to stay within the max width of the [`Ui`].
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
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = Some(sense);
        self
    }
}

impl Label {
    /// Do layout and position the galley in the ui, without painting it or adding widget info.
    pub fn layout_in_ui(self, ui: &mut Ui) -> (Pos2, WidgetTextGalley, Response) {
        let sense = self.sense.unwrap_or_else(|| {
            // We only want to focus labels if the screen reader is on.
            if ui.memory(|mem| mem.options.screen_reader) {
                Sense::focusable_noninteractive()
            } else {
                Sense::hover()
            }
        });
        if let WidgetText::Galley(galley) = self.text {
            // If the user said "use this specific galley", then just use it:
            let (rect, response) = ui.allocate_exact_size(galley.size(), sense);
            let pos = match galley.job.halign {
                Align::LEFT => rect.left_top(),
                Align::Center => rect.center_top(),
                Align::RIGHT => rect.right_top(),
            };
            let text_galley = WidgetTextGalley {
                galley,
                galley_has_color: true,
            };
            return (pos, text_galley, response);
        }

        let valign = ui.layout().vertical_align();
        let mut text_job = self
            .text
            .into_text_job(ui.style(), FontSelection::Default, valign);

        let should_wrap = self.wrap.unwrap_or_else(|| ui.wrap_text());
        let available_width = ui.available_width();

        if should_wrap
            && ui.layout().main_dir() == Direction::LeftToRight
            && ui.layout().main_wrap()
            && available_width.is_finite()
        {
            // On a wrapping horizontal layout we want text to start after the previous widget,
            // then continue on the line below! This will take some extra work:

            let cursor = ui.cursor();
            let first_row_indentation = available_width - ui.available_size_before_wrap().x;
            egui_assert!(first_row_indentation.is_finite());

            text_job.job.wrap.max_width = available_width;
            text_job.job.first_row_min_height = cursor.height();
            text_job.job.halign = Align::Min;
            text_job.job.justify = false;
            if let Some(first_section) = text_job.job.sections.first_mut() {
                first_section.leading_space = first_row_indentation;
            }
            let text_galley = ui.fonts(|f| text_job.into_galley(f));

            let pos = pos2(ui.max_rect().left(), ui.cursor().top());
            assert!(
                !text_galley.galley.rows.is_empty(),
                "Galleys are never empty"
            );
            // collect a response from many rows:
            let rect = text_galley.galley.rows[0]
                .rect
                .translate(vec2(pos.x, pos.y));
            let mut response = ui.allocate_rect(rect, sense);
            for row in text_galley.galley.rows.iter().skip(1) {
                let rect = row.rect.translate(vec2(pos.x, pos.y));
                response |= ui.allocate_rect(rect, sense);
            }
            (pos, text_galley, response)
        } else {
            if should_wrap {
                text_job.job.wrap.max_width = available_width;
            } else {
                text_job.job.wrap.max_width = f32::INFINITY;
            };

            if ui.is_grid() {
                // TODO(emilk): remove special Grid hacks like these
                text_job.job.halign = Align::LEFT;
                text_job.job.justify = false;
            } else {
                text_job.job.halign = ui.layout().horizontal_placement();
                text_job.job.justify = ui.layout().horizontal_justify();
            };

            let text_galley = ui.fonts(|f| text_job.into_galley(f));
            let (rect, response) = ui.allocate_exact_size(text_galley.size(), sense);
            let pos = match text_galley.galley.job.halign {
                Align::LEFT => rect.left_top(),
                Align::Center => rect.center_top(),
                Align::RIGHT => rect.right_top(),
            };
            (pos, text_galley, response)
        }
    }
}

impl Widget for Label {
    fn ui(self, ui: &mut Ui) -> Response {
        let (pos, text_galley, response) = self.layout_in_ui(ui);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, text_galley.text()));

        if ui.is_rect_visible(response.rect) {
            let response_color = ui.style().interact(&response).text_color();

            let underline = if response.has_focus() || response.highlighted() {
                Stroke::new(1.0, response_color)
            } else {
                Stroke::NONE
            };

            let override_text_color = if text_galley.galley_has_color {
                None
            } else {
                Some(response_color)
            };

            ui.painter().add(epaint::TextShape {
                pos,
                galley: text_galley.galley,
                override_text_color,
                underline,
                angle: 0.0,
            });
        }

        response
    }
}
