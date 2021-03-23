use egui::*;

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct LayoutTest {
    // Identical to contents of `egui::Layout`
    main_dir: Direction,
    main_wrap: bool,
    cross_align: Align,
    cross_justify: bool,

    // Extra for testing wrapping:
    wrap_column_width: f32,
    wrap_row_height: f32,
}

impl Default for LayoutTest {
    fn default() -> Self {
        Self {
            main_dir: Direction::TopDown,
            main_wrap: false,
            cross_align: Align::Min,
            cross_justify: false,
            wrap_column_width: 150.0,
            wrap_row_height: 20.0,
        }
    }
}

impl super::Demo for LayoutTest {
    fn name(&self) -> &'static str {
        "Layout Test"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .show(ctx, |ui| {
                use super::View;
                self.ui(ui);
            });
    }
}

impl super::View for LayoutTest {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("Tests and demonstrates the limits of the egui layouts");
        self.content_ui(ui);
        Resize::default()
            .default_size([300.0, 200.0])
            .show(ui, |ui| {
                if self.main_wrap {
                    if self.main_dir.is_horizontal() {
                        ui.allocate_ui(
                            vec2(
                                ui.available_size_before_wrap_finite().x,
                                self.wrap_row_height,
                            ),
                            |ui| ui.with_layout(self.layout(), demo_ui),
                        );
                    } else {
                        ui.allocate_ui(
                            vec2(
                                self.wrap_column_width,
                                ui.available_size_before_wrap_finite().y,
                            ),
                            |ui| ui.with_layout(self.layout(), demo_ui),
                        );
                    }
                } else {
                    ui.with_layout(self.layout(), demo_ui);
                }
            });
        ui.label("Resize to see effect");
    }
}

impl LayoutTest {
    fn layout(&self) -> Layout {
        Layout::from_main_dir_and_cross_align(self.main_dir, self.cross_align)
            .with_main_wrap(self.main_wrap)
            .with_cross_justify(self.cross_justify)
    }

    pub fn content_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Top-down").clicked() {
                *self = Default::default();
            }
            if ui.button("Top-down, centered and justified").clicked() {
                *self = Default::default();
                self.cross_align = Align::Center;
                self.cross_justify = true;
            }
            if ui.button("Horizontal wrapped").clicked() {
                *self = Default::default();
                self.main_dir = Direction::LeftToRight;
                self.cross_align = Align::Center;
                self.main_wrap = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Main Direction:");
            for &dir in &[
                Direction::LeftToRight,
                Direction::RightToLeft,
                Direction::TopDown,
                Direction::BottomUp,
            ] {
                ui.radio_value(&mut self.main_dir, dir, format!("{:?}", dir));
            }
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.main_wrap, "Main wrap")
                .on_hover_text("Wrap when next widget doesn't fit the current row/column");

            if self.main_wrap {
                if self.main_dir.is_horizontal() {
                    ui.add(Slider::f32(&mut self.wrap_row_height, 0.0..=200.0).text("Row height"));
                } else {
                    ui.add(
                        Slider::f32(&mut self.wrap_column_width, 0.0..=200.0).text("Column width"),
                    );
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Cross Align:");
            for &align in &[Align::Min, Align::Center, Align::Max] {
                ui.radio_value(&mut self.cross_align, align, format!("{:?}", align));
            }
        });

        ui.checkbox(&mut self.cross_justify, "Cross Justified")
            .on_hover_text("Try to fill full width/height (e.g. buttons)");
    }
}

fn demo_ui(ui: &mut Ui) {
    ui.monospace("Example widgets:");
    for _ in 0..3 {
        ui.label("label");
    }
    for _ in 0..3 {
        let mut dummy = false;
        ui.checkbox(&mut dummy, "checkbox");
    }
    for _ in 0..3 {
        let _ = ui.button("button");
    }
}
