#[derive(Clone, Copy, Debug, PartialEq)]
enum WidgetType {
    Label,
    Button,
    TextEdit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ManualLayoutTest {
    widget_offset: egui::Vec2,
    widget_size: egui::Vec2,
    widget_type: WidgetType,
    text_edit_contents: String,
}

impl Default for ManualLayoutTest {
    fn default() -> Self {
        Self {
            widget_offset: egui::Vec2::splat(150.0),
            widget_size: egui::vec2(200.0, 100.0),
            widget_type: WidgetType::Button,
            text_edit_contents: crate::LOREM_IPSUM.to_owned(),
        }
    }
}

impl crate::Demo for ManualLayoutTest {
    fn name(&self) -> &'static str {
        "Manual Layout Test"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .resizable(false)
            .open(open)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for ManualLayoutTest {
    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::reset_button(ui, self, "Reset");

        let Self {
            widget_offset,
            widget_size,
            widget_type,
            text_edit_contents,
        } = self;
        ui.horizontal(|ui| {
            ui.label("Test widget:");
            ui.radio_value(widget_type, WidgetType::Button, "Button");
            ui.radio_value(widget_type, WidgetType::Label, "Label");
            ui.radio_value(widget_type, WidgetType::TextEdit, "TextEdit");
        });
        egui::Grid::new("pos_size").show(ui, |ui| {
            ui.label("Widget position:");
            ui.add(egui::Slider::new(&mut widget_offset.x, 0.0..=400.0))
                .on_hover_text("Widget position x");
            ui.add(egui::Slider::new(&mut widget_offset.y, 0.0..=400.0))
                .on_hover_text("Widget position y");
            ui.end_row();

            ui.label("Widget size:");
            ui.add(egui::Slider::new(&mut widget_size.x, 0.0..=400.0))
                .on_hover_text("Widget width");
            ui.add(egui::Slider::new(&mut widget_size.y, 0.0..=400.0))
                .on_hover_text("Widget height");
            ui.end_row();
        });

        let widget_rect =
            widget_rect_from_measurement(ui.min_rect(), *widget_offset, *widget_size, ui.is_rtl());

        ui.add(crate::egui_github_link_file!());

        // Showing how to place a widget anywhere in the [`Ui`]:
        match *widget_type {
            WidgetType::Button => {
                ui.put(widget_rect, egui::Button::new("Example button"));
            }
            WidgetType::Label => {
                ui.put(widget_rect, egui::Label::new("Example label"));
            }
            WidgetType::TextEdit => {
                ui.put(widget_rect, egui::TextEdit::multiline(text_edit_contents));
            }
        }
    }
}

/// Build the manually placed widget's rect from the laid-out content's
/// measurement, anchored on the interface reading direction's start edge.
///
/// In a left-to-right interface the reading-start edge is `measurement.min`
/// and the offset and size extend rightward (the original upstream math).
/// Under a right-to-left interface the reading-start edge is
/// `measurement.max` — the right edge, the same edge every other element in
/// this window hangs from (the window body is re-seeded and laid out there) —
/// and the offset and size extend leftward toward the reading end, so the
/// widget grows reading-forward and the window's reading-start-anchored
/// growth can contain it. The vertical math is identical in both directions
/// (vertical layout is never mirrored). Keyed on the interface-direction
/// flag, the same condition the window's own mirroring uses.
fn widget_rect_from_measurement(
    measurement: egui::Rect,
    widget_offset: egui::Vec2,
    widget_size: egui::Vec2,
    rtl: bool,
) -> egui::Rect {
    if rtl {
        egui::Rect::from_min_size(
            egui::pos2(
                measurement.max.x - widget_offset.x - widget_size.x,
                measurement.min.y + widget_offset.y,
            ),
            widget_size,
        )
    } else {
        egui::Rect::from_min_size(measurement.min + widget_offset, widget_size)
    }
}

#[cfg(test)]
mod tests {
    use super::widget_rect_from_measurement;

