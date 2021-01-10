use crate::{paint::Galley, *};

/// Static text.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Label {
    // TODO: not pub
    pub(crate) text: String,
    pub(crate) multiline: Option<bool>,
    pub(crate) text_style: Option<TextStyle>,
    pub(crate) text_color: Option<Color32>,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            multiline: None,
            text_style: None,
            text_color: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// If `true`, the text will wrap at the `max_width`.
    /// By default `multiline` will be true in vertical layouts
    /// and horizontal layouts with wrapping,
    /// and false on non-wrapping horizontal layouts.
    ///
    /// If the text has any newlines (`\n`) in it, multiline will automatically turn on.
    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = Some(multiline);
        self
    }

    /// If you do not set a `TextStyle`, the default `style.text_style`.
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

    pub fn small(self) -> Self {
        self.text_style(TextStyle::Small)
    }

    pub fn text_color(mut self, text_color: impl Into<Color32>) -> Self {
        self.text_color = Some(text_color.into());
        self
    }

    pub fn layout(&self, ui: &Ui) -> Galley {
        let max_width = ui.available_width();
        self.layout_width(ui, max_width)
    }

    pub fn layout_width(&self, ui: &Ui, max_width: f32) -> Galley {
        let text_style = self.text_style_or_default(ui.style());
        let font = &ui.fonts()[text_style];
        if self.is_multiline(ui) {
            font.layout_multiline(self.text.clone(), max_width) // TODO: avoid clone
        } else {
            font.layout_single_line(self.text.clone()) // TODO: avoid clone
        }
    }

    pub fn font_height(&self, fonts: &paint::text::Fonts, style: &Style) -> f32 {
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
        let text_style = self.text_style_or_default(ui.style());
        let text_color = self
            .text_color
            .unwrap_or_else(|| ui.style().visuals.text_color());
        ui.painter().galley(pos, galley, text_style, text_color);
    }

    /// Read the text style, or get the default for the current style
    pub fn text_style_or_default(&self, style: &Style) -> TextStyle {
        self.text_style.unwrap_or(style.body_text_style)
    }

    fn is_multiline(&self, ui: &Ui) -> bool {
        self.multiline.unwrap_or_else(|| {
            let layout = ui.layout();
            layout.is_vertical()
                || layout.is_horizontal() && layout.main_wrap()
                || self.text.contains('\n')
        })
    }
}

impl Widget for Label {
    fn ui(self, ui: &mut Ui) -> Response {
        if self.is_multiline(ui)
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
