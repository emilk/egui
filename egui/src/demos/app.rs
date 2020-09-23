use std::sync::Arc;

use crate::{app, color::*, containers::*, demos::*, paint::*, widgets::*, *};

// ----------------------------------------------------------------------------

/// How often we repaint the demo app by default
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RunMode {
    /// This is the default for the demo.
    ///
    /// If this is selected, Egui is only updated if are input events
    /// (like mouse movements) or there are some animations in the GUI.
    ///
    /// Reactive mode saves CPU.
    ///
    /// The downside is that the UI can become out-of-date if something it is supposed to monitor changes.
    /// For instance, a GUI for a thermostat need to repaint each time the temperature changes.
    /// To ensure the UI is up to date you need to call `egui::Context::request_repaint()` each
    /// time such an event happens. You can also chose to call `request_repaint()` once every second
    /// or after every single frame - this is called `Continuous` mode,
    /// and for games and interactive tools that need repainting every frame anyway, this should be the default.
    Reactive,

    /// This will call `egui::Context::request_repaint()` at the end of each frame
    /// to request the backend to repaint as soon as possible.
    ///
    /// On most platforms this will mean that Egui will run at the display refresh rate of e.g. 60 Hz.
    ///
    /// For this demo it is not any reason to do so except to
    /// demonstrate how quickly Egui runs.
    ///
    /// For games or other interactive apps, this is probably what you want to do.
    /// It will guarantee that Egui is always up-to-date.
    Continuous,
}

/// Default for demo is Reactive since
/// 1) We want to use minimal CPU
/// 2) There are no external events that could invalidate the UI
///    so there are no events to miss.
impl Default for RunMode {
    fn default() -> Self {
        RunMode::Reactive
    }
}

// ----------------------------------------------------------------------------

/// Special input to the demo-app.
#[derive(Default)]
pub struct DemoEnvironment {
    /// For web demo only. e.g. "#fragment".
    /// Used to link to specific part of the demo app.
    pub web_location_hash: String,

    /// Local time. Used for the clock in the demo app.
    pub seconds_since_midnight: Option<f64>,
}

/// Demonstrates how to make an app using Egui.
///
/// Implements `egui::app::App` so it can be used with
/// [`egui_glium`](https://crates.io/crates/egui_glium) and [`egui_web`](https://crates.io/crates/egui_web).
#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DemoApp {
    #[cfg_attr(feature = "serde", serde(skip))] // go back to `Reactive` mode each time we start
    run_mode: RunMode,
    previous_web_location_hash: String,
    open_windows: OpenWindows,
    demo_window: DemoWindow,
    fractal_clock: FractalClock,
    num_frames_painted: u64,
    #[cfg_attr(feature = "serde", serde(skip))]
    color_test: ColorTest,
    show_color_test: bool,
}

impl DemoApp {
    /// Show the app ui (menu bar and windows).
    pub fn ui(&mut self, ui: &mut Ui, env: &DemoEnvironment) {
        if self.previous_web_location_hash != env.web_location_hash {
            // #fragment end of URL:
            if env.web_location_hash == "#clock" {
                self.open_windows = OpenWindows {
                    fractal_clock: true,
                    ..OpenWindows::none()
                };
            }

            self.previous_web_location_hash = env.web_location_hash.clone();
        }

        show_menu_bar(ui, &mut self.open_windows, env);
        self.windows(ui.ctx(), env);
    }

    /// Show the open windows.
    pub fn windows(&mut self, ctx: &Arc<Context>, env: &DemoEnvironment) {
        let DemoApp {
            open_windows,
            demo_window,
            fractal_clock,
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
                ui.label(LOREM_IPSUM);
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
                    ui.label(LOREM_IPSUM_LONG);
                    ui.label(LOREM_IPSUM_LONG);
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
                ui.label(LOREM_IPSUM_LONG);
            });

