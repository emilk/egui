use crate::{color::*, demos::*, *};

/// Showcase some ui code
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DemoWindow {
    num_columns: usize,

    widgets: Widgets,
    colors: ColorWidgets,
    layout: LayoutDemo,
    tree: Tree,
    box_painting: BoxPainting,
    painting: Painting,
}

impl Default for DemoWindow {
    fn default() -> DemoWindow {
        DemoWindow {
            num_columns: 2,

            widgets: Default::default(),
            colors: Default::default(),
            layout: Default::default(),
            tree: Tree::demo(),
            box_painting: Default::default(),
            painting: Default::default(),
        }
    }
}

impl DemoWindow {
    pub fn ui(&mut self, ui: &mut Ui) {
        CollapsingHeader::new("Widgets")
            .default_open(true)
            .show(ui, |ui| {
                self.widgets.ui(ui);
            });

        CollapsingHeader::new("Colors")
            .default_open(false)
            .show(ui, |ui| {
                self.colors.ui(ui);
            });

        CollapsingHeader::new("Layout")
            .default_open(false)
            .show(ui, |ui| self.layout.ui(ui));

        CollapsingHeader::new("Tree")
            .default_open(false)
            .show(ui, |ui| self.tree.ui(ui));

        ui.collapsing("Columns", |ui| {
            ui.add(Slider::usize(&mut self.num_columns, 1..=10).text("Columns"));
            ui.columns(self.num_columns, |cols| {
                for (i, col) in cols.iter_mut().enumerate() {
                    col.label(format!("Column {} out of {}", i + 1, self.num_columns));
                    if i + 1 == self.num_columns && col.button("Delete this").clicked {
                        self.num_columns -= 1;
                    }
                }
            });
        });

        CollapsingHeader::new("Test box rendering")
            .default_open(false)
            .show(ui, |ui| self.box_painting.ui(ui));

        CollapsingHeader::new("Scroll area")
            .default_open(false)
            .show(ui, |ui| {
                ScrollArea::from_max_height(200.0).show(ui, |ui| {
                    ui.label(LOREM_IPSUM_LONG);
                });
            });

        CollapsingHeader::new("Paint with your mouse")
            .default_open(false)
            .show(ui, |ui| self.painting.ui(ui));

        CollapsingHeader::new("Resize")
            .default_open(false)
            .show(ui, |ui| {
                Resize::default().default_height(100.0).show(ui, |ui| {
                    ui.label("This ui can be resized!");
                    ui.label("Just pull the handle on the bottom right");
                });
            });

        CollapsingHeader::new("Misc")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("You can pretty easily paint your own small icons:");
                    let rect = ui.allocate_space(Vec2::splat(16.0));
                    let painter = ui.painter();
                    let c = rect.center();
                    let r = rect.width() / 2.0 - 1.0;
                    let color = Srgba::gray(128);
                    let stroke = Stroke::new(1.0, color);
                    painter.circle_stroke(c, r, stroke);
                    painter.line_segment([c - vec2(0.0, r), c + vec2(0.0, r)], stroke);
                    painter.line_segment([c, c + r * Vec2::angled(TAU * 1.0 / 8.0)], stroke);
                    painter.line_segment([c, c + r * Vec2::angled(TAU * 3.0 / 8.0)], stroke);
                });
            });
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
struct ColorWidgets {
    srgba_unmul: [u8; 4],
    srgba_premul: [u8; 4],
    rgba_unmul: [f32; 4],
    rgba_premul: [f32; 4],
}

impl Default for ColorWidgets {
    fn default() -> Self {
        // Approximately the same color.
        ColorWidgets {
            srgba_unmul: [0, 255, 183, 127],
            srgba_premul: [0, 187, 140, 127],
            rgba_unmul: [0.0, 1.0, 0.5, 0.5],
            rgba_premul: [0.0, 0.5, 0.25, 0.5],
        }
    }
}

