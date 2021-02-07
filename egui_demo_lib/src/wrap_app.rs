/// All the different demo apps.
#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Apps {
    demo: crate::apps::DemoApp,
    easy_mark_editor: crate::apps::EasyMarkEditor,
    #[cfg(feature = "http")]
    http: crate::apps::HttpApp,
    clock: crate::apps::FractalClock,
    color_test: crate::apps::ColorTest,
}

impl Apps {
    fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut dyn epi::App)> {
        vec![
            ("demo", &mut self.demo as &mut dyn epi::App),
            ("easymark", &mut self.easy_mark_editor as &mut dyn epi::App),
            #[cfg(feature = "http")]
            ("http", &mut self.http as &mut dyn epi::App),
            ("clock", &mut self.clock as &mut dyn epi::App),
            ("colors", &mut self.color_test as &mut dyn epi::App),
        ]
        .into_iter()
    }
}

/// Wraps many demo/test apps into one.
#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct WrapApp {
    selected_anchor: String,
    apps: Apps,
    backend_panel: BackendPanel,
}

impl epi::App for WrapApp {
    fn name(&self) -> &str {
        "egui demo apps"
    }

    #[cfg(feature = "persistence")]
    fn load(&mut self, storage: &dyn epi::Storage) {
        *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
    }

    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    fn warm_up_enabled(&self) -> bool {
        // The example windows use a lot of emojis. Pre-cache them by running one frame where everything is open
        #[cfg(debug_assertions)]
        {
            false // debug
        }
        #[cfg(not(debug_assertions))]
        {
            true // release
        }
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        if let Some(web_info) = frame.info().web_info.as_ref() {
            if let Some(anchor) = web_info.web_location_hash.strip_prefix("#") {
                self.selected_anchor = anchor.to_owned();
            }
        }

        if self.selected_anchor.is_empty() {
            self.selected_anchor = self.apps.iter_mut().next().unwrap().0.to_owned();
        }

        egui::TopPanel::top("wrap_app_top_bar").show(ctx, |ui| {
            // A menu-bar is a horizontal layout with some special styles applied.
            // egui::menu::bar(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                dark_light_mode_switch(ui);

                ui.checkbox(&mut self.backend_panel.open, "💻 Backend");
                ui.separator();

                for (anchor, app) in self.apps.iter_mut() {
                    if ui
                        .selectable_label(self.selected_anchor == anchor, app.name())
                        .clicked()
                    {
                        self.selected_anchor = anchor.to_owned();
                        if frame.is_web() {
                            ui.output().open_url = Some(format!("#{}", anchor));
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(), |ui| {
                    if false {
                        // TODO: fix the overlap on small screens
                        if let Some(seconds_since_midnight) = frame.info().seconds_since_midnight {
                            if clock_button(ui, seconds_since_midnight).clicked() {
                                self.selected_anchor = "clock".to_owned();
                                if frame.is_web() {
                                    ui.output().open_url = Some("#clock".to_owned());
                                }
                            }
                        }
                    }

                    egui::warn_if_debug_build(ui);
                });
            });
        });

        self.backend_panel.update(ctx, frame);
        if self.backend_panel.open || ctx.memory().everything_is_visible() {
            egui::SidePanel::left("backend_panel", 150.0).show(ctx, |ui| {
                self.backend_panel.ui(ui, frame);
            });
        }

        for (anchor, app) in self.apps.iter_mut() {
            if anchor == self.selected_anchor || ctx.memory().everything_is_visible() {
                app.update(ctx, frame);
            }
        }
    }
}

fn clock_button(ui: &mut egui::Ui, seconds_since_midnight: f64) -> egui::Response {
    let time = seconds_since_midnight;
    let time = format!(
        "{:02}:{:02}:{:02}.{:02}",
        (time % (24.0 * 60.0 * 60.0) / 3600.0).floor(),
        (time % (60.0 * 60.0) / 60.0).floor(),
        (time % 60.0).floor(),
        (time % 1.0 * 100.0).floor()
    );

    ui.add(egui::Button::new(time).text_style(egui::TextStyle::Monospace))
}

/// Show a button to switch to/from dark/light mode (globally).
fn dark_light_mode_switch(ui: &mut egui::Ui) {
    let style: egui::Style = (*ui.ctx().style()).clone();
    let new_visuals = style.visuals.light_dark_small_toggle_button(ui);
    if let Some(visuals) = new_visuals {
        ui.ctx().set_visuals(visuals);
    }
}

