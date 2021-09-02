use crate::*;
use epaint::{
    text::{LayoutJob, LayoutSection, TextFormat},
    Galley,
};
use std::sync::Arc;

/// Static text.
///
/// ```
/// # let ui = &mut egui::Ui::__test();
/// ui.label("Equivalent");
/// ui.add(egui::Label::new("Equivalent"));
/// ui.add(egui::Label::new("With Options").text_color(egui::Color32::RED));
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Label {
    // TODO: not pub
    pub(crate) text: String,
    pub(crate) wrap: Option<bool>,
    pub(crate) text_style: Option<TextStyle>,
    pub(crate) background_color: Color32,
    pub(crate) text_color: Option<Color32>,
    code: bool,
    strong: bool,
    weak: bool,
    strikethrough: bool,
    underline: bool,
    italics: bool,
    raised: bool,
    sense: Sense,
}

impl Label {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(text: impl ToString) -> Self {
        Self {
            text: text.to_string(),
            wrap: None,
            text_style: None,
            background_color: Color32::TRANSPARENT,
            text_color: None,
            code: false,
            strong: false,
            weak: false,
            strikethrough: false,
            underline: false,
            italics: false,
            raised: false,
            sense: Sense::focusable_noninteractive(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// If `true`, the text will wrap at the `max_width`.
    /// By default [`Self::wrap`] will be true in vertical layouts
    /// and horizontal layouts with wrapping,
    /// and false on non-wrapping horizontal layouts.
    ///
    /// Note that any `\n` in the text label will always produce a new line.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = Some(wrap);
        self
    }

    /// The default is [`Style::body_text_style`] (generally [`TextStyle::Body`]).
    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }

    pub fn heading(self) -> Self {
        self.text_style(TextStyle::Heading)
    }

    pub fn monospace(self) -> Self {
        self.text_style(TextStyle::Monospace)
    }

    /// Monospace label with gray background
    pub fn code(mut self) -> Self {
        self.code = true;
        self.text_style(TextStyle::Monospace)
    }

    /// Extra strong text (stronger color).
    pub fn strong(mut self) -> Self {
        self.strong = true;
        self
    }

    /// Extra weak text (fainter color).
    pub fn weak(mut self) -> Self {
        self.weak = true;
        self
    }

    /// draw a line under the text
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// draw a line through the text, crossing it out
    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    /// tilt the characters to the right.
    pub fn italics(mut self) -> Self {
        self.italics = true;
        self
    }

    /// Smaller text
    pub fn small(self) -> Self {
        self.text_style(TextStyle::Small)
    }

    /// For e.g. exponents
    pub fn small_raised(self) -> Self {
        self.text_style(TextStyle::Small).raised()
    }

    /// Align text to top. Only applicable together with [`Self::small()`].
    pub fn raised(mut self) -> Self {
        self.raised = true;
        self
    }

    /// Fill-color behind the text
    pub fn background_color(mut self, background_color: impl Into<Color32>) -> Self {
        self.background_color = background_color.into();
        self
    }

    pub fn text_color(mut self, text_color: impl Into<Color32>) -> Self {
        self.text_color = Some(text_color.into());
        self
    }

    /// Make the label respond to clicks and/or drags.
    ///
    /// By default, a label is inert and does not respond to click or drags.
    /// By calling this you can turn the label into a button of sorts.
    /// This will also give the label the hover-effect of a button, but without the frame.
    ///
    /// ``` rust
    /// # use egui::{Label, Sense};
    /// # let ui = &mut egui::Ui::__test();
    /// if ui.add(Label::new("click me").sense(Sense::click())).clicked() {
    ///     /* … */
    /// }
    /// ```
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }
}

impl Label {
    pub fn layout(&self, ui: &Ui) -> Arc<Galley> {
        let max_width = ui.available_width();
        let line_color = self.get_text_color(ui, ui.visuals().text_color());
        self.layout_width(ui, max_width, line_color)
    }

    /// `line_color`: used for underline and strikethrough, if any.
    pub fn layout_width(&self, ui: &Ui, max_width: f32, line_color: Color32) -> Arc<Galley> {
        self.layout_impl(ui, 0.0, max_width, 0.0, line_color)
    }

