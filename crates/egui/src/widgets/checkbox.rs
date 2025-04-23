use crate::AtomicKind::Custom;
use crate::{
    epaint, pos2, vec2, AtomicLayout, Atomics, Id, IntoAtomics, NumExt, Response, Sense, Shape,
    TextStyle, Ui, Vec2, Widget, WidgetInfo, WidgetText, WidgetType,
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
    atomics: Atomics<'a>,
    indeterminate: bool,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, atomics: impl IntoAtomics<'a>) -> Self {
        Checkbox {
            checked,
            atomics: atomics.into_atomics(),
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
            mut atomics,
            indeterminate,
        } = self;

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;

        let rect_id = Id::new("egui::checkbox");
        atomics.add_front(Custom(rect_id, Vec2::splat(icon_width)));

        let text = atomics.text();

        let mut response = AtomicLayout::new(atomics).sense(Sense::click()).show(ui);

        if response.response.clicked() {
            *checked = !*checked;
            response.response.mark_changed();
        }
        response.response.widget_info(|| {
            if indeterminate {
                WidgetInfo::labeled(
                    WidgetType::Checkbox,
                    ui.is_enabled(),
                    text.clone().unwrap_or("".to_owned()),
                )
            } else {
                WidgetInfo::selected(
                    WidgetType::Checkbox,
                    ui.is_enabled(),
                    *checked,
                    text.clone().unwrap_or("".to_owned()),
                )
            }
        });

        if ui.is_rect_visible(response.response.rect) {
            // let visuals = ui.style().interact_selectable(&response, *checked); // too colorful
            let visuals = ui.style().interact(&response.response);
            let rect = response.custom_rects.get(&rect_id).unwrap().clone();

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
            } else if *checked {
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
    }
}