// ----------------------------------------------------------------------------

/// How often we repaint the demo app by default
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RunMode {
    /// This is the default for the demo.
    ///
    /// If this is selected, egui is only updated if are input events
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
    /// On most platforms this will mean that egui will run at the display refresh rate of e.g. 60 Hz.
    ///
    /// For this demo it is not any reason to do so except to
    /// demonstrate how quickly egui runs.
    ///
    /// For games or other interactive apps, this is probably what you want to do.
    /// It will guarantee that egui is always up-to-date.
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

#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
struct BackendPanel {
    open: bool,

    #[cfg_attr(feature = "persistence", serde(skip))]
    // go back to `Reactive` mode each time we start
    run_mode: RunMode,

    /// current slider value for current gui scale
    pixels_per_point: Option<f32>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    frame_history: crate::frame_history::FrameHistory,
}

impl BackendPanel {
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        self.frame_history
            .on_new_frame(ctx.input().time, frame.info().cpu_usage);

        if self.run_mode == RunMode::Continuous {
            // Tell the backend to repaint as soon as possible
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut epi::Frame<'_>) {
        ui.heading("💻 Backend");

        self.run_mode_ui(ui);

        ui.separator();

        self.frame_history.ui(ui);

        if !frame.is_web() {
            // web browsers have their own way of zooming, which egui_web respects
            ui.separator();
            if let Some(new_pixels_per_point) = self.pixels_per_point_ui(ui, frame.info()) {
                frame.set_pixels_per_point(new_pixels_per_point);
            }
        }

        ui.separator();

        if frame.is_web() {
            ui.label("egui is an immediate mode GUI written in Rust, compiled to WebAssembly, rendered with WebGL.");
            ui.label(
                "Everything you see is rendered as textured triangles. There is no DOM. There are no HTML elements. \
                This is not JavaScript. This is Rust, running at 60 FPS. This is the web page, reinvented with game tech.");
            ui.label("This is also work in progress, and not ready for production... yet :)");
            ui.horizontal_wrapped_for_text(egui::TextStyle::Body, |ui| {
                ui.label("Project home page:");
                ui.hyperlink("https://github.com/emilk/egui");
            });
        } else {
            if ui
                .button("📱 Phone Size")
                .on_hover_text("Resize the window to be small like a phone.")
                .clicked()
            {
                frame.set_window_size(egui::Vec2::new(375.0, 812.0)); // iPhone 12 mini
            }
            if ui.button("Quit").clicked() {
                frame.quit();
            }
        }
    }

    fn pixels_per_point_ui(
        &mut self,
        ui: &mut egui::Ui,
        info: &epi::IntegrationInfo,
    ) -> Option<f32> {
        #![allow(clippy::float_cmp)]

        self.pixels_per_point = self
            .pixels_per_point
            .or(info.native_pixels_per_point)
            .or_else(|| Some(ui.ctx().pixels_per_point()));

        let pixels_per_point = self.pixels_per_point.as_mut()?;

        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = 90.0;
            ui.add(
                egui::Slider::f32(pixels_per_point, 0.5..=5.0)
                    .logarithmic(true)
                    .text("Scale"),
            )
            .on_hover_text("Physical pixels per point.");
            if let Some(native_pixels_per_point) = info.native_pixels_per_point {
                let button = egui::Button::new("Reset")
                    .enabled(*pixels_per_point != native_pixels_per_point);
                if ui
                    .add(button)
                    .on_hover_text(format!(
                        "Reset scale to native value ({:.1})",
                        native_pixels_per_point
                    ))
                    .clicked()
                {
                    *pixels_per_point = native_pixels_per_point;
                }
            }
        });

        // We wait until mouse release to activate:
        if ui.ctx().is_using_pointer() {
            None
        } else {
            Some(*pixels_per_point)
        }
    }

    fn run_mode_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let run_mode = &mut self.run_mode;
            ui.label("Mode:");
            ui.radio_value(run_mode, RunMode::Continuous, "Continuous")
                .on_hover_text("Repaint everything each frame");
            ui.radio_value(run_mode, RunMode::Reactive, "Reactive")
                .on_hover_text("Repaint when there are animations or input (e.g. mouse movement)");
        });

        if self.run_mode == RunMode::Continuous {
            ui.label(format!(
                "Repainting the UI each frame. FPS: {:.1}",
                self.frame_history.fps()
            ));
        } else {
            ui.label("Only running UI code when there are animations or input");
        }
    }
}
