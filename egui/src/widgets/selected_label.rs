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
pub struct SelectableLabel {
    selected: bool,
    text: WidgetText,
}

impl SelectableLabel {
    pub fn new(selected: bool, text: impl Into<WidgetText>) -> Self {
        Self {
            selected,
            text: text.into(),
        }
    }

    #[deprecated = "Replaced by: Button::new(RichText::new(text).text_style(…))"]
    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text = self.text.text_style(text_style);
        self
    }
}

impl Widget for SelectableLabel {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { selected, text } = self;

        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding;

        let wrap_width = ui.available_width() - total_extra.x;
        let text = text.layout(ui, None, wrap_width, TextStyle::Button);

        let mut desired_size = total_extra + text.size();
        desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, selected, text.text())
        });

        let text_pos = ui
            .layout()
            .align_size_within_rect(text.size(), rect.shrink2(button_padding))
            .min;

        let visuals = ui.style().interact_selectable(&response, selected);

        if selected || response.hovered() || response.has_focus() {
            let rect = rect.expand(visuals.expansion);

            let corner_radius = 2.0;
            ui.painter()
                .rect(rect, corner_radius, visuals.bg_fill, visuals.bg_stroke);
        }

        text.paint(ui, text_pos, &visuals);
        response
    }
}