        Window::new("auto_sized")
            .open(open)
            .auto_sized()
            .show(ctx, |ui| {
                ui.label("This window will auto-size based on its contents.");
                ui.heading("Resize this area:");
                Resize::default().show(ui, |ui| {
                    ui.label(LOREM_IPSUM);
                });
                ui.heading("Resize the above area!");
            });
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
            let run_mode = &mut self.run_mode;
            ui.label("Run mode:");
            ui.radio_value("Continuous", run_mode, RunMode::Continuous)
                .tooltip_text("Repaint everything each frame");
            ui.radio_value("Reactive", run_mode, RunMode::Reactive)
                .tooltip_text("Repaint when there are animations or input (e.g. mouse movement)");
        });

        if self.run_mode == RunMode::Continuous {
            ui.add(
                label!("Repainting the UI each frame. FPS: {:.1}", backend.fps())
                    .text_style(TextStyle::Monospace),
            );
        } else {
            ui.label("Only running UI code when there are animations or input");
        }

        self.num_frames_painted += 1;
        ui.label(format!("Total frames painted: {}", self.num_frames_painted));

        ui.separator();
        ui.checkbox(
            "Show color blend test (debug backend painter)",
            &mut self.show_color_test,
        );
    }
}

impl app::App for DemoApp {
    fn ui(&mut self, ui: &mut Ui, backend: &mut dyn app::Backend) {
        Window::new("Backend").scroll(false).show(ui.ctx(), |ui| {
            self.backend_ui(ui, backend);
        });

        let Self {
            show_color_test,
            color_test,
            ..
        } = self;

        if *show_color_test {
            let mut tex_loader = |size: (usize, usize), pixels: &[Srgba]| {
                backend.new_texture_srgba_premultiplied(size, pixels)
            };
            Window::new("Color Test")
                .default_size(vec2(1024.0, 1024.0))
                .scroll(true)
                .open(show_color_test)
                .show(ui.ctx(), |ui| {
                    color_test.ui(ui, &mut tex_loader);
                });
        }

        let web_info = backend.web_info();
        let web_location_hash = web_info
            .as_ref()
            .map(|info| info.web_location_hash.clone())
            .unwrap_or_default();

        let environment = DemoEnvironment {
            web_location_hash,
            seconds_since_midnight: backend.seconds_since_midnight(),
        };

        self.ui(ui, &environment);

        if self.run_mode == RunMode::Continuous {
            // Tell the backend to repaint as soon as possible
            ui.ctx().request_repaint();
        }
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
    resize: bool,
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
        }
    }
}

fn show_menu_bar(ui: &mut Ui, windows: &mut OpenWindows, env: &DemoEnvironment) {
    menu::bar(ui, |ui| {
        menu::menu(ui, "File", |ui| {
            if ui.add(Button::new("Reorganize windows")).clicked {
                ui.ctx().memory().reset_areas();
            }
            if ui
                .add(Button::new("Clear entire Egui memory"))
                .tooltip_text("Forget scroll, collapsibles etc")
                .clicked
            {
                *ui.ctx().memory() = Default::default();
            }
        });
        menu::menu(ui, "Windows", |ui| {
            let OpenWindows {
                demo,
                fractal_clock,
                settings,
                inspection,
                memory,
                resize,
            } = windows;
            ui.add(Checkbox::new(demo, "Demo"));
            ui.add(Checkbox::new(fractal_clock, "Fractal Clock"));
            ui.separator();
            ui.add(Checkbox::new(settings, "Settings"));
            ui.add(Checkbox::new(inspection, "Inspection"));
            ui.add(Checkbox::new(memory, "Memory"));
            ui.add(Checkbox::new(resize, "Resize examples"));
        });
        menu::menu(ui, "About", |ui| {
            ui.add(label!("This is Egui"));
            ui.add(Hyperlink::new("https://github.com/emilk/egui").text("Egui home page"));
        });

        if let Some(time) = env.seconds_since_midnight {
            let time = format!(
                "{:02}:{:02}:{:02}.{:02}",
                (time.rem_euclid(24.0 * 60.0 * 60.0) / 3600.0).floor(),
                (time.rem_euclid(60.0 * 60.0) / 60.0).floor(),
                (time.rem_euclid(60.0)).floor(),
                (time.rem_euclid(1.0) * 100.0).floor()
            );
            ui.set_layout(Layout::horizontal(Align::Max).reverse());
            if ui
                .add(Button::new(time).text_style(TextStyle::Monospace))
                .clicked
            {
                windows.fractal_clock = !windows.fractal_clock;
            }
        }
    });
}
