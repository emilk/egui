// #![allow(dead_code, unused_variables)] // should be commented out
use std::sync::Arc;

use crate::{color::*, containers::*, examples::FractalClock, widgets::*, *};

// ----------------------------------------------------------------------------

#[derive(Default)]
#[cfg_attr(feature = "with_serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "with_serde", serde(default))]
pub struct ExampleApp {
    previous_web_location_hash: String,

    open_windows: OpenWindows,
    // TODO: group the following together as ExampleWindows
    example_window: ExampleWindow,
    fractal_clock: FractalClock,
}

impl ExampleApp {
    /// `web_location_hash`: for web demo only. e.g. "#fragmet".
    pub fn ui(&mut self, ui: &mut Ui, web_location_hash: &str) {
        if self.previous_web_location_hash != web_location_hash {
            // #fragment end of URL:
            if web_location_hash == "#clock" {
                self.open_windows = OpenWindows {
                    fractal_clock: true,
                    ..OpenWindows::none()
                };
            }

            self.previous_web_location_hash = web_location_hash.to_owned();
        }

        show_menu_bar(ui, &mut self.open_windows);
        self.windows(ui.ctx());
    }

    pub fn windows(&mut self, ctx: &Arc<Context>) {
        // TODO: Make it even simpler to show a window

        // TODO: window manager for automatic positioning?

        let ExampleApp {
            open_windows,
            example_window,
            fractal_clock,
            ..
        } = self;

        Window::new("Examples")
            .open(&mut open_windows.examples)
            .default_pos([32.0, 100.0])
            .default_size([430.0, 600.0])
            .show(ctx, |ui| {
                example_window.ui(ui);
            });

        Window::new("Settings")
            .open(&mut open_windows.settings)
            .default_pos([500.0, 100.0])
            .default_size([350.0, 400.0])
            .show(ctx, |ui| {
                ctx.settings_ui(ui);
            });

        Window::new("Inspection")
            .open(&mut open_windows.inspection)
            .default_pos([500.0, 400.0])
            .default_size([400.0, 300.0])
            .show(ctx, |ui| {
                ctx.inspection_ui(ui);
            });

        Window::new("Memory")
            .open(&mut open_windows.memory)
            .default_pos([700.0, 350.0])
            .auto_sized()
            .show(ctx, |ui| {
                ctx.memory_ui(ui);
            });

        fractal_clock.window(ctx, &mut open_windows.fractal_clock);
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "with_serde", derive(serde::Deserialize, serde::Serialize))]
struct OpenWindows {
    // examples:
    examples: bool,
    fractal_clock: bool,

    // egui stuff:
    settings: bool,
    inspection: bool,
    memory: bool,
}

impl Default for OpenWindows {
    fn default() -> Self {
        Self {
            examples: true,
            ..OpenWindows::none()
        }
    }
}

impl OpenWindows {
    fn none() -> Self {
        Self {
            examples: false,
            fractal_clock: false,

            settings: false,
            inspection: false,
            memory: false,
        }
    }
}

fn show_menu_bar(ui: &mut Ui, windows: &mut OpenWindows) {
    menu::bar(ui, |ui| {
        menu::menu(ui, "File", |ui| {
            if ui.add(Button::new("Clear memory")).clicked {
                *ui.ctx().memory() = Default::default();
            }
        });
        menu::menu(ui, "Windows", |ui| {
            ui.add(Checkbox::new(&mut windows.examples, "Examples"));
            ui.add(Checkbox::new(&mut windows.fractal_clock, "Fractal Clock"));
            ui.add(Separator::new());
            ui.add(Checkbox::new(&mut windows.settings, "Settings"));
            ui.add(Checkbox::new(&mut windows.inspection, "Inspection"));
            ui.add(Checkbox::new(&mut windows.memory, "Memory"));
        });
        menu::menu(ui, "About", |ui| {
            ui.add(label!("This is Egui"));
            ui.add(Hyperlink::new("https://github.com/emilk/emigui/").text("Egui home page"));
        });

        if let Some(time) = ui.input().seconds_since_midnight {
            let time = format!(
                "{:02}:{:02}:{:02}.{:02}",
                (time.rem_euclid(24.0 * 60.0 * 60.0) / 3600.0).floor(),
                (time.rem_euclid(60.0 * 60.0) / 60.0).floor(),
                (time.rem_euclid(60.0)).floor(),
                (time.rem_euclid(1.0) * 100.0).floor()
            );
            ui.inner_layout(Layout::horizontal(Align::Max).reverse(), |ui| {
                if ui
                    .add(Button::new(time).text_style(TextStyle::Monospace))
                    .clicked
                {
                    windows.fractal_clock = !windows.fractal_clock;
                }
            });
        }
    });
}

