use emath::Rect;

use crate::{
    Atom, AtomLayout, Atoms, Id, IntoAtoms, NumExt as _, Response, Sense, Shape, Ui, Vec2, Widget,
    WidgetInfo, WidgetType, epaint, pos2, widget_style::CheckboxStyle,
};

// TODO(emilk): allow checkbox without a text label
/// Boolean on/off control with text label.
///
/// Usually you'd use [`Ui::checkbox`] instead.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_bool = true;
/// // These are equivalent:
/// ui.checkbox(&mut my_bool, "Checked");
/// ui.add(egui::Checkbox::new(&mut my_bool, "Checked"));
/// # });
/// ```
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    atoms: Atoms<'a>,
    indeterminate: bool,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, atoms: impl IntoAtoms<'a>) -> Self {
        Checkbox {
            checked,
            atoms: atoms.into_atoms(),
            indeterminate: false,
        }
    }

    pub fn without_text(checked: &'a mut bool) -> Self {
        Self::new(checked, ())
    }

    /// Display an indeterminate state (neither checked nor unchecked)
    ///
    /// This only affects the checkbox's appearance. It will still toggle its boolean value when
    /// clicked.
    #[inline]
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Checkbox {
            checked,
            mut atoms,
            indeterminate,
        } = self;

        // Get the widget style by reading the response from the previous pass
        let id = ui.next_auto_id();
        let response: Option<Response> = ui.ctx().read_response(id);
        let state = response.map(|r| r.widget_state()).unwrap_or_default();

        let CheckboxStyle {
            check_size,
            checkbox_frame,
            checkbox_size,
            frame,
            check_stroke,
            text_style,
        } = ui.style().checkbox_style(state);

        let mut min_size = Vec2::splat(ui.spacing().interact_size.y);
        min_size.y = min_size.y.at_least(checkbox_size);

        // In order to center the checkbox based on min_size we set the icon height to at least min_size.y
        let mut icon_size = Vec2::splat(checkbox_size);
        icon_size.y = icon_size.y.at_least(min_size.y);
        let rect_id = Id::new("egui::checkbox");
        atoms.push_left(Atom::custom(rect_id, icon_size));

        let text = atoms.text().map(String::from);

        let mut prepared = AtomLayout::new(atoms)
            .sense(Sense::click())
            .min_size(min_size)
            .frame(frame)
            .allocate(ui);

        if prepared.response.clicked() {
            *checked = !*checked;
            prepared.response.mark_changed();
        }
        prepared.response.widget_info(|| {
            if indeterminate {
                WidgetInfo::labeled(
                    WidgetType::Checkbox,
                    ui.is_enabled(),
                    text.as_deref().unwrap_or(""),
                )
            } else {
                WidgetInfo::selected(
                    WidgetType::Checkbox,
                    ui.is_enabled(),
                    *checked,
                    text.as_deref().unwrap_or(""),
                )
            }
        });

        if ui.is_rect_visible(prepared.response.rect) {
            prepared.fallback_text_color = text_style.color;
            let response = prepared.paint(ui);

            if let Some(rect) = response.rect(rect_id) {
                let big_icon_rect = Rect::from_center_size(
                    pos2(rect.left() + checkbox_size / 2.0, rect.center().y),
                    Vec2::splat(checkbox_size),
                );
                let small_icon_rect =
                    Rect::from_center_size(big_icon_rect.center(), Vec2::splat(check_size));
                ui.painter().add(epaint::RectShape::new(
                    big_icon_rect.expand(checkbox_frame.inner_margin.left.into()),
                    checkbox_frame.corner_radius,
                    checkbox_frame.fill,
                    checkbox_frame.stroke,
                    epaint::StrokeKind::Inside,
                ));

                if indeterminate {
                    // Horizontal line:
                    ui.painter().add(Shape::hline(
                        small_icon_rect.x_range(),
                        small_icon_rect.center().y,
                        check_stroke,
                    ));
                } else if *checked {
                    // Check mark:
                    ui.painter().add(Shape::line(
                        vec![
                            pos2(small_icon_rect.left(), small_icon_rect.center().y),
                            pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                            pos2(small_icon_rect.right(), small_icon_rect.top()),
                        ],
                        check_stroke,
                    ));
                }
            }
            response.response
        } else {
            prepared.response
        }
    }
}