    fn layout_impl(
        &self,
        ui: &Ui,
        leading_space: f32,
        max_width: f32,
        first_row_min_height: f32,
        line_color: Color32,
    ) -> Arc<Galley> {
        let text_style = self.text_style_or_default(ui.style());
        let wrap_width = if self.should_wrap(ui) {
            max_width
        } else {
            f32::INFINITY
        };

        let mut background_color = self.background_color;
        if self.code {
            background_color = ui.visuals().code_bg_color;
        }
        let underline = if self.underline {
            Stroke::new(1.0, line_color)
        } else {
            Stroke::none()
        };
        let strikethrough = if self.strikethrough {
            Stroke::new(1.0, line_color)
        } else {
            Stroke::none()
        };

        let valign = if self.raised {
            Align::TOP
        } else {
            ui.layout().vertical_align()
        };

        let job = LayoutJob {
            text: self.text.clone(), // TODO: avoid clone
            sections: vec![LayoutSection {
                leading_space,
                byte_range: 0..self.text.len(),
                format: TextFormat {
                    style: text_style,
                    color: Color32::TEMPORARY_COLOR,
                    background: background_color,
                    italics: self.italics,
                    underline,
                    strikethrough,
                    valign,
                },
            }],
            wrap_width,
            first_row_min_height,
            ..Default::default()
        };

        ui.fonts().layout_job(job)
    }

    /// `has_focus`: the item is selected with the keyboard, so highlight with underline.
    /// `response_color`: Unless we have a special color set, use this.
    pub(crate) fn paint_galley(
        &self,
        ui: &mut Ui,
        pos: Pos2,
        galley: Arc<Galley>,
        has_focus: bool,
        response_color: Color32,
    ) {
        let text_color = self.get_text_color(ui, response_color);

        let underline = if has_focus {
            Stroke::new(1.0, text_color)
        } else {
            Stroke::none()
        };

        ui.painter().add(Shape::Text {
            pos,
            galley,
            override_text_color: Some(text_color),
            underline,
        });
    }

    /// `response_color`: Unless we have a special color set, use this.
    fn get_text_color(&self, ui: &Ui, response_color: Color32) -> Color32 {
        if let Some(text_color) = self.text_color {
            text_color
        } else if self.strong {
            ui.visuals().strong_text_color()
        } else if self.weak {
            ui.visuals().weak_text_color()
        } else {
            response_color
        }
    }

    pub fn font_height(&self, fonts: &epaint::text::Fonts, style: &Style) -> f32 {
        let text_style = self.text_style_or_default(style);
        fonts.row_height(text_style)
    }

    // TODO: this should return a LabelLayout which has a paint method.
    // We can then split Widget::Ui in two: layout + allocating space, and painting.
    // this allows us to assemble labels, THEN detect interaction, THEN chose color style based on that.
    // pub fn layout(self, ui: &mut ui) -> LabelLayout { }

    // TODO: a paint method for painting anywhere in a ui.
    // This should be the easiest method of putting text anywhere.

    /// Read the text style, or get the default for the current style
    pub fn text_style_or_default(&self, style: &Style) -> TextStyle {
        self.text_style
            .or(style.override_text_style)
            .unwrap_or(style.body_text_style)
    }

    fn should_wrap(&self, ui: &Ui) -> bool {
        self.wrap.or(ui.style().wrap).unwrap_or_else(|| {
            if let Some(grid) = ui.grid() {
                grid.wrap_text()
            } else {
                let layout = ui.layout();
                layout.is_vertical() || layout.is_horizontal() && layout.main_wrap()
            }
        })
    }

    /// Do layout and place the galley in the ui, without painting it or adding widget info.
    pub(crate) fn layout_in_ui(&self, ui: &mut Ui) -> (Pos2, Arc<Galley>, Response) {
        let sense = self.sense;
        let max_width = ui.available_width();

        if self.should_wrap(ui)
            && ui.layout().main_dir() == Direction::LeftToRight
            && ui.layout().main_wrap()
            && max_width.is_finite()
        {
            // On a wrapping horizontal layout we want text to start after the previous widget,
            // then continue on the line below! This will take some extra work:

            let cursor = ui.cursor();
            let first_row_indentation = max_width - ui.available_size_before_wrap().x;
            egui_assert!(first_row_indentation.is_finite());

            let first_row_min_height = cursor.height();
            let default_color = self.get_text_color(ui, ui.visuals().text_color());
            let galley = self.layout_impl(
                ui,
                first_row_indentation,
                max_width,
                first_row_min_height,
                default_color,
            );

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
            let galley = self.layout(ui);
            let (rect, response) = ui.allocate_exact_size(galley.size, sense);
            (rect.min, galley, response)
        }
    }
}

impl Widget for Label {
    fn ui(self, ui: &mut Ui) -> Response {
        let (pos, galley, response) = self.layout_in_ui(ui);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, galley.text()));
        let response_color = ui.style().interact(&response).text_color();
        self.paint_galley(ui, pos, galley, response.has_focus(), response_color);
        response
    }
}

impl From<&str> for Label {
    fn from(s: &str) -> Label {
        Label::new(s)
    }
}

impl From<&String> for Label {
    fn from(s: &String) -> Label {
        Label::new(s)
    }
}

impl From<String> for Label {
    fn from(s: String) -> Label {
        Label::new(s)
    }
}
