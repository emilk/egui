use std::sync::Arc;

use crate::{app, color::*, containers::*, demos::FractalClock, paint::*, widgets::*, *};

// ----------------------------------------------------------------------------

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DemoApp {
    previous_web_location_hash: String,
    open_windows: OpenWindows,
    demo_window: DemoWindow,
    fractal_clock: FractalClock,
    num_frames_painted: u64,
}

impl DemoApp {
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
        let DemoApp {
            open_windows,
            demo_window,
            fractal_clock,
            ..
        } = self;

        Window::new("Demo")
            .open(&mut open_windows.demo)
            .show(ctx, |ui| {
                demo_window.ui(ui);
            });

        Window::new("Settings")
            .open(&mut open_windows.settings)
            .show(ctx, |ui| {
                ctx.settings_ui(ui);
            });

        Window::new("Inspection")
            .open(&mut open_windows.inspection)
            .show(ctx, |ui| {
                ctx.inspection_ui(ui);
            });

        Window::new("Memory")
            .open(&mut open_windows.memory)
            .resizable(false)
            .show(ctx, |ui| {
                ctx.memory_ui(ui);
            });

        fractal_clock.window(ctx, &mut open_windows.fractal_clock);
    }

    fn backend_ui(&mut self, ui: &mut Ui, backend: &mut dyn app::Backend) {
        let is_web = backend.web_info().is_some();

        if is_web {
            ui.label("Egui is an immediate mode GUI written in Rust, compiled to WebAssembly, rendered with WebGL.");
            ui.label(
                "Everything you see is rendered as textured triangles. There is no DOM. There are no HTML elements."
            );
            ui.label("This is not JavaScript. This is Rust, running at 60 FPS. This is the web page, reinvented with game tech.");
            ui.label("This is also work in progress, and not ready for production... yet :)");
            ui.horizontal(|ui| {
                ui.label("Project home page:");
                ui.hyperlink("https://github.com/emilk/egui");
            });
        } else {
            ui.add(label!("Egui").text_style(TextStyle::Heading));
            if ui.add(Button::new("Quit")).clicked {
                backend.quit();
                return;
            }
        }

        ui.separator();

        ui.add(
            label!(
                "CPU usage: {:.2} ms / frame (excludes painting)",
                1e3 * backend.cpu_time()
            )
            .text_style(TextStyle::Monospace),
        );

        ui.separator();

        ui.horizontal(|ui| {
            let mut run_mode = backend.run_mode();
            ui.label("Run mode:");
            ui.radio_value("Continuous", &mut run_mode, app::RunMode::Continuous)
                .tooltip_text("Repaint everything each frame");
            ui.radio_value("Reactive", &mut run_mode, app::RunMode::Reactive)
                .tooltip_text("Repaint when there are animations or input (e.g. mouse movement)");
            backend.set_run_mode(run_mode);
        });

        if backend.run_mode() == app::RunMode::Continuous {
            ui.add(
                label!("Repainting the UI each frame. FPS: {:.1}", backend.fps())
                    .text_style(TextStyle::Monospace),
            );
        } else {
            ui.label("Only running UI code when there are animations or input");
        }

        self.num_frames_painted += 1;
        ui.label(format!("Total frames painted: {}", self.num_frames_painted));
    }
}

impl app::App for DemoApp {
    fn ui(&mut self, ui: &mut Ui, backend: &mut dyn app::Backend) {
        Window::new("Backend")
            .default_width(500.0)
            .show(ui.ctx(), |ui| {
                self.backend_ui(ui, backend);
            });

        let web_info = backend.web_info();
        let web_location_hash = web_info
            .as_ref()
            .map(|info| info.web_location_hash.as_str())
            .unwrap_or_default();
        self.ui(ui, web_location_hash);
    }

