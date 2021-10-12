use super::*;
use egui::{color::*, *};

/// Showcase some ui code
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct MiscDemoWindow {
    num_columns: usize,

    widgets: Widgets,
    colors: ColorWidgets,
    tree: Tree,
    box_painting: BoxPainting,
}

impl Default for MiscDemoWindow {
    fn default() -> MiscDemoWindow {
        MiscDemoWindow {
            num_columns: 2,

            widgets: Default::default(),
            colors: Default::default(),
            tree: Tree::demo(),
            box_painting: Default::default(),
        }
    }
}

impl Demo for MiscDemoWindow {
    fn name(&self) -> &'static str {
        "✨ Misc Demos"
    }

    fn show(&mut self, ctx: &CtxRef, open: &mut bool) {
        Window::new(self.name())
            .open(open)
            .vscroll(true)
            .hscroll(true)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl View for MiscDemoWindow {
    fn ui(&mut self, ui: &mut Ui) {
        ui.set_min_width(250.0);

        CollapsingHeader::new("Widgets")
            .default_open(true)
            .show(ui, |ui| {
                self.widgets.ui(ui);
            });

        CollapsingHeader::new("Text layout")
            .default_open(false)
            .show(ui, |ui| {
                text_layout_ui(ui);
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
                    let size = Vec2::splat(16.0);
                    let (response, painter) = ui.allocate_painter(size, Sense::hover());
                    let rect = response.rect;
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

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Widgets {
    angle: f32,
    password: String,
}

impl Default for Widgets {
    fn default() -> Self {
        Self {
            angle: std::f32::consts::TAU / 3.0,
            password: "hunter2".to_owned(),
        }
    }
}

impl Widgets {
    pub fn ui(&mut self, ui: &mut Ui) {
        let Self { angle, password } = self;
        ui.vertical_centered(|ui| {
            ui.add(crate::__egui_github_link_file_line!());
        });

        ui.horizontal_wrapped(|ui| {
            // Trick so we don't have to add spaces in the text below:
            ui.spacing_mut().item_spacing.x = ui.fonts()[TextStyle::Body].glyph_width(' ');

            ui.add(Label::new("Text can have").text_color(Color32::from_rgb(110, 255, 110)));
            ui.colored_label(Color32::from_rgb(128, 140, 255), "color"); // Shortcut version
            ui.label("and tooltips.").on_hover_text(
                "This is a multiline tooltip that demonstrates that you can easily add tooltips to any element.\nThis is the second line.\nThis is the third.",
            );

            ui.label("You can mix in other widgets into text, like");
            let _ = ui.small_button("this button");
            ui.label(".");

            ui.label("The default font supports all latin and cyrillic characters (ИÅđ…), common math symbols (∫√∞²⅓…), and many emojis (💓🌟🖩…).")
                .on_hover_text("There is currently no support for right-to-left languages.");
            ui.label("See the 🔤 Font Book for more!");

            ui.monospace("There is also a monospace font.");
        });

        let tooltip_ui = |ui: &mut Ui| {
            ui.heading("The name of the tooltip");
            ui.horizontal(|ui| {
                ui.label("This tooltip was created with");
                ui.monospace(".on_hover_ui(...)");
            });
            let _ = ui.button("A button you can never press");
        };
        ui.label("Tooltips can be more than just simple text.")
            .on_hover_ui(tooltip_ui);

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("An angle:");
            ui.drag_angle(angle);
            ui.label(format!("≈ {:.3}τ", *angle / std::f32::consts::TAU))
                .on_hover_text("Each τ represents one turn (τ = 2π)");
        })
        .response
        .on_hover_text("The angle is stored in radians, but presented in degrees");

        ui.separator();

        ui.horizontal(|ui| {
            ui.hyperlink_to("Password:", super::password::url_to_file_source_code())
                .on_hover_text("See the example code for how to use egui to store UI state");
            ui.add(super::password::password(password));
        });
    }
}

// ----------------------------------------------------------------------------

#[derive(PartialEq)]
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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct Tree(String, SubTree);

impl Tree {
    pub fn demo() -> Self {
        Self(
            String::from("root"),
            SubTree(vec![
                SubTree(vec![SubTree::default(); 4]),
                SubTree(vec![SubTree(vec![SubTree::default(); 2]); 3]),
            ]),
        )
    }
    pub fn ui(&mut self, ui: &mut Ui) -> Action {
        self.1.ui(ui, 0, "root", &mut self.0)
    }
}

#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct SubTree(Vec<SubTree>);

impl SubTree {
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        depth: usize,
        name: &str,
        selected_name: &mut String,
    ) -> Action {
        let response = CollapsingHeader::new(name)
            .default_open(depth < 1)
            .selectable(true)
            .selected(selected_name.as_str() == name)
            .show(ui, |ui| self.children_ui(ui, name, depth, selected_name));
        if response.header_response.clicked() {
            *selected_name = name.to_string();
        }
        response.body_returned.unwrap_or(Action::Keep)
    }

    fn children_ui(
        &mut self,
        ui: &mut Ui,
        parent_name: &str,
        depth: usize,
        selected_name: &mut String,
    ) -> Action {
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
                if tree.ui(
                    ui,
                    depth + 1,
                    &format!("{}/{}", parent_name, i),
                    selected_name,
                ) == Action::Keep
                {
                    Some(tree)
                } else {
                    None
                }
            })
            .collect();

        if ui.button("+").clicked() {
            self.0.push(SubTree::default());
        }

        Action::Keep
    }
}

