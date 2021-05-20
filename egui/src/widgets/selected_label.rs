use crate::*;

/// One out of several alternatives, either selected or not.
/// Will mark selected items with a different background color.
/// An alternative to [`RadioButton`] and [`Checkbox`].
///
/// Usually you'd use [`Ui::selectable_value`] or [`Ui::selectable_label`] instead.
///
/// ```
/// # let ui = &mut egui::Ui::__test();
/// #[derive(PartialEq)]
/// enum Enum { First, Second, Third }
/// let mut my_enum = Enum::First;
///
/// ui.selectable_value(&mut my_enum, Enum::First, "First");
///
/// // is equivalent to:
///
/// if ui.add(egui::SelectableLabel::new(my_enum == Enum::First, "First")).clicked() {
///     my_enum = Enum::First
/// }
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Debug)]
pub struct SelectableLabel {
    selected: bool,
    text: String,
    text_style: Option<TextStyle>,
}

impl SelectableLabel {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(selected: bool, text: impl ToString) -> Self {
        Self {
            selected,
            text: text.to_string(),
            text_style: None,
        }
    }

    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }
}

impl Widget for SelectableLabel {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            selected,
            text,
            text_style,
        } = self;

        let text_style = text_style
            .or(ui.style().override_text_style)
            .unwrap_or(TextStyle::Button);

        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding;

        let galley = if ui.wrap_text() {
            ui.fonts()
                .layout_multiline(text_style, text, ui.available_width() - total_extra.x)
        } else {
            ui.fonts().layout_no_wrap(text_style, text)
        };

        let mut desired_size = total_extra + galley.size;
        desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, selected, &galley.text)
        });

        let text_pos = ui
            .layout()
            .align_size_within_rect(galley.size, rect.shrink2(button_padding))
            .min;

        let visuals = ui.style().interact_selectable(&response, selected);

        if selected || response.hovered() || response.has_focus() {
            let rect = rect.expand(visuals.expansion);

            let corner_radius = 2.0;
            ui.painter()
                .rect(rect, corner_radius, visuals.bg_fill, visuals.bg_stroke);
        }

        let text_color = ui
            .style()
            .visuals
            .override_text_color
            .unwrap_or_else(|| visuals.text_color());
        ui.painter().galley(text_pos, galley, text_color);
        response
    }
}
