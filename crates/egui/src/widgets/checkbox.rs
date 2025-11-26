use crate::{
    Atom, AtomLayout, Atoms, Id, IntoAtoms, NumExt as _, Response, Sense, Shape, Ui, Vec2, Widget,
    WidgetInfo, WidgetType, epaint, pos2,
};
use bytemuck::TransparentWrapper;
use std::ops::{Deref, DerefMut};

#[repr(transparent)]
#[derive(Debug, TransparentWrapper)]
struct BoolWrapper(bool);

impl Deref for BoolWrapper {
    type Target = bool;

    fn deref(&self) -> &bool {
        &self.0
    }
}

impl DerefMut for BoolWrapper {
    fn deref_mut(&mut self) -> &mut bool {
        &mut self.0
    }
}

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
    checked: &'a mut dyn DerefMut<Target = bool>,
    atoms: Atoms<'a>,
    indeterminate: bool,
}

impl<'a> Checkbox<'a> {
    pub fn new_ref(
        checked: &'a mut dyn DerefMut<Target = bool>,
        atoms: impl IntoAtoms<'a>,
    ) -> Self {
        Checkbox {
            checked,
            atoms: atoms.into_atoms(),
            indeterminate: false,
        }
    }

    pub fn new(checked: &'a mut bool, atoms: impl IntoAtoms<'a>) -> Self {
        Self::new_ref(BoolWrapper::wrap_mut(checked), atoms)
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

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;

        let mut min_size = Vec2::splat(spacing.interact_size.y);
        min_size.y = min_size.y.at_least(icon_width);

        // In order to center the checkbox based on min_size we set the icon height to at least min_size.y
        let mut icon_size = Vec2::splat(icon_width);
        icon_size.y = icon_size.y.at_least(min_size.y);
        let rect_id = Id::new("egui::checkbox");
        atoms.push_left(Atom::custom(rect_id, icon_size));

        let text = atoms.text().map(String::from);

        let mut prepared = AtomLayout::new(atoms)
            .sense(Sense::click())
            .min_size(min_size)
            .allocate(ui);

        if prepared.response.clicked() {
            **checked = !**checked;
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
                    **checked,
                    text.as_deref().unwrap_or(""),
                )
            }
        });

        if ui.is_rect_visible(prepared.response.rect) {
            // let visuals = ui.style().interact_selectable(&response, *checked); // too colorful
            let visuals = *ui.style().interact(&prepared.response);
            prepared.fallback_text_color = visuals.text_color();
            let response = prepared.paint(ui);

            if let Some(rect) = response.rect(rect_id) {
                let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
                ui.painter().add(epaint::RectShape::new(
                    big_icon_rect.expand(visuals.expansion),
                    visuals.corner_radius,
                    visuals.bg_fill,
                    visuals.bg_stroke,
                    epaint::StrokeKind::Inside,
                ));

                if indeterminate {
                    // Horizontal line:
                    ui.painter().add(Shape::hline(
                        small_icon_rect.x_range(),
                        small_icon_rect.center().y,
                        visuals.fg_stroke,
                    ));
                } else if **checked {
                    // Check mark:
                    ui.painter().add(Shape::line(
                        vec![
                            pos2(small_icon_rect.left(), small_icon_rect.center().y),
                            pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                            pos2(small_icon_rect.right(), small_icon_rect.top()),
                        ],
                        visuals.fg_stroke,
                    ));
                }
            }
            response.response
        } else {
            prepared.response
        }
    }
}
