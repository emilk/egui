use std::sync::Arc;

use crate::{
    app,
    demos::{self, Demo},
    Context, Id, Resize, ScrollArea, Ui, Window,
};

// ----------------------------------------------------------------------------

/// Link to show a specific part of the demo app.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DemoLink {
    Clock,
}

/// Special input to the demo-app.
#[derive(Default)]
pub struct DemoEnvironment {
    /// Local time. Used for the clock in the demo app.
    pub seconds_since_midnight: Option<f64>,

    /// Set to `Some` to open a specific part of the demo app.
    pub link: Option<DemoLink>,
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
struct Demos {
    /// open, view
    #[cfg_attr(feature = "serde", serde(skip))] // TODO: serialize the `open` state.
    demos: Vec<(bool, Box<dyn Demo>)>,
}
impl Default for Demos {
    fn default() -> Self {
        Self {
            demos: vec![
                (false, Box::new(crate::demos::DancingStrings::default())),
                (false, Box::new(crate::demos::DragAndDropDemo::default())),
                (false, Box::new(crate::demos::Tests::default())),
            ],
        }
    }
}
impl Demos {
    pub fn checkboxes(&mut self, ui: &mut Ui) {
        for (ref mut open, demo) in &mut self.demos {
            ui.checkbox(open, demo.name());
        }
    }

    pub fn show(&mut self, ctx: &Arc<Context>) {
        for (ref mut open, demo) in &mut self.demos {
            demo.show(ctx, open);
        }
    }
}

// ----------------------------------------------------------------------------

/// A menu bar in which you can select different demo windows to show.
#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DemoWindows {
    open_windows: OpenWindows,

    demo_window: demos::DemoWindow,

    #[cfg_attr(feature = "serde", serde(skip))]
    color_test: demos::ColorTest,

    fractal_clock: demos::FractalClock,

    /// open, title, view
    demos: Demos,

    #[cfg_attr(feature = "serde", serde(skip))]
    previous_link: Option<DemoLink>,
}

impl DemoWindows {
    /// Show the app ui (menu bar and windows).
    pub fn ui(
        &mut self,
        ctx: &Arc<Context>,
        env: &DemoEnvironment,
        tex_allocator: &mut Option<&mut dyn app::TextureAllocator>,
    ) {
        if self.previous_link != env.link {
            match env.link {
                None => {}
                Some(DemoLink::Clock) => {
                    self.open_windows = OpenWindows {
                        fractal_clock: true,
                        ..OpenWindows::none()
                    };
                }
            }
            self.previous_link = env.link;
        }

        crate::SidePanel::left(Id::new("side_panel"), 200.0).show(ctx, |ui| {
            ui.heading("Egui Demo");
            crate::demos::warn_if_debug_build(ui);
            ui.label("Egui is an immediate mode GUI library written in Rust.");
            ui.add(crate::Hyperlink::new("https://github.com/emilk/egui").text("Egui home page"));

            ui.separator();
            ui.label(
                "This is an example of a panel. Windows are constrained to the area that remain.",
            );
            if ui.button("Organize windows").clicked {
                ui.ctx().memory().reset_areas();
            }
            ui.separator();

            ui.heading("Windows:");
            ui.indent("windows", |ui| {
                self.open_windows.checkboxes(ui);
                self.demos.checkboxes(ui);
            });
        });

        crate::TopPanel::top(Id::new("menu_bar")).show(ctx, |ui| {
            show_menu_bar(ui, &mut self.open_windows, env.seconds_since_midnight);
        });

        self.windows(ctx, env, tex_allocator);
    }

    /// Show the open windows.
    fn windows(
        &mut self,
        ctx: &Arc<Context>,
        env: &DemoEnvironment,
        tex_allocator: &mut Option<&mut dyn app::TextureAllocator>,
    ) {
        let Self {
            open_windows,
            demo_window,
            color_test,
            fractal_clock,
            demos,
            ..
        } = self;

        Window::new("Demo")
            .open(&mut open_windows.demo)
            .scroll(true)
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
            .scroll(true)
            .show(ctx, |ui| {
                ctx.inspection_ui(ui);
            });

        Window::new("Memory")
            .open(&mut open_windows.memory)
            .resizable(false)
            .show(ctx, |ui| {
                ctx.memory_ui(ui);
            });

        Window::new("Color Test")
            .default_size([800.0, 1024.0])
            .scroll(true)
            .open(&mut open_windows.color_test)
            .show(ctx, |ui| {
                color_test.ui(ui, tex_allocator);
            });

        demos.show(ctx);

        fractal_clock.window(
            ctx,
            &mut open_windows.fractal_clock,
            env.seconds_since_midnight,
        );

        self.resize_windows(ctx);
    }