// ----------------------------------------------------------------------------

fn text_layout_ui(ui: &mut egui::Ui) {
    use egui::text::LayoutJob;

    let mut job = LayoutJob::default();

    let first_row_indentation = 10.0;

    let (default_color, strong_color) = if ui.visuals().dark_mode {
        (Color32::LIGHT_GRAY, Color32::WHITE)
    } else {
        (Color32::DARK_GRAY, Color32::BLACK)
    };

    job.append(
        "This is a demonstration of ",
        first_row_indentation,
        TextFormat {
            style: TextStyle::Body,
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "the egui text layout engine. ",
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: strong_color,
            ..Default::default()
        },
    );
    job.append(
        "It supports ",
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "different ",
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: Color32::from_rgb(110, 255, 110),
            ..Default::default()
        },
    );
    job.append(
        "colors, ",
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: Color32::from_rgb(128, 140, 255),
            ..Default::default()
        },
    );
    job.append(
        "backgrounds, ",
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: default_color,
            background: Color32::from_rgb(128, 32, 32),
            ..Default::default()
        },
    );
    job.append(
        "mixing ",
        0.0,
        TextFormat {
            style: TextStyle::Heading,
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "fonts, ",
        0.0,
        TextFormat {
            style: TextStyle::Monospace,
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "raised text, ",
        0.0,
        TextFormat {
            style: TextStyle::Small,
            color: default_color,
            valign: Align::TOP,
            ..Default::default()
        },
    );
    job.append(
        "with ",
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "underlining",
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: default_color,
            underline: Stroke::new(1.0, Color32::LIGHT_BLUE),
            ..Default::default()
        },
    );
    job.append(
        " and ",
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "strikethrough",
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: default_color,
            strikethrough: Stroke::new(2.0, Color32::RED.linear_multiply(0.5)),
            ..Default::default()
        },
    );
    job.append(
        ". Of course, ",
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "you can",
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: default_color,
            strikethrough: Stroke::new(1.0, strong_color),
            ..Default::default()
        },
    );
    job.append(
        " mix these!",
        0.0,
        TextFormat {
            style: TextStyle::Small,
            color: Color32::LIGHT_BLUE,
            background: Color32::from_rgb(128, 0, 0),
            underline: Stroke::new(1.0, strong_color),
            ..Default::default()
        },
    );

    job.wrap_width = ui.available_width();

    let galley = ui.fonts().layout_job(job);

    let (response, painter) = ui.allocate_painter(galley.size(), Sense::hover());
    painter.add(Shape::galley(response.rect.min, galley));

    ui.vertical_centered(|ui| {
        ui.add(crate::__egui_github_link_file_line!());
    });
}