impl ColorWidgets {
    fn ui(&mut self, ui: &mut Ui) {
        if ui.button("Reset").clicked {
            *self = Default::default();
        }

        ui.label("Egui lets you edit colors stored as either sRGBA or linear RGBA and with or without premultiplied alpha");

        let Self {
            srgba_unmul,
            srgba_premul,
            rgba_unmul,
            rgba_premul,
        } = self;

        ui.horizontal(|ui| {
            ui.color_edit_button_srgba_unmultiplied(srgba_unmul);
            ui.label(format!(
                "sRGBA: {} {} {} {}",
                srgba_unmul[0], srgba_unmul[1], srgba_unmul[2], srgba_unmul[3],
            ));
        });

        ui.horizontal(|ui| {
            ui.color_edit_button_srgba_premultiplied(srgba_premul);
            ui.label(format!(
                "sRGBA with premultiplied alpha: {} {} {} {}",
                srgba_premul[0], srgba_premul[1], srgba_premul[2], srgba_premul[3],
            ));
        });

        ui.horizontal(|ui| {
            ui.color_edit_button_rgba_unmultiplied(rgba_unmul);
            ui.label(format!(
                "Linear RGBA: {:.02} {:.02} {:.02} {:.02}",
                rgba_unmul[0], rgba_unmul[1], rgba_unmul[2], rgba_unmul[3],
            ));
        });

        ui.horizontal(|ui| {
            ui.color_edit_button_rgba_premultiplied(rgba_premul);
            ui.label(format!(
                "Linear RGBA with premultiplied alpha: {:.02} {:.02} {:.02} {:.02}",
                rgba_premul[0], rgba_premul[1], rgba_premul[2], rgba_premul[3],
            ));
        });
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
struct BoxPainting {
    size: Vec2,
    corner_radius: f32,
    stroke_width: f32,
    num_boxes: usize,
}

impl Default for BoxPainting {
    fn default() -> Self {
        Self {
            size: vec2(100.0, 50.0),
            corner_radius: 5.0,
            stroke_width: 2.0,
            num_boxes: 1,
        }
    }
}

impl BoxPainting {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.add(Slider::f32(&mut self.size.x, 0.0..=500.0).text("width"));
        ui.add(Slider::f32(&mut self.size.y, 0.0..=500.0).text("height"));
        ui.add(Slider::f32(&mut self.corner_radius, 0.0..=50.0).text("corner_radius"));
        ui.add(Slider::f32(&mut self.stroke_width, 0.0..=10.0).text("stroke_width"));
        ui.add(Slider::usize(&mut self.num_boxes, 0..=5).text("num_boxes"));

        let pos = ui
            .allocate_space(vec2(self.size.x * (self.num_boxes as f32), self.size.y))
            .min;

        let mut cmds = vec![];
        for i in 0..self.num_boxes {
            cmds.push(paint::PaintCmd::Rect {
                corner_radius: self.corner_radius,
                fill: Srgba::gray(64),
                rect: Rect::from_min_size(
                    pos2(10.0 + pos.x + (i as f32) * (self.size.x * 1.1), pos.y),
                    self.size,
                ),
                stroke: Stroke::new(self.stroke_width, WHITE),
            });
        }
        ui.painter().extend(cmds);
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
struct Painting {
    lines: Vec<Vec<Vec2>>,
    stroke: Stroke,
}

impl Default for Painting {
    fn default() -> Self {
        Self {
            lines: Default::default(),
            stroke: Stroke::new(1.0, LIGHT_BLUE),
        }
    }
}

impl Painting {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("Draw with your mouse to paint");

        ui.horizontal(|ui| {
            self.stroke.ui(ui, "Stroke");
            if ui.button("Clear").clicked {
                self.lines.clear();
            }
        });

        Resize::default()
            .default_size([200.0, 200.0])
            .show(ui, |ui| self.content(ui));
    }

    fn content(&mut self, ui: &mut Ui) {
        let painter = ui.allocate_painter(ui.available_size_before_wrap_finite());
        let rect = painter.clip_rect();
        let id = ui.make_position_id();
        let response = ui.interact(rect, id, Sense::drag());

        if self.lines.is_empty() {
            self.lines.push(vec![]);
        }

        let current_line = self.lines.last_mut().unwrap();

        if response.active {
            if let Some(mouse_pos) = ui.input().mouse.pos {
                let canvas_pos = mouse_pos - rect.min;
                if current_line.last() != Some(&canvas_pos) {
                    current_line.push(canvas_pos);
                }
            }
        } else if !current_line.is_empty() {
            self.lines.push(vec![]);
        }

        for line in &self.lines {
            if line.len() >= 2 {
                let points: Vec<Pos2> = line.iter().map(|p| rect.min + *p).collect();
                painter.add(paint::PaintCmd::line(points, self.stroke));
            }
        }
    }
}

// ----------------------------------------------------------------------------

use crate::layout::*;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
struct LayoutDemo {
    // Identical to contents of `egui::Layout`
    main_dir: Direction,
    main_wrap: bool,
    cross_align: Align,
    cross_justify: bool,

    // Extra for testing wrapping:
    wrap_column_width: f32,
    wrap_row_height: f32,
}

impl Default for LayoutDemo {
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

impl LayoutDemo {
    fn layout(&self) -> Layout {
        Layout::from_main_dir_and_cross_align(self.main_dir, self.cross_align)
            .with_main_wrap(self.main_wrap)
            .with_cross_justify(self.cross_justify)
    }

    pub fn ui(&mut self, ui: &mut Ui) {
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
                            |ui| ui.with_layout(self.layout(), |ui| self.demo_ui(ui)),
                        );
                    } else {
                        ui.allocate_ui(
                            vec2(
                                self.wrap_column_width,
                                ui.available_size_before_wrap_finite().y,
                            ),
                            |ui| ui.with_layout(self.layout(), |ui| self.demo_ui(ui)),
                        );
                    }
                } else {
                    ui.with_layout(self.layout(), |ui| self.demo_ui(ui));
                }
            });
        ui.label("Resize to see effect");
    }

    pub fn content_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Top-down").clicked {
                *self = Default::default();
            }
            if ui.button("Top-down, centered and justified").clicked {
                *self = Default::default();
                self.cross_align = Align::Center;
                self.cross_justify = true;
            }
            if ui.button("Horizontal wrapped").clicked {
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

    pub fn demo_ui(&mut self, ui: &mut Ui) {
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
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Keep,
    Delete,
}

#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct Tree(Vec<Tree>);

impl Tree {
    pub fn demo() -> Self {
        Self(vec![
            Tree(vec![Tree::default(); 4]),
            Tree(vec![Tree(vec![Tree::default(); 2]); 3]),
        ])
    }
    pub fn ui(&mut self, ui: &mut Ui) -> Action {
        self.ui_impl(ui, 0, "root")
    }

    fn ui_impl(&mut self, ui: &mut Ui, depth: usize, name: &str) -> Action {
        CollapsingHeader::new(name)
            .default_open(depth < 1)
            .show(ui, |ui| self.children_ui(ui, depth))
            .body_returned
            .unwrap_or(Action::Keep)
    }

    fn children_ui(&mut self, ui: &mut Ui, depth: usize) -> Action {
        if depth > 0 && ui.add(Button::new("delete").text_color(color::RED)).clicked {
            return Action::Delete;
        }

        self.0 = std::mem::take(self)
            .0
            .into_iter()
            .enumerate()
            .filter_map(|(i, mut tree)| {
                if tree.ui_impl(ui, depth + 1, &format!("child #{}", i)) == Action::Keep {
                    Some(tree)
                } else {
                    None
                }
            })
            .collect();

        if ui.button("+").clicked {
            self.0.push(Tree::default());
        }

        Action::Keep
    }
}
