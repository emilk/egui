use crate::*;

/// One out of several alternatives, either selected or not.
/// Will mark selected items with a different background color
/// An alternative to [`RadioButton`] and [`Checkbox`].
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Debug)]
pub struct SelectableLabel {
    selected: bool,
    text: String,
}

impl SelectableLabel {
    pub fn new(selected: bool, text: impl Into<String>) -> Self {
        Self {
            selected,
            text: text.into(),
        }
    }
}

impl Widget for SelectableLabel {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { selected, text } = self;

        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];

        let button_padding = ui.style().spacing.button_padding;
        let total_extra = button_padding + button_padding;

        let galley = font.layout_multiline(text, ui.available_width() - total_extra.x);

        let mut desired_size = total_extra + galley.size;
        desired_size = desired_size.at_least(ui.style().spacing.interact_size);
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click());

        let text_cursor = pos2(
            rect.min.x + button_padding.x,
            rect.center().y - 0.5 * galley.size.y,
        );

        let visuals = ui.style().interact(&response);

        if selected || response.hovered {
            let bg_fill = if selected {
                ui.style().visuals.selection.bg_fill
            } else {
                Default::default()
            };
            ui.painter().rect(rect, 0.0, bg_fill, visuals.bg_stroke);
        }

        let text_color = ui
            .style()
            .visuals
            .override_text_color
            .unwrap_or_else(|| visuals.text_color());
        ui.painter()
            .galley(text_cursor, galley, text_style, text_color);
        response
    }
}