// ----------------------------------------------------------------------------

/// Showcase some ui code
#[cfg_attr(feature = "with_serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ExampleWindow {
    num_columns: usize,

    widgets: Widgets,
    layout: LayoutExample,
    tree: Tree,
    box_painting: BoxPainting,
    painting: Painting,
}

impl Default for ExampleWindow {
    fn default() -> ExampleWindow {
        ExampleWindow {
            num_columns: 2,

            widgets: Default::default(),
            layout: Default::default(),
            tree: Tree::example(),
            box_painting: Default::default(),
            painting: Default::default(),
        }
    }
}

impl ExampleWindow {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.collapsing("About Egui", |ui| {
            ui.add(label!(
                "Egui is an experimental immediate mode GUI written in Rust."
            ));

            ui.horizontal(|ui| {
                ui.label("Project home page:");
                ui.hyperlink("https://github.com/emilk/emigui/");
            });
        });

        CollapsingHeader::new("Widgets")
            .default_open(true)
            .show(ui, |ui| {
                self.widgets.ui(ui);
            });

        CollapsingHeader::new("Layout")
            .default_open(false)
            .show(ui, |ui| self.layout.ui(ui));

        CollapsingHeader::new("Tree")
            .default_open(true)
            .show(ui, |ui| self.tree.ui(ui));

        ui.collapsing("Columns", |ui| {
            ui.add(Slider::usize(&mut self.num_columns, 1..=10).text("Columns"));
            ui.columns(self.num_columns, |cols| {
                for (i, col) in cols.iter_mut().enumerate() {
                    col.add(label!("Column {} out of {}", i + 1, self.num_columns));
                    if i + 1 == self.num_columns && col.add(Button::new("Delete this")).clicked {
                        self.num_columns -= 1;
                    }
                }
            });
        });

        ui.collapsing("Test box rendering", |ui| self.box_painting.ui(ui));

        CollapsingHeader::new("Scroll area")
            .default_open(false)
            .show(ui, |ui| {
                ScrollArea::default().show(ui, |ui| {
                    ui.label(LOREM_IPSUM);
                });
            });

        CollapsingHeader::new("Painting")
            .default_open(false)
            .show(ui, |ui| self.painting.ui(ui));

        CollapsingHeader::new("Resize")
            .default_open(false)
            .show(ui, |ui| {
                Resize::default()
                    .default_height(200.0)
                    // .as_wide_as_possible()
                    .auto_shrink_height(false)
                    .show(ui, |ui| {
                        ui.add(label!("This ui can be resized!"));
                        ui.add(label!("Just pull the handle on the bottom right"));
                    });
            });