    #[cfg(feature = "serde_json")]
    fn on_exit(&mut self, storage: &mut dyn app::Storage) {
        app::set_value(storage, app::APP_KEY, self);
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct OpenWindows {
    demo: bool,
    fractal_clock: bool,

    // egui stuff:
    settings: bool,
    inspection: bool,
    memory: bool,
}

impl Default for OpenWindows {
    fn default() -> Self {
        Self {
            demo: true,
            ..OpenWindows::none()
        }
    }
}

impl OpenWindows {
    fn none() -> Self {
        Self {
            demo: false,
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
            ui.add(Checkbox::new(&mut windows.demo, "Demo"));
            ui.add(Checkbox::new(&mut windows.fractal_clock, "Fractal Clock"));
            ui.add(Separator::new());
            ui.add(Checkbox::new(&mut windows.settings, "Settings"));
            ui.add(Checkbox::new(&mut windows.inspection, "Inspection"));
            ui.add(Checkbox::new(&mut windows.memory, "Memory"));
        });
        menu::menu(ui, "About", |ui| {
            ui.add(label!("This is Egui"));
            ui.add(Hyperlink::new("https://github.com/emilk/egui").text("Egui home page"));
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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DemoWindow {
    num_columns: usize,

    widgets: Widgets,
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
            layout: Default::default(),
            tree: Tree::demo(),
            box_painting: Default::default(),
            painting: Default::default(),
        }
    }
}

impl DemoWindow {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.collapsing("About Egui", |ui| {
            ui.add(label!(
                "Egui is an experimental immediate mode GUI written in Rust."
            ));

            ui.horizontal(|ui| {
                ui.label("Project home page:");
                ui.hyperlink("https://github.com/emilk/egui");
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
                Resize::default().default_height(100.0).show(ui, |ui| {
                    ui.add(label!("This ui can be resized!"));
                    ui.add(label!("Just pull the handle on the bottom right"));
                });
            });

        ui.collapsing("Name clash demo", |ui| {
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

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
struct Widgets {
    button_enabled: bool,
    count: usize,
    radio: usize,
    slider_value: f32,
    single_line_text_input: String,
    multiline_text_input: String,
}

impl Default for Widgets {
    fn default() -> Self {
        Self {
            button_enabled: true,
            radio: 0,
            count: 0,
            slider_value: 3.4,
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

        ui.horizontal(|ui| {
            ui.radio_value("First", &mut self.radio, 0);
            ui.radio_value("Second", &mut self.radio, 1);
            ui.radio_value("Final", &mut self.radio, 2);
        });

        ui.add(Checkbox::new(&mut self.button_enabled, "Button enabled"));

        ui.inner_layout(Layout::horizontal(Align::Center), |ui| {
            if ui
                .add(Button::new("Click me").enabled(self.button_enabled))
                .tooltip_text("This will just increase a counter.")
                .clicked
            {
                self.count += 1;
            }
            ui.add(label!("The button has been clicked {} times", self.count));
        });

        ui.add(
            Slider::f32(&mut self.slider_value, -10.0..=10.0)
                .text("value")
                .precision(2),
        );
        if ui.add(Button::new("Double it")).clicked {
            self.slider_value *= 2.0;
        }
        ui.horizontal(|ui| {
            ui.label("drag this number:");
            ui.add(DragValue::f32(&mut self.slider_value).speed(0.01));
        });

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
        ui.painter().extend(cmds);
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
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
            .default_size([200.0, 200.0])
            .show(ui, |ui| self.content(ui));
    }

    fn content(&mut self, ui: &mut Ui) {
        let rect = ui.allocate_space(ui.available_finite().size());
        let interact = ui.interact(rect, ui.id(), Sense::drag());
        let rect = interact.rect;
        let clip_rect = ui.clip_rect().intersect(rect); // Make sure we don't paint out of bounds
        let painter = Painter::new(ui.ctx().clone(), ui.layer(), clip_rect);

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
                painter.add(PaintCmd::Path {
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

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
struct LayoutDemo {
    dir: Direction,
    align: Option<Align>, // None == jusitifed
    reversed: bool,
}

impl Default for LayoutDemo {
    fn default() -> Self {
        Self {
            dir: Direction::Vertical,
            align: Some(Align::Center),
            reversed: false,
        }
    }
}

impl LayoutDemo {
    pub fn ui(&mut self, ui: &mut Ui) {
        Resize::default()
            .default_size([200.0, 100.0])
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
