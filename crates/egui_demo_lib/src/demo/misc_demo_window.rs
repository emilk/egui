use std::sync::Arc;

use super::{Demo, View};

use egui::{
    Align, Align2, Checkbox, CollapsingHeader, Color32, ComboBox, FontId, Resize, RichText, Sense,
    Slider, Stroke, TextFormat, TextStyle, Ui, Vec2, Window, vec2,
};

/// Showcase some ui code
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct MiscDemoWindow {
    num_columns: usize,

    widgets: Widgets,
    colors: ColorWidgets,
    custom_collapsing_header: CustomCollapsingHeader,
    tree: Tree,
    box_painting: BoxPainting,
    text_rotation: TextRotation,

    dummy_bool: bool,
    dummy_usize: usize,
    checklist: [bool; 3],
}

impl Default for MiscDemoWindow {
    fn default() -> Self {
        Self {
            num_columns: 2,

            widgets: Default::default(),
            colors: Default::default(),
            custom_collapsing_header: Default::default(),
            tree: Tree::demo(),
            box_painting: Default::default(),
            text_rotation: Default::default(),

            dummy_bool: false,
            dummy_usize: 0,
            checklist: std::array::from_fn(|i| i == 0),
        }
    }
}

impl Demo for MiscDemoWindow {
    fn name(&self) -> &'static str {
        "✨ Misc Demos"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        Window::new(self.name())
            .open(open)
            .vscroll(true)
            .hscroll(true)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| self.ui(ui));
    }
}

impl View for MiscDemoWindow {
    fn ui(&mut self, ui: &mut Ui) {
        ui.set_min_width(250.0);

        CollapsingHeader::new("Label")
            .default_open(true)
            .show(ui, |ui| {
                label_ui(ui);
            });

        CollapsingHeader::new("Misc widgets")
            .default_open(false)
            .show(ui, |ui| {
                self.widgets.ui(ui);
            });

        CollapsingHeader::new("Text layout")
            .default_open(false)
            .show(ui, |ui| {
                text_layout_demo(ui);
                ui.vertical_centered(|ui| {
                    ui.add(crate::egui_github_link_file_line!());
                });
            });

        CollapsingHeader::new("Text rotation")
            .default_open(false)
            .show(ui, |ui| self.text_rotation.ui(ui));

        CollapsingHeader::new("Colors")
            .default_open(false)
            .show(ui, |ui| {
                self.colors.ui(ui);
            });

        CollapsingHeader::new("Custom Collapsing Header")
            .default_open(false)
            .show(ui, |ui| self.custom_collapsing_header.ui(ui));

        CollapsingHeader::new("Tree")
            .default_open(false)
            .show(ui, |ui| self.tree.ui(ui));

        CollapsingHeader::new("Checkboxes")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("Checkboxes with empty labels take up very little space:");
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.horizontal_wrapped(|ui| {
                    for _ in 0..64 {
                        ui.checkbox(&mut self.dummy_bool, "");
                    }
                });
                ui.checkbox(&mut self.dummy_bool, "checkbox");

                ui.label("Radiobuttons are similar:");
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.horizontal_wrapped(|ui| {
                    for i in 0..64 {
                        ui.radio_value(&mut self.dummy_usize, i, "");
                    }
                });
                ui.radio_value(&mut self.dummy_usize, 64, "radio_value");
                ui.label("Checkboxes can be in an indeterminate state:");
                let mut all_checked = self.checklist.iter().all(|item| *item);
                let any_checked = self.checklist.iter().any(|item| *item);
                let indeterminate = any_checked && !all_checked;
                if ui
                    .add(
                        Checkbox::new(&mut all_checked, "Check/uncheck all")
                            .indeterminate(indeterminate),
                    )
                    .changed()
                {
                    for check in &mut self.checklist {
                        *check = all_checked;
                    }
                }
                for (i, checked) in self.checklist.iter_mut().enumerate() {
                    ui.checkbox(checked, format!("Item {}", i + 1));
                }
            });

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

        CollapsingHeader::new("Ui Stack")
            .default_open(false)
            .show(ui, ui_stack_demo);

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

        CollapsingHeader::new("Many circles of different sizes")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for i in 0..100 {
                        let r = i as f32 * 0.5;
                        let size = Vec2::splat(2.0 * r + 5.0);
                        let (rect, _response) = ui.allocate_at_least(size, Sense::hover());
                        ui.painter()
                            .circle_filled(rect.center(), r, ui.visuals().text_color());
                    }
                });
            });
    }
}

// ----------------------------------------------------------------------------

