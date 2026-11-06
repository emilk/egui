use egui::{Align, Direction, Layout, Resize, Slider, Ui, vec2};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct LayoutTest {
    // Identical to contents of `egui::Layout`
    layout: LayoutSettings,

    // Extra for testing wrapping:
    wrap_column_width: f32,
    wrap_row_height: f32,
}

impl Default for LayoutTest {
    fn default() -> Self {
        Self {
            layout: LayoutSettings::top_down(),
            wrap_column_width: 150.0,
            wrap_row_height: 20.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct LayoutSettings {
    // Similar to the contents of `egui::Layout`
    main_dir: Direction,
    main_wrap: bool,
    cross_align: Align,
    cross_justify: bool,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self::top_down()
    }
}

impl LayoutSettings {
    fn top_down() -> Self {
        Self {
            main_dir: Direction::TopDown,
            main_wrap: false,
            cross_align: Align::Min,
            cross_justify: false,
        }
    }

    fn top_down_justified_centered() -> Self {
        Self {
            main_dir: Direction::TopDown,
            main_wrap: false,
            cross_align: Align::Center,
            cross_justify: true,
        }
    }

    fn horizontal_wrapped() -> Self {
        Self {
            main_dir: Direction::LeftToRight,
            main_wrap: true,
            cross_align: Align::Center,
            cross_justify: false,
        }
    }

    fn layout(&self) -> Layout {
        Layout::from_main_dir_and_cross_align(self.main_dir, self.cross_align)
            .with_main_wrap(self.main_wrap)
            .with_cross_justify(self.cross_justify)
    }

    /// [`Layout`] resolved for the interface reading direction.
    ///
    /// A left-to-right main direction is mirrored to right-to-left under a
    /// right-to-left interface, so the row follows the reading order — the same
    /// resolution the [`Ui::horizontal`](egui::Ui::horizontal) helpers apply.
    /// A main direction that is already right-to-left, and all vertical
    /// directions (which core mirrors itself), are kept. Everything else
    /// (main wrap, cross align, cross justify) is kept as configured.
    fn layout_for_reading_direction(&self, ui_is_rtl: bool) -> Layout {
        let mut layout = self.layout();
        if ui_is_rtl && layout.main_dir == Direction::LeftToRight {
            layout.main_dir = Direction::RightToLeft;
        }
        layout
    }
}

impl crate::Demo for LayoutTest {
    fn name(&self) -> &'static str {
        "Layout Test"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for LayoutTest {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("Tests and demonstrates the limits of the egui layouts");
        self.content_ui(ui);
        Resize::default()
            .default_size([150.0, 200.0])
            .show(ui, |ui| {
                let layout = self.layout.layout_for_reading_direction(ui.is_rtl());
                if self.layout.main_wrap {
                    if self.layout.main_dir.is_horizontal() {
                        ui.allocate_ui(
                            vec2(ui.available_size_before_wrap().x, self.wrap_row_height),
                            |ui| ui.with_layout(layout, demo_ui),
                        );
                    } else {
                        ui.allocate_ui(
                            vec2(self.wrap_column_width, ui.available_size_before_wrap().y),
                            |ui| ui.with_layout(layout, demo_ui),
                        );
                    }
                } else {
                    ui.with_layout(layout, demo_ui);
                }
            });
        ui.label("Resize to see effect");

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}

impl LayoutTest {
    pub fn content_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.layout, LayoutSettings::top_down(), "Top-down");
            ui.selectable_value(
                &mut self.layout,
                LayoutSettings::top_down_justified_centered(),
                "Top-down, centered and justified",
            );
            ui.selectable_value(
                &mut self.layout,
                LayoutSettings::horizontal_wrapped(),
                "Horizontal wrapped",
            );
        });

        ui.horizontal(|ui| {
            ui.label("Main Direction:").on_hover_text(
                "Under a right-to-left interface the horizontal directions are \
                 applied relative to the reading direction: a LeftToRight row is \
                 laid out right-to-left so it follows the reading order",
            );
            for &dir in &[
                Direction::LeftToRight,
                Direction::RightToLeft,
                Direction::TopDown,
                Direction::BottomUp,
            ] {
                ui.radio_value(&mut self.layout.main_dir, dir, format!("{dir:?}"));
            }
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.layout.main_wrap, "Main wrap")
                .on_hover_text("Wrap when next widget doesn't fit the current row/column");

            if self.layout.main_wrap {
                if self.layout.main_dir.is_horizontal() {
                    ui.add(Slider::new(&mut self.wrap_row_height, 0.0..=200.0).text("Row height"));
                } else {
                    ui.add(
                        Slider::new(&mut self.wrap_column_width, 0.0..=200.0).text("Column width"),
                    );
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Cross Align:");
            for &align in &[Align::Min, Align::Center, Align::Max] {
                ui.radio_value(&mut self.layout.cross_align, align, format!("{align:?}"));
            }
        });

        ui.checkbox(&mut self.layout.cross_justify, "Cross Justified")
            .on_hover_text("Try to fill full width/height (e.g. buttons)");
    }
}

fn demo_ui(ui: &mut Ui) {
    ui.add(egui::Label::new("Wrapping text followed by example widgets:").wrap());
    let mut dummy = false;
    ui.checkbox(&mut dummy, "checkbox");
    ui.radio_value(&mut dummy, false, "radio");
    let _ = ui.button("button");
}

#[cfg(test)]
mod tests {
    use super::LayoutSettings;

    fn settings(main_dir: egui::Direction, main_wrap: bool) -> LayoutSettings {
        LayoutSettings {
            main_dir,
            main_wrap,
            cross_align: egui::Align::Center,
            cross_justify: true,
        }
    }

    #[test]
    fn rtl_interface_mirrors_left_to_right_rows() {
        let layout =
            settings(egui::Direction::LeftToRight, true).layout_for_reading_direction(true);
        assert_eq!(layout.main_dir, egui::Direction::RightToLeft);
        // Everything else is kept as configured:
        assert!(layout.main_wrap);
        assert_eq!(layout.cross_align, egui::Align::Center);
        assert!(layout.cross_justify);
    }

    #[test]
    fn rtl_interface_keeps_right_to_left_and_vertical_directions() {
        for dir in [
            egui::Direction::RightToLeft,
            egui::Direction::TopDown,
            egui::Direction::BottomUp,
        ] {
            let layout = settings(dir, false).layout_for_reading_direction(true);
            assert_eq!(layout.main_dir, dir, "{dir:?} must not change under RTL");
        }
    }

    #[test]
    fn ltr_interface_keeps_every_direction() {
        for dir in [
            egui::Direction::LeftToRight,
            egui::Direction::RightToLeft,
            egui::Direction::TopDown,
            egui::Direction::BottomUp,
        ] {
            let layout = settings(dir, true).layout_for_reading_direction(false);
            assert_eq!(layout.main_dir, dir, "{dir:?} must not change under LTR");
            assert!(layout.main_wrap);
        }
    }
}
