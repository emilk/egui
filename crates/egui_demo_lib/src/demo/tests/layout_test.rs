use egui::{Align, Direction, Layout, Resize, Slider, Ui, vec2};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct LayoutTest {
    // Identical to contents of `egui::Layout`
    layout: LayoutSettings,

    // Extra for testing wrapping:
    wrap_column_width: f32,
    wrap_row_height: f32,

    // Improve UX around `main_justify`
    restrict_resize: Restriction,
    single_element: bool,
}

impl Default for LayoutTest {
    fn default() -> Self {
        Self {
            layout: LayoutSettings::top_down(),
            wrap_column_width: 150.0,
            wrap_row_height: 20.0,
            restrict_resize: Restriction::None,
            single_element: false,
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
    main_align: Align,
    main_justify: bool,
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
            main_align: Align::Center,
            main_justify: false,
            cross_align: Align::Min,
            cross_justify: false,
        }
    }

    fn top_down_justified_centered() -> Self {
        Self {
            main_dir: Direction::TopDown,
            main_wrap: false,
            main_align: Align::Center,
            main_justify: false,
            cross_align: Align::Center,
            cross_justify: true,
        }
    }

    fn horizontal_wrapped() -> Self {
        Self {
            main_dir: Direction::LeftToRight,
            main_wrap: true,
            main_align: Align::Center,
            main_justify: false,
            cross_align: Align::Center,
            cross_justify: false,
        }
    }

    fn layout(&self) -> Layout {
        Layout::from_main_dir_and_cross_align(self.main_dir, self.cross_align)
            .with_main_wrap(self.main_wrap)
            .with_cross_justify(self.cross_justify)
            .with_main_align(self.main_align)
            .with_main_justify(self.main_justify)
    }
}

impl crate::Demo for LayoutTest {
    fn name(&self) -> &'static str {
        "Layout Test"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .show(ctx, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for LayoutTest {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("Tests and demonstrates the limits of the egui layouts");
        self.content_ui(ui);
        let area = Resize::default().default_size([150.0, 200.0]);
        let layout = self.layout.layout();
        let button_only = self.single_element;
        match self.restrict_resize {
            Restriction::None => self.demo_area(
                ui,
                area,
                |ui| ui.with_layout(layout, |ui| demo_ui(ui, button_only)),
            ),
            Restriction::AllocateUi => self.demo_area(
                ui,
                area,
                |ui| ui.allocate_ui_with_layout(
                    [RESIZE_WIDTH, RESIZE_HEIGHT].into(),
                    layout,
                    |ui| demo_ui(ui, button_only),
                ),
            ),
            Restriction::MaximumSize => self.demo_area(
                ui,
                area.max_size([RESIZE_WIDTH, RESIZE_HEIGHT]),
                |ui| ui.with_layout(layout, |ui| demo_ui(ui, button_only)),
            ),
        }
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
            ui.label("Main Direction:");
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
            ui.label("Main Align:");
            for &align in &[Align::Min, Align::Center, Align::Max] {
                ui.radio_value(&mut self.layout.main_align, align, format!("{align:?}"));
            }
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.layout.main_justify, "Main Justified")
                .on_hover_text("Try to fill full width/height (e.g. buttons)");
            if !self.layout.main_wrap &&
                !self.single_element &&
                (self.restrict_resize.is_none() || self.layout.main_justify)
            {
                ui.label(
                    egui::RichText::new(
                        "⚠ Unrestricted main justify with multiple elements results in infinite resize"
                    ).color(egui::Color32::ORANGE));
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

        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.label("Limit Resize:");
            ui.selectable_value(&mut self.restrict_resize, Restriction::None, "None");
            ui.selectable_value(&mut self.restrict_resize, Restriction::AllocateUi, "Allocate")
                .on_hover_text(format!("Allocate area of {RESIZE_WIDTH}x{RESIZE_HEIGHT}"));
            ui.selectable_value(&mut self.restrict_resize, Restriction::MaximumSize, "Max size")
                .on_hover_text(format!("Maximum size of {RESIZE_WIDTH}x{RESIZE_HEIGHT}"));
            ui.separator();
            ui.checkbox(&mut self.single_element, "Button only")
                .on_hover_text("Include only the button");
        });
        ui.add_space(10.0);
    }

    pub fn demo_area(&mut self, ui: &mut Ui, area: Resize, inner: impl FnOnce(&mut Ui) -> egui::InnerResponse<()>) {
        area.show(ui, |ui| {
            if self.layout.main_wrap {
                if self.layout.main_dir.is_horizontal() {
                    ui.allocate_ui(
                        vec2(ui.available_size_before_wrap().x, self.wrap_row_height),
                        |ui| inner(ui),
                    );
                } else {
                    ui.allocate_ui(
                        vec2(self.wrap_column_width, ui.available_size_before_wrap().y),
                        |ui| inner(ui),
                    );
                }
            } else {
                inner(ui);
            }
        });
    }
}

fn demo_ui(ui: &mut Ui, single_widget: bool) {
    if !single_widget {
        ui.add(egui::Label::new("Wrapping text followed by example widgets:").wrap());
        let mut dummy = false;
        ui.checkbox(&mut dummy, "checkbox");
        ui.radio_value(&mut dummy, false, "radio");
    }
    let _ = ui.add(egui::Button::new("button").min_size([100.0, 100.0].into()));
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(PartialEq)]
enum Restriction {
    None,
    AllocateUi,
    MaximumSize,
}

impl Restriction {
    fn is_none(&self) -> bool {
        self == &Restriction::None
    }
}

const RESIZE_WIDTH: f32 = 500.0;
const RESIZE_HEIGHT: f32 = 400.0;