fn label_ui(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add(crate::egui_github_link_file_line!());
    });

    ui.horizontal_wrapped(|ui| {
            // Trick so we don't have to add spaces in the text below:
            let width = ui.fonts_mut(|f|f.glyph_width(&TextStyle::Body.resolve(ui.style()), ' '));
            ui.spacing_mut().item_spacing.x = width;

            ui.label(RichText::new("Text can have").color(Color32::from_rgb(110, 255, 110)));
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

    ui.add(
        egui::Label::new(
            "Labels containing long text can be set to elide the text that doesn't fit on a single line using `Label::truncate`. When hovered, the label will show the full text.",
        )
        .truncate(),
    );
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
            ui.add(crate::egui_github_link_file_line!());
        });

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
        Self {
            srgba_unmul: [0, 255, 183, 127],
            srgba_premul: [0, 187, 140, 127],
            rgba_unmul: [0.0, 1.0, 0.5, 0.5],
            rgba_premul: [0.0, 0.5, 0.25, 0.5],
        }
    }
}

impl ColorWidgets {
    fn ui(&mut self, ui: &mut Ui) {
        egui::reset_button(ui, self, "Reset");

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
                    ui.visuals().text_color().gamma_multiply(0.5),
                    Stroke::new(self.stroke_width, Color32::WHITE),
                    egui::StrokeKind::Inside,
                );
            }
        });
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct CustomCollapsingHeader {
    selected: bool,
    radio_value: bool,
}

impl Default for CustomCollapsingHeader {
    fn default() -> Self {
        Self {
            selected: true,
            radio_value: false,
        }
    }
}

impl CustomCollapsingHeader {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Example of a collapsing header with custom header:");

        let id = ui.make_persistent_id("my_collapsing_header");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                ui.toggle_value(&mut self.selected, "Click to select/unselect");
                ui.radio_value(&mut self.radio_value, false, "");
                ui.radio_value(&mut self.radio_value, true, "");
            })
            .body(|ui| {
                ui.label("The body is always custom");
            });

        CollapsingHeader::new("Normal collapsing header for comparison").show(ui, |ui| {
            ui.label("Nothing exciting here");
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
struct Tree(Vec<Self>);

impl Tree {
    pub fn demo() -> Self {
        Self(vec![
            Self(vec![Self::default(); 4]),
            Self(vec![Self(vec![Self::default(); 2]); 3]),
        ])
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Action {
        self.ui_impl(ui, 0, "root")
    }
}

impl Tree {
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
                .button(RichText::new("delete").color(ui.visuals().warn_fg_color))
                .clicked()
        {
            return Action::Delete;
        }

        self.0 = std::mem::take(self)
            .0
            .into_iter()
            .enumerate()
            .filter_map(|(i, mut tree)| {
                if tree.ui_impl(ui, depth + 1, &format!("child #{i}")) == Action::Keep {
                    Some(tree)
                } else {
                    None
                }
            })
            .collect();

        if ui.button("+").clicked() {
            self.0.push(Self::default());
        }

        Action::Keep
    }
}

// ----------------------------------------------------------------------------

fn ui_stack_demo(ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.label("The");
        ui.code("egui::Ui");
        ui.label("core type is typically deeply nested in");
        ui.code("egui");
        ui.label(
            "applications. To provide context to nested code, it maintains a stack \
                        with various information.\n\nThis is how the stack looks like here:",
        );
    });
    let stack = Arc::clone(ui.stack());
    egui::Frame::new()
        .inner_margin(ui.spacing().menu_margin)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .show(ui, |ui| {
            egui_extras::TableBuilder::new(ui)
                .column(egui_extras::Column::auto())
                .column(egui_extras::Column::auto())
                .header(18.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("id");
                    });
                    header.col(|ui| {
                        ui.strong("kind");
                    });
                })
                .body(|mut body| {
                    for node in stack.iter() {
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                let response = ui.label(format!("{:?}", node.id));

                                if response.hovered() {
                                    ui.debug_painter().debug_rect(
                                        node.max_rect,
                                        Color32::GREEN,
                                        "max_rect",
                                    );
                                    ui.debug_painter().circle_filled(
                                        node.min_rect.min,
                                        2.0,
                                        Color32::RED,
                                    );
                                }
                            });

                            row.col(|ui| {
                                ui.label(if let Some(kind) = node.kind() {
                                    format!("{kind:?}")
                                } else {
                                    "-".to_owned()
                                });
                            });
                        });
                    }
                });
        });

    ui.small("Hover on UI's ids to display their origin and max rect.");
}

// ----------------------------------------------------------------------------