    fn measurement() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(400.0, 300.0))
    }

    fn offset() -> egui::Vec2 {
        egui::vec2(150.0, 25.0)
    }

    fn size() -> egui::Vec2 {
        egui::vec2(200.0, 100.0)
    }

    #[test]
    fn ltr_matches_the_original_formula() {
        assert_eq!(
            widget_rect_from_measurement(measurement(), offset(), size(), false),
            egui::Rect::from_min_size(measurement().min + offset(), size()),
            "the LTR branch must stay byte-identical to the original expression"
        );
    }

    #[test]
    fn rtl_anchors_at_the_reading_start_edge() {
        let measurement = measurement();
        let offset = offset();
        let size = size();
        let ltr = widget_rect_from_measurement(measurement, offset, size, false);
        let rtl = widget_rect_from_measurement(measurement, offset, size, true);
        assert_eq!(rtl.max.x, measurement.max.x - offset.x);
        assert_eq!(rtl.min.x, rtl.max.x - size.x);
        // The vertical math is identical in both directions:
        assert_eq!(rtl.min.y, ltr.min.y);
        assert_eq!(rtl.max.y, ltr.max.y);
    }

    #[test]
    fn rtl_growth_is_reading_start_anchored() {
        let measurement = measurement();
        let offset = offset();
        let size = size();
        let small = widget_rect_from_measurement(measurement, offset, size, true);
        let big = widget_rect_from_measurement(
            measurement,
            offset,
            egui::vec2(size.x + 200.0, size.y),
            true,
        );
        assert_eq!(big.max.x, small.max.x, "the reading-start edge stays fixed");
        assert_eq!(
            big.min.x,
            small.min.x - 200.0,
            "growing extends toward the reading end (leftward)"
        );
    }

    #[test]
    fn ltr_growth_is_reading_start_anchored() {
        let measurement = measurement();
        let offset = offset();
        let size = size();
        let small = widget_rect_from_measurement(measurement, offset, size, false);
        let big = widget_rect_from_measurement(
            measurement,
            offset,
            egui::vec2(size.x + 200.0, size.y),
            false,
        );
        assert_eq!(big.min.x, small.min.x, "the reading-start edge stays fixed");
        assert_eq!(
            big.max.x,
            small.max.x + 200.0,
            "growing extends toward the reading end (rightward)"
        );
    }

    #[test]
    fn degenerate_inputs_mirror_the_anchor_without_nan_or_inversion() {
        let measurement = measurement();
        let offset = offset();
        let zero_offset = egui::Vec2::ZERO;
        let zero_size = egui::Vec2::ZERO;

        // Zero offset: flush with the reading-start edge in both directions.
        let ltr = widget_rect_from_measurement(measurement, zero_offset, size(), false);
        assert_eq!(ltr.min.x, measurement.min.x);
        let rtl = widget_rect_from_measurement(measurement, zero_offset, size(), true);
        assert_eq!(rtl.max.x, measurement.max.x);

        // Zero size: degenerate rect at the same place as LTR's, mirrored,
        // still finite and non-negative (no NaN or inverted edges).
        let ltr_point = widget_rect_from_measurement(measurement, offset, zero_size, false);
        let rtl_point = widget_rect_from_measurement(measurement, offset, zero_size, true);
        for rect in [ltr_point, rtl_point] {
            assert!(rect.is_finite(), "{rect:?} must stay finite");
            assert!(!rect.any_nan(), "{rect:?} must have no NaN edges");
            assert!(!rect.is_negative(), "{rect:?} must not be inverted");
        }
        assert_eq!(rtl_point.max.x, measurement.max.x - offset.x);
        assert_eq!(rtl_point.min.x, rtl_point.max.x);
        assert_eq!(rtl_point.min.y, ltr_point.min.y);
        assert_eq!(rtl_point.max.y, ltr_point.max.y);
    }
}
