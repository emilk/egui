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

        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding;

        let galley = if ui.wrap_text() {
            font.layout_multiline(text, ui.available_width() - total_extra.x)
        } else {
            font.layout_no_wrap(text)
        };

        let mut desired_size = total_extra + galley.size;
        desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click());

        let text_cursor = ui
            .layout()
            .align_size_within_rect(galley.size, rect.shrink2(button_padding))
            .min;

        let visuals = ui.style().interact(&response);

        if selected || response.hovered() {
            let rect = rect.expand(visuals.expansion);

            let fill = if selected {
                ui.visuals().selection.bg_fill
            } else {
                Default::default()
            };

            let stroke = if selected {
                ui.visuals().selection.stroke
            } else {
                visuals.bg_stroke
            };

            let corner_radius = 2.0;
            ui.painter().rect(rect, corner_radius, fill, stroke);
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
