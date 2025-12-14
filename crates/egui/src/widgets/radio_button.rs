use crate::{
    Atom, AtomLayout, Atoms, Id, IntoAtoms, NumExt as _, Response, Sense, Ui, Vec2, Widget,
    WidgetInfo, WidgetType, epaint,
};

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
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct RadioButton<'a> {
    checked: bool,
    atoms: Atoms<'a>,
}

impl<'a> RadioButton<'a> {
    pub fn new(checked: bool, atoms: impl IntoAtoms<'a>) -> Self {
        Self {
            checked,
            atoms: atoms.into_atoms(),
        }
    }
}

impl Widget for RadioButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { checked, mut atoms } = self;

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;

        let mut min_size = Vec2::splat(spacing.interact_size.y);
        min_size.y = min_size.y.at_least(icon_width);

        // In order to center the checkbox based on min_size we set the icon height to at least min_size.y
        let mut icon_size = Vec2::splat(icon_width);
        icon_size.y = icon_size.y.at_least(min_size.y);
        let rect_id = Id::new("egui::radio_button");
        atoms.push_left(Atom::custom(rect_id, icon_size));

        let text = atoms.text().map(String::from);

        let mut prepared = AtomLayout::new(atoms)
            .sense(Sense::click())
            .min_size(min_size)
            .allocate(ui);

        prepared.response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::RadioButton,
                ui.is_enabled(),
                checked,
                text.as_deref().unwrap_or(""),
            )
        });

        if ui.is_rect_visible(prepared.response.rect) {
            // let visuals = ui.style().interact_selectable(&response, checked); // too colorful
            let visuals = *ui.style().interact(&prepared.response);

            prepared.fallback_text_color = visuals.text_color();
            let response = prepared.paint(ui);

            if let Some(rect) = response.rect(rect_id) {
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
            }
            response.response
        } else {
            prepared.response
        }
    }
}
