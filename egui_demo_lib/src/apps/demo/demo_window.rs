use super::*;
use egui::{color::*, *};

/// Showcase some ui code
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct DemoWindow {
    num_columns: usize,

    widgets: Widgets,
    colors: ColorWidgets,
    tree: Tree,
    box_painting: BoxPainting,
}

impl Default for DemoWindow {
    fn default() -> DemoWindow {
        DemoWindow {
            num_columns: 2,

            widgets: Default::default(),
            colors: Default::default(),
            tree: Tree::demo(),
            box_painting: Default::default(),
        }
    }
}

impl Demo for DemoWindow {
    fn name(&self) -> &'static str {
        "✨ Misc Demos"
    }

    fn show(&mut self, ctx: &CtxRef, open: &mut bool) {
        Window::new(self.name())
            .open(open)
            .scroll(true)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl View for DemoWindow {
    fn ui(&mut self, ui: &mut Ui) {
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

        CollapsingHeader::new("Tree")
            .default_open(false)
            .show(ui, |ui| self.tree.ui(ui));

        ui.collapsing("Columns", |ui| {
            ui.add(Slider::new(&mut self.num_columns, 1..=10).text("Columns"));
            ui.columns(self.num_columns, |cols| {
                for (i, col) in cols.iter_mut().enumerate() {
                    col.label(format!("Column {} out of {}", i + 1, self.num_columns));
                    if i + 1 == self.num_columns && col.button("Delete this").clicked() {
                        self.num_columns -= 1;
                    }
                }
            });
        });

        CollapsingHeader::new("Test box rendering")
            .default_open(false)
            .show(ui, |ui| self.box_painting.ui(ui));

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
                    use std::f32::consts::TAU;
                    let (rect, _response) = ui.allocate_at_least(Vec2::splat(16.0), Sense::hover());
                    let painter = ui.painter();
                    let c = rect.center();
                    let r = rect.width() / 2.0 - 1.0;
                    let color = Color32::from_gray(128);
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

#[derive(PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
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
        egui::reset_button(ui, self);

        ui.label("egui lets you edit colors stored as either sRGBA or linear RGBA and with or without premultiplied alpha");

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

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
struct BoxPainting {
    size: Vec2,
    corner_radius: f32,
    stroke_width: f32,
    num_boxes: usize,
}

impl Default for BoxPainting {
    fn default() -> Self {
        Self {
            size: vec2(64.0, 32.0),
            corner_radius: 5.0,
            stroke_width: 2.0,
            num_boxes: 1,
        }
    }
}

impl BoxPainting {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.add(Slider::new(&mut self.size.x, 0.0..=500.0).text("width"));
        ui.add(Slider::new(&mut self.size.y, 0.0..=500.0).text("height"));
        ui.add(Slider::new(&mut self.corner_radius, 0.0..=50.0).text("corner_radius"));
        ui.add(Slider::new(&mut self.stroke_width, 0.0..=10.0).text("stroke_width"));
        ui.add(Slider::new(&mut self.num_boxes, 0..=8).text("num_boxes"));

        ui.horizontal_wrapped(|ui| {
            for _ in 0..self.num_boxes {
                let (rect, _response) = ui.allocate_at_least(self.size, Sense::hover());
                ui.painter().rect(
                    rect,
                    self.corner_radius,
                    Color32::from_gray(64),
                    Stroke::new(self.stroke_width, Color32::WHITE),
                );
            }
        });
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Keep,
    Delete,
}

#[derive(Clone, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
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
        if depth > 0
            && ui
                .add(Button::new("delete").text_color(Color32::RED))
                .clicked()
        {
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

        if ui.button("+").clicked() {
            self.0.push(Tree::default());
        }

        Action::Keep
    }
}