        ui.collapsing("Name clash example", |ui| {
            ui.label("\
                Widgets that store state require unique identifiers so we can track their state between frames. \
                Identifiers are normally derived from the titles of the widget.");

            ui.label("\
                For instance, collapsable headers needs to store wether or not they are open. \
                If you fail to give them unique names then clicking one will open both. \
                To help you debug this, an error message is printed on screen:");

            ui.collapsing("Collapsing header", |ui| {
                ui.label("Contents of first folddable ui");
            });
            ui.collapsing("Collapsing header", |ui| {
                ui.label("Contents of second folddable ui");
            });

            ui.label("\
                Most widgets don't need unique names, but are tracked \
                based on their position on screen. For instance, buttons:");
            ui.add(Button::new("Button"));
            ui.add(Button::new("Button"));
        });
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "with_serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "with_serde", serde(default))]
struct Widgets {
    checked: bool,
    count: usize,
    radio: usize,
    slider_value: usize,
    single_line_text_input: String,
    multiline_text_input: String,
}

impl Default for Widgets {
    fn default() -> Self {
        Self {
            checked: true,
            radio: 0,
            count: 0,
            slider_value: 100,
            single_line_text_input: "Hello World!".to_owned(),
            multiline_text_input: "Text can both be so wide that it needs a linebreak, but you can also add manual linebreak by pressing enter, creating new paragraphs.\nThis is the start of the next paragraph.\n\nClick me to edit me!".to_owned(),
        }
    }
}

impl Widgets {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
                ui.add(label!("Text can have").text_color(srgba(110, 255, 110, 255)));
                ui.add(label!("color").text_color(srgba(128, 140, 255, 255)));
                ui.add(label!("and tooltips (hover me)")).tooltip_text(
                    "This is a multiline tooltip that demonstrates that you can easily add tooltips to any element.\nThis is the second line.\nThis is the third.",
                );
            });

        ui.add(Checkbox::new(&mut self.checked, "checkbox"));

        ui.horizontal(|ui| {
            if ui.add(radio(self.radio == 0, "First")).clicked {
                self.radio = 0;
            }
            if ui.add(radio(self.radio == 1, "Second")).clicked {
                self.radio = 1;
            }
            if ui.add(radio(self.radio == 2, "Final")).clicked {
                self.radio = 2;
            }
        });

        ui.inner_layout(Layout::horizontal(Align::Center), |ui| {
            if ui
                .add(Button::new("Click me"))
                .tooltip_text("This will just increase a counter.")
                .clicked
            {
                self.count += 1;
            }
            ui.add(label!("The button has been clicked {} times", self.count));
        });

        ui.add(Slider::usize(&mut self.slider_value, 1..=1000).text("value"));
        if ui.add(Button::new("Double it")).clicked {
            self.slider_value *= 2;
        }

        ui.horizontal(|ui| {
            ui.add(label!("Single line text input:"));
            ui.add(
                TextEdit::new(&mut self.single_line_text_input)
                    .multiline(false)
                    .id("single line"),
            );
        }); // TODO: .tooltip_text("Enter text to edit me")

        ui.add(label!("Multiline text input:"));
        ui.add(TextEdit::new(&mut self.multiline_text_input).id("multiline"));
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "with_serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "with_serde", serde(default))]
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
            cmds.push(PaintCmd::Rect {
                corner_radius: self.corner_radius,
                fill: Some(gray(136, 255)),
                rect: Rect::from_min_size(
                    pos2(10.0 + pos.x + (i as f32) * (self.size.x * 1.1), pos.y),
                    self.size,
                ),
                outline: Some(LineStyle::new(self.stroke_width, gray(255, 255))),
            });
        }
        ui.add_paint_cmds(cmds);
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
#[cfg_attr(feature = "with_serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "with_serde", serde(default))]
struct Painting {
    lines: Vec<Vec<Vec2>>,
}

impl Painting {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("Draw with your mouse to paint");
        if ui.add(Button::new("Clear")).clicked {
            self.lines.clear();
        }

