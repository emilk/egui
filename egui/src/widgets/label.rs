use crate::*;
use epaint::Galley;
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

    #[deprecated = "Use Label::wrap instead"]
    pub fn multiline(self, multiline: bool) -> Self {
        self.wrap(multiline)
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
        self.layout_width(ui, max_width)
    }

    pub fn layout_width(&self, ui: &Ui, max_width: f32) -> Arc<Galley> {
        let text_style = self.text_style_or_default(ui.style());
        let wrap_width = if self.should_wrap(ui) {
            max_width
        } else {
            f32::INFINITY
        };
        let galley = ui
            .fonts()
            .layout_multiline(text_style, self.text.clone(), wrap_width); // TODO: avoid clone
        self.valign_galley(ui, text_style, galley)
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

    pub fn paint_galley(&self, ui: &mut Ui, pos: Pos2, galley: Arc<Galley>) {
        self.paint_galley_impl(ui, pos, galley, false, ui.visuals().text_color())
    }

    fn paint_galley_impl(
        &self,
        ui: &mut Ui,
        pos: Pos2,
        galley: Arc<Galley>,
        has_focus: bool,
        response_color: Color32,
    ) {
        let Self {
            mut background_color,
            code,
            strong,
            weak,
            strikethrough,
            underline,
            italics,
            raised: _,
            ..
        } = *self;

        let underline = underline || has_focus;

        let text_color = if let Some(text_color) = self.text_color {
            text_color
        } else if strong {
            ui.visuals().strong_text_color()
        } else if weak {
            ui.visuals().weak_text_color()
        } else {
            response_color
        };

        if code {
            background_color = ui.visuals().code_bg_color;
        }

        let mut lines = vec![];

        if strikethrough || underline || background_color != Color32::TRANSPARENT {
            for row in &galley.rows {
                let rect = row.rect().translate(pos.to_vec2());

                if background_color != Color32::TRANSPARENT {
                    let rect = rect.expand(1.0); // looks better
                    ui.painter().rect_filled(rect, 0.0, background_color);
                }

                let stroke_width = 1.0;
                if strikethrough {
                    lines.push(Shape::line_segment(
                        [rect.left_center(), rect.right_center()],
                        (stroke_width, text_color),
                    ));
                }
                if underline {
                    lines.push(Shape::line_segment(
                        [rect.left_bottom(), rect.right_bottom()],
                        (stroke_width, text_color),
                    ));
                }
            }
        }

        ui.painter()
            .galley_with_italics(pos, galley, text_color, italics);

        ui.painter().extend(lines);
    }

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

    fn valign_galley(
        &self,
        ui: &Ui,
        text_style: TextStyle,
        mut galley: Arc<Galley>,
    ) -> Arc<Galley> {
        if text_style == TextStyle::Small {
            // Hacky McHackface strikes again:
            let dy = if self.raised {
                -2.0
            } else {
                let normal_text_height = ui.fonts()[TextStyle::Body].row_height();
                let font_height = ui.fonts().row_height(text_style);
                (normal_text_height - font_height) / 2.0 - 1.0 // center

                // normal_text_height - font_height // align bottom
            };

            if dy != 0.0 {
                for row in &mut Arc::make_mut(&mut galley).rows {
                    row.translate_y(dy);
                }
            }
        }
        galley
    }
}

impl Widget for Label {
    fn ui(self, ui: &mut Ui) -> Response {
        let sense = self.sense;

        if self.should_wrap(ui)
            && ui.layout().main_dir() == Direction::LeftToRight
            && ui.layout().main_wrap()
        {
            // On a wrapping horizontal layout we want text to start after the previous widget,
            // then continue on the line below! This will take some extra work:

            let cursor = ui.cursor();
            let max_width = ui.available_width();
            let first_row_indentation = max_width - ui.available_size_before_wrap().x;

            let text_style = self.text_style_or_default(ui.style());
            let galley = ui.fonts().layout_multiline_with_indentation_and_max_width(
                text_style,
                self.text.clone(),
                first_row_indentation,
                max_width,
            );
            let mut galley: Galley = (*galley).clone();

            let pos = pos2(ui.max_rect().left(), ui.cursor().top());

            assert!(!galley.rows.is_empty(), "Galleys are never empty");

            // Center first row within the cursor:
            let dy = 0.5 * (cursor.height() - galley.rows[0].height());
            galley.rows[0].translate_y(dy);

            // We could be sharing the first row with e.g. a button which is higher than text.
            // So we need to compensate for that:
            if let Some(row) = galley.rows.get_mut(1) {
                if pos.y + row.y_min < cursor.bottom() {
                    let y_translation = cursor.bottom() - row.y_min - pos.y;
                    if y_translation != 0.0 {
                        for row in galley.rows.iter_mut().skip(1) {
                            row.translate_y(y_translation);
                        }
                    }
                }
            }

            let galley = self.valign_galley(ui, text_style, Arc::new(galley));

            let rect = galley.rows[0].rect().translate(vec2(pos.x, pos.y));
            let mut response = ui.allocate_rect(rect, sense);
            for row in galley.rows.iter().skip(1) {
                let rect = row.rect().translate(vec2(pos.x, pos.y));
                response |= ui.allocate_rect(rect, sense);
            }
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, &galley.text));
            let response_color = ui.style().interact(&response).text_color();
            self.paint_galley_impl(ui, pos, galley, response.has_focus(), response_color);
            response
        } else {
            let galley = self.layout(ui);
            let (rect, response) = ui.allocate_exact_size(galley.size, sense);
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, &galley.text));
            let response_color = ui.style().interact(&response).text_color();
            self.paint_galley_impl(ui, rect.min, galley, response.has_focus(), response_color);
            response
        }
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