    fn resize_windows(&mut self, ctx: &Arc<Context>) {
        let open = &mut self.open_windows.resize;

        Window::new("resizable")
            .open(open)
            .scroll(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label("scroll:    NO");
                ui.label("resizable: YES");
                ui.label(demos::LOREM_IPSUM);
            });

        Window::new("resizable + embedded scroll")
            .open(open)
            .scroll(false)
            .resizable(true)
            .default_height(300.0)
            .show(ctx, |ui| {
                ui.label("scroll:    NO");
                ui.label("resizable: YES");
                ui.heading("We have a sub-region with scroll bar:");
                ScrollArea::auto_sized().show(ui, |ui| {
                    ui.label(demos::LOREM_IPSUM_LONG);
                    ui.label(demos::LOREM_IPSUM_LONG);
                });
                // ui.heading("Some additional text here, that should also be visible"); // this works, but messes with the resizing a bit
            });

        Window::new("resizable + scroll")
            .open(open)
            .scroll(true)
            .resizable(true)
            .default_height(300.0)
            .show(ctx, |ui| {
                ui.label("scroll:    YES");
                ui.label("resizable: YES");
                ui.label(demos::LOREM_IPSUM_LONG);
            });

        Window::new("auto_sized")
            .open(open)
            .auto_sized()
            .show(ctx, |ui| {
                ui.label("This window will auto-size based on its contents.");
                ui.heading("Resize this area:");
                Resize::default().show(ui, |ui| {
                    ui.label(demos::LOREM_IPSUM);
                });
                ui.heading("Resize the above area!");
            });
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
    resize: bool,

    // debug stuff:
    color_test: bool,
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
            resize: false,

            color_test: false,
        }
    }

    fn checkboxes(&mut self, ui: &mut Ui) {
        let Self {
            demo,
            fractal_clock,
            settings,
            inspection,
            memory,
            resize,
            color_test,
        } = self;
        ui.label("Egui:");
        ui.checkbox(settings, "Settings");
        ui.checkbox(inspection, "Inspection");
        ui.checkbox(memory, "Memory");
        ui.separator();
        ui.checkbox(demo, "Demo");
        ui.separator();
        ui.checkbox(resize, "Resize examples");
        ui.checkbox(color_test, "Color test")
            .on_hover_text("For testing the integrations painter");
        ui.separator();
        ui.label("Misc:");
        ui.checkbox(fractal_clock, "Fractal Clock");
    }
}

fn show_menu_bar(ui: &mut Ui, windows: &mut OpenWindows, seconds_since_midnight: Option<f64>) {
    use crate::*;

    menu::bar(ui, |ui| {
        menu::menu(ui, "File", |ui| {
            if ui.button("Organize windows").clicked {
                ui.ctx().memory().reset_areas();
            }
            if ui
                .button("Clear entire Egui memory")
                .on_hover_text("Forget scroll, collapsing headers etc")
                .clicked
            {
                *ui.ctx().memory() = Default::default();
            }
        });
        menu::menu(ui, "Windows", |ui| windows.checkboxes(ui));
        menu::menu(ui, "About", |ui| {
            ui.label("This is Egui");
            ui.add(Hyperlink::new("https://github.com/emilk/egui").text("Egui home page"));
        });

        if let Some(time) = seconds_since_midnight {
            let time = format!(
                "{:02}:{:02}:{:02}.{:02}",
                (time % (24.0 * 60.0 * 60.0) / 3600.0).floor(),
                (time % (60.0 * 60.0) / 60.0).floor(),
                (time % 60.0).floor(),
                (time % 1.0 * 100.0).floor()
            );

            ui.with_layout(Layout::horizontal(Align::Center).reverse(), |ui| {
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
