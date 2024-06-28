use crate::*;

/// One out of several alternatives, either selected or not.
///
/// Usually you'd use [`Ui::radio_value`] or [`Ui::radio`] instead.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// #[derive(PartialEq)]
/// enum Enum { First, Second, Third }
/// let mut my_enum = Enum::First;
///
/// ui.radio_value(&mut my_enum, Enum::First, "First");
///
/// // is equivalent to:
///
/// if ui.add(egui::RadioButton::new(my_enum == Enum::First, "First")).clicked() {
///     my_enum = Enum::First
/// }
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct RadioButton {
    checked: bool,
    text: WidgetText,
}

impl RadioButton {
    pub fn new(checked: bool, text: impl Into<WidgetText>) -> Self {
        Self {
            checked,
            text: text.into(),
        }
    }
}

impl Widget for RadioButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { checked, text } = self;

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;
        let icon_spacing = spacing.icon_spacing;

        let (galley, mut desired_size) = if text.is_empty() {
            (None, vec2(icon_width, 0.0))
        } else {
            let total_extra = vec2(icon_width + icon_spacing, 0.0);

            let wrap_width = ui.available_width() - total_extra.x;
            let text = text.into_galley(ui, None, wrap_width, TextStyle::Button);

            let mut desired_size = total_extra + text.size();
            desired_size = desired_size.at_least(spacing.interact_size);

            (Some(text), desired_size)
        };

        desired_size = desired_size.at_least(Vec2::splat(spacing.interact_size.y));
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::RadioButton,
                ui.is_enabled(),
                checked,
                galley.as_ref().map_or("", |x| x.text()),
            )
        });

        if ui.is_rect_visible(rect) {
            // let visuals = ui.style().interact_selectable(&response, checked); // too colorful
            let visuals = ui.style().interact(&response);

            let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);

            let painter = ui.painter();

            painter.add(epaint::CircleShape {
                center: big_icon_rect.center(),
                radius: big_icon_rect.width() / 2.0 + visuals.expansion,
                fill: visuals.bg_fill,
                stroke: visuals.bg_stroke,
            });

            if checked {
                painter.add(epaint::CircleShape {
                    center: small_icon_rect.center(),
                    radius: small_icon_rect.width() / 3.0,
                    fill: visuals.fg_stroke.color, // Intentional to use stroke and not fill
                    // fill: ui.visuals().selection.stroke.color, // too much color
                    stroke: Default::default(),
                });
            }

            if let Some(galley) = galley {
                let text_pos = pos2(
                    rect.min.x + icon_width + icon_spacing,
                    rect.center().y - 0.5 * galley.size().y,
                );
                ui.painter().galley(text_pos, galley, visuals.text_color());
            }
        }

        response
    }
}
