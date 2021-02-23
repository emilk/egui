use crate::*;
use epaint::Galley;

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
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
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

    pub fn small(self) -> Self {
        self.text_style(TextStyle::Small)
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
}

impl Label {
    pub fn layout(&self, ui: &Ui) -> Galley {
        let max_width = ui.available_width();
        self.layout_width(ui, max_width)
    }

    pub fn layout_width(&self, ui: &Ui, max_width: f32) -> Galley {
        let text_style = self.text_style_or_default(ui.style());
        let font = &ui.fonts()[text_style];
        let wrap_width = if self.should_wrap(ui) {
            max_width
        } else {
            f32::INFINITY
        };
        font.layout_multiline(self.text.clone(), wrap_width) // TODO: avoid clone
    }

    pub fn font_height(&self, fonts: &epaint::text::Fonts, style: &Style) -> f32 {
        let text_style = self.text_style_or_default(style);
        fonts[text_style].row_height()
    }

    // TODO: this should return a LabelLayout which has a paint method.
    // We can then split Widget::Ui in two: layout + allocating space, and painting.
    // this allows us to assemble labels, THEN detect interaction, THEN chose color style based on that.
    // pub fn layout(self, ui: &mut ui) -> LabelLayout { }

    // TODO: a paint method for painting anywhere in a ui.
    // This should be the easiest method of putting text anywhere.

    pub fn paint_galley(&self, ui: &mut Ui, pos: Pos2, galley: Galley) {
        let Self {
            mut background_color,
            code,
            strong,
            weak,
            strikethrough,
            underline,
            italics,
            ..
        } = *self;

        let text_color = if let Some(text_color) = self.text_color {
            text_color
        } else if strong {
            ui.visuals().strong_text_color()
        } else if weak {
            ui.visuals().weak_text_color()
        } else {
            ui.visuals().text_color()
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

        let text_style = self.text_style_or_default(ui.style());
        ui.painter()
            .galley_with_italics(pos, galley, text_style, text_color, italics);

        ui.painter().extend(lines);
    }

    /// Read the text style, or get the default for the current style
    pub fn text_style_or_default(&self, style: &Style) -> TextStyle {
        self.text_style.unwrap_or(style.body_text_style)
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
}

impl Widget for Label {
    fn ui(self, ui: &mut Ui) -> Response {
        if self.should_wrap(ui)
            && ui.layout().main_dir() == Direction::LeftToRight
            && ui.layout().main_wrap()
        {
            // On a wrapping horizontal layout we want text to start after the last widget,
            // then continue on the line below! This will take some extra work:

            let max_width = ui.available_width();
            let first_row_indentation = max_width - ui.available_size_before_wrap().x;

            let text_style = self.text_style_or_default(ui.style());
            let font = &ui.fonts()[text_style];
            let mut galley = font.layout_multiline_with_indentation_and_max_width(
                self.text.clone(),
                first_row_indentation,
                max_width,
            );

            let pos = pos2(ui.min_rect().left(), ui.cursor().y);

            assert!(!galley.rows.is_empty(), "Galleys are never empty");
            let rect = galley.rows[0].rect().translate(vec2(pos.x, pos.y));
            let id = ui.advance_cursor_after_rect(rect);
            let mut total_response = ui.interact(rect, id, Sense::hover());

            let mut y_translation = 0.0;
            if let Some(row) = galley.rows.get(1) {
                // We could be sharing the first row with e.g. a button, that is higher than text.
                // So we need to compensate for that:
                if pos.y + row.y_min < ui.min_rect().bottom() {
                    y_translation = ui.min_rect().bottom() - row.y_min - pos.y;
                }
            }

            for row in galley.rows.iter_mut().skip(1) {
                row.y_min += y_translation;
                row.y_max += y_translation;
                let rect = row.rect().translate(vec2(pos.x, pos.y));
                ui.advance_cursor_after_rect(rect);
                total_response |= ui.interact(rect, id, Sense::hover());
            }

            self.paint_galley(ui, pos, galley);
            total_response
        } else {
            let galley = self.layout(ui);
            let (rect, response) = ui.allocate_exact_size(galley.size, Sense::click());
            self.paint_galley(ui, rect.min, galley);
            response
        }
    }
}

impl Into<Label> for &str {
    fn into(self) -> Label {
        Label::new(self)
    }
}

impl Into<Label> for &String {
    fn into(self) -> Label {
        Label::new(self)
    }
}

impl Into<Label> for String {
    fn into(self) -> Label {
        Label::new(self)
    }
}
