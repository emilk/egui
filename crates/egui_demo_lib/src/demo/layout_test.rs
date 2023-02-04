use egui::*;

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
}

impl super::Demo for LayoutTest {
    fn name(&self) -> &'static str {
        "Layout Test"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .show(ctx, |ui| {
                use super::View as _;
                self.ui(ui);
            });
    }
}

impl super::View for LayoutTest {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("Tests and demonstrates the limits of the egui layouts");
        self.content_ui(ui);
        Resize::default()
            .default_size([150.0, 200.0])
            .show(ui, |ui| {
                if self.layout.main_wrap {
                    if self.layout.main_dir.is_horizontal() {
                        ui.allocate_ui(
                            vec2(ui.available_size_before_wrap().x, self.wrap_row_height),
                            |ui| ui.with_layout(self.layout.layout(), demo_ui),
                        );
                    } else {
                        ui.allocate_ui(
                            vec2(self.wrap_column_width, ui.available_size_before_wrap().y),
                            |ui| ui.with_layout(self.layout.layout(), demo_ui),
                        );
                    }
                } else {
                    ui.with_layout(self.layout.layout(), demo_ui);
                }
            });
        ui.label("Resize to see effect");
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
            ui.label("Main Direction:");
            for &dir in &[
                Direction::LeftToRight,
                Direction::RightToLeft,
                Direction::TopDown,
                Direction::BottomUp,
            ] {
                ui.radio_value(&mut self.layout.main_dir, dir, format!("{:?}", dir));
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
                ui.radio_value(&mut self.layout.cross_align, align, format!("{:?}", align));
            }
        });

        ui.checkbox(&mut self.layout.cross_justify, "Cross Justified")
            .on_hover_text("Try to fill full width/height (e.g. buttons)");
    }
}

fn demo_ui(ui: &mut Ui) {
    ui.add(egui::Label::new("Wrapping text followed by example widgets:").wrap(true));
    let mut dummy = false;
    ui.checkbox(&mut dummy, "checkbox");
    ui.radio_value(&mut dummy, false, "radio");
    let _ = ui.button("button");
}