        Resize::default()
            .default_height(200.0)
            .show(ui, |ui| self.content(ui));
    }

    fn content(&mut self, ui: &mut Ui) {
        let rect = ui.allocate_space(ui.available_finite().size());
        let interact = ui.interact(rect, ui.id(), Sense::drag());
        let rect = interact.rect;
        ui.set_clip_rect(ui.clip_rect().intersect(rect)); // Make sure we don't paint out of bounds

        if self.lines.is_empty() {
            self.lines.push(vec![]);
        }

        let current_line = self.lines.last_mut().unwrap();

        if interact.active {
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
                ui.add_paint_cmd(PaintCmd::Path {
                    path: Path::from_open_points(&points),
                    closed: false,
                    outline: Some(LineStyle::new(2.0, LIGHT_GRAY)),
                    fill: None,
                });
            }
        }
    }
}

// ----------------------------------------------------------------------------

use crate::layout::*;

#[cfg_attr(feature = "with_serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "with_serde", serde(default))]
struct LayoutExample {
    dir: Direction,
    align: Option<Align>, // None == jusitifed
    reversed: bool,
}

impl Default for LayoutExample {
    fn default() -> Self {
        Self {
            dir: Direction::Vertical,
            align: Some(Align::Center),
            reversed: false,
        }
    }
}

impl LayoutExample {
    pub fn ui(&mut self, ui: &mut Ui) {
        Resize::default()
            .default_size([200.0, 200.0])
            .show(ui, |ui| self.content_ui(ui));
    }

    pub fn content_ui(&mut self, ui: &mut Ui) {
        let layout = Layout::from_dir_align(self.dir, self.align);
        if self.reversed {
            ui.set_layout(layout.reverse());
        } else {
            ui.set_layout(layout);
        }

        // ui.add(label!("Available space: {:?}", ui.available().size()));
        if ui.add(Button::new("Reset")).clicked {
            *self = Default::default();
        }
        ui.add(Separator::new());
        ui.add(label!("Direction:"));

        // TODO: enum iter

        for &dir in &[Direction::Horizontal, Direction::Vertical] {
            if ui
                .add(RadioButton::new(self.dir == dir, format!("{:?}", dir)))
                .clicked
            {
                self.dir = dir;
            }
        }

        ui.add(Checkbox::new(&mut self.reversed, "Reversed"));

        ui.add(Separator::new());

        ui.add(label!("Align:"));

        for &align in &[Align::Min, Align::Center, Align::Max] {
            if ui
                .add(RadioButton::new(
                    self.align == Some(align),
                    format!("{:?}", align),
                ))
                .clicked
            {
                self.align = Some(align);
            }
        }
        if ui
            .add(RadioButton::new(self.align == None, "Justified"))
            .tooltip_text("Try to fill full width/heigth (e.g. buttons)")
            .clicked
        {
            self.align = None;
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
#[cfg_attr(feature = "with_serde", derive(serde::Deserialize, serde::Serialize))]
struct Tree(Vec<Tree>);

impl Tree {
    pub fn example() -> Self {
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

// ----------------------------------------------------------------------------

const LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

Curabitur pretium tincidunt lacus. Nulla gravida orci a odio. Nullam varius, turpis et commodo pharetra, est eros bibendum elit, nec luctus magna felis sollicitudin mauris. Integer in mauris eu nibh euismod gravida. Duis ac tellus et risus vulputate vehicula. Donec lobortis risus a elit. Etiam tempor. Ut ullamcorper, ligula eu tempor congue, eros est euismod turpis, id tincidunt sapien risus a quam. Maecenas fermentum consequat mi. Donec fermentum. Pellentesque malesuada nulla a mi. Duis sapien sem, aliquet nec, commodo eget, consequat quis, neque. Aliquam faucibus, elit ut dictum aliquet, felis nisl adipiscing sapien, sed malesuada diam lacus eget erat. Cras mollis scelerisque nunc. Nullam arcu. Aliquam consequat. Curabitur augue lorem, dapibus quis, laoreet et, pretium ac, nisi. Aenean magna nisl, mollis quis, molestie eu, feugiat in, orci. In hac habitasse platea dictumst.";