fn text_layout_demo(ui: &mut Ui) {
    use egui::text::LayoutJob;

    let mut job = LayoutJob::default();

    let first_row_indentation = 10.0;

    let (default_color, strong_color) = if ui.visuals().dark_mode {
        (Color32::LIGHT_GRAY, Color32::WHITE)
    } else {
        (Color32::DARK_GRAY, Color32::BLACK)
    };

    job.append(
        "This",
        first_row_indentation,
        TextFormat {
            color: default_color,
            font_id: FontId::proportional(20.0),
            ..Default::default()
        },
    );
    job.append(
        " is a demonstration of ",
        0.0,
        TextFormat {
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "the egui text layout engine. ",
        0.0,
        TextFormat {
            color: strong_color,
            ..Default::default()
        },
    );
    job.append(
        "It supports ",
        0.0,
        TextFormat {
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "different ",
        0.0,
        TextFormat {
            color: Color32::from_rgb(110, 255, 110),
            ..Default::default()
        },
    );
    job.append(
        "colors, ",
        0.0,
        TextFormat {
            color: Color32::from_rgb(128, 140, 255),
            ..Default::default()
        },
    );
    job.append(
        "backgrounds, ",
        0.0,
        TextFormat {
            color: default_color,
            background: Color32::from_rgb(128, 32, 32),
            ..Default::default()
        },
    );
    job.append(
        "mixing ",
        0.0,
        TextFormat {
            font_id: FontId::proportional(20.0),
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "fonts, ",
        0.0,
        TextFormat {
            font_id: FontId::monospace(12.0),
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "raised text, ",
        0.0,
        TextFormat {
            font_id: FontId::proportional(7.0),
            color: default_color,
            valign: Align::TOP,
            ..Default::default()
        },
    );
    job.append(
        "with ",
        0.0,
        TextFormat {
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "underlining",
        0.0,
        TextFormat {
            color: default_color,
            underline: Stroke::new(1.0, Color32::LIGHT_BLUE),
            ..Default::default()
        },
    );
    job.append(
        " and ",
        0.0,
        TextFormat {
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "strikethrough",
        0.0,
        TextFormat {
            color: default_color,
            strikethrough: Stroke::new(2.0, Color32::RED.linear_multiply(0.5)),
            ..Default::default()
        },
    );
    job.append(
        ". Of course, ",
        0.0,
        TextFormat {
            color: default_color,
            ..Default::default()
        },
    );
    job.append(
        "you can",
        0.0,
        TextFormat {
            color: default_color,
            strikethrough: Stroke::new(1.0, strong_color),
            ..Default::default()
        },
    );
    job.append(
        " mix these!",
        0.0,
        TextFormat {
            font_id: FontId::proportional(7.0),
            color: Color32::LIGHT_BLUE,
            background: Color32::from_rgb(128, 0, 0),
            underline: Stroke::new(1.0, strong_color),
            ..Default::default()
        },
    );

    ui.label(job);
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
struct TextRotation {
    size: Vec2,
    angle: f32,
    align: egui::Align2,
}

impl Default for TextRotation {
    fn default() -> Self {
        Self {
            size: vec2(200.0, 200.0),
            angle: 0.0,
            align: egui::Align2::LEFT_TOP,
        }
    }
}

impl TextRotation {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.add(Slider::new(&mut self.angle, 0.0..=2.0 * std::f32::consts::PI).text("angle"));

        let default_color = if ui.visuals().dark_mode {
            Color32::LIGHT_GRAY
        } else {
            Color32::DARK_GRAY
        };

        let aligns = [
            (Align2::LEFT_TOP, "LEFT_TOP"),
            (Align2::LEFT_CENTER, "LEFT_CENTER"),
            (Align2::LEFT_BOTTOM, "LEFT_BOTTOM"),
            (Align2::CENTER_TOP, "CENTER_TOP"),
            (Align2::CENTER_CENTER, "CENTER_CENTER"),
            (Align2::CENTER_BOTTOM, "CENTER_BOTTOM"),
            (Align2::RIGHT_TOP, "RIGHT_TOP"),
            (Align2::RIGHT_CENTER, "RIGHT_CENTER"),
            (Align2::RIGHT_BOTTOM, "RIGHT_BOTTOM"),
        ];

        ComboBox::new("anchor", "Anchor")
            .selected_text(aligns.iter().find(|(a, _)| *a == self.align).unwrap().1)
            .show_ui(ui, |ui| {
                for (align2, name) in &aligns {
                    ui.selectable_value(&mut self.align, *align2, *name);
                }
            });

        ui.horizontal_wrapped(|ui| {
            let (response, painter) = ui.allocate_painter(self.size, Sense::empty());
            let rect = response.rect;

            let start_pos = self.size / 2.0;

            let s = ui.ctx().fonts_mut(|f| {
                let mut t = egui::Shape::text(
                    f,
                    rect.min + start_pos,
                    egui::Align2::LEFT_TOP,
                    "sample_text",
                    egui::FontId::new(12.0, egui::FontFamily::Proportional),
                    default_color,
                );

                if let egui::epaint::Shape::Text(ts) = &mut t {
                    let new = ts.clone().with_angle_and_anchor(self.angle, self.align);
                    *ts = new;
                }

                t
            });

            if let egui::epaint::Shape::Text(ts) = &s {
                let align_pt =
                    rect.min + start_pos + self.align.pos_in_rect(&ts.galley.rect).to_vec2();
                painter.circle(align_pt, 2.0, Color32::RED, (0.0, Color32::RED));
            }

            painter.rect(
                rect,
                0.0,
                default_color.gamma_multiply(0.3),
                (0.0, Color32::BLACK),
                egui::StrokeKind::Middle,
            );
            painter.add(s);
        });
    }
}
