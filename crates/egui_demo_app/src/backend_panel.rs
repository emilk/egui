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
    /// or after every single frame - this is called [`Continuous`](RunMode::Continuous) mode,
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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct BackendPanel {
    pub open: bool,

    #[cfg_attr(feature = "serde", serde(skip))]
    // go back to [`RunMode::Reactive`] mode each time we start
    run_mode: RunMode,

    #[cfg_attr(feature = "serde", serde(skip))]
    frame_history: crate::frame_history::FrameHistory,

    egui_windows: EguiWindows,
}

impl BackendPanel {
    pub fn update(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        self.frame_history
            .on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);

        match self.run_mode {
            RunMode::Continuous => {
                // Tell the backend to repaint as soon as possible
                ctx.request_repaint();
            }
            RunMode::Reactive => {
                // let the computer rest for a bit
            }
        }
    }

    pub fn end_of_frame(&mut self, ctx: &egui::Context) {
        self.egui_windows.windows(ctx);
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        integration_ui(ui, frame);

        ui.separator();

        self.run_mode_ui(ui);

        ui.separator();

        self.frame_history.ui(ui);

        ui.separator();

        ui.label("egui windows:");
        self.egui_windows.checkboxes(ui);

        #[cfg(debug_assertions)]
        if ui.ctx().style().debug.debug_on_hover_with_all_modifiers {
            ui.separator();
            ui.label("Press down all modifiers and hover a widget to see a callstack for it");
        }

        #[cfg(target_arch = "wasm32")]
        {
            ui.separator();
            let mut screen_reader = ui.ctx().options(|o| o.screen_reader);
            ui.checkbox(&mut screen_reader, "🔈 Screen reader").on_hover_text("Experimental feature: checking this will turn on the screen reader on supported platforms");
            ui.ctx().options_mut(|o| o.screen_reader = screen_reader);
        }

        if cfg!(debug_assertions) && cfg!(target_arch = "wasm32") {
            ui.separator();
            // For testing panic handling on web:
            #[allow(clippy::manual_assert)]
            if ui.button("panic!()").clicked() {
                panic!("intentional panic!");
            }
        }

        if !cfg!(target_arch = "wasm32") {
            ui.separator();
            if ui.button("Quit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn run_mode_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let run_mode = &mut self.run_mode;
            ui.label("Mode:");
            ui.radio_value(run_mode, RunMode::Reactive, "Reactive")
                .on_hover_text("Repaint when there are animations or input (e.g. mouse movement)");
            ui.radio_value(run_mode, RunMode::Continuous, "Continuous")
                .on_hover_text("Repaint everything each frame");
        });

        if self.run_mode == RunMode::Continuous {
            ui.label(format!(
                "Repainting the UI each frame. FPS: {:.1}",
                self.frame_history.fps()
            ));
        } else {
            ui.label("Only running UI code when there are animations or input.");

            // Add a test for `request_repaint_after`, but only in debug
            // builds to keep the noise down in the official demo.
            if cfg!(debug_assertions) {
                ui.collapsing("More…", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Frame number:");
                        ui.monospace(ui.ctx().frame_nr().to_string());
                    });
                    if ui
                        .button("Wait 2s, then request repaint after another 3s")
                        .clicked()
                    {
                        log::info!("Waiting 2s before requesting repaint...");
                        let ctx = ui.ctx().clone();
                        call_after_delay(std::time::Duration::from_secs(2), move || {
                            log::info!("Request a repaint in 3s...");
                            ctx.request_repaint_after(std::time::Duration::from_secs(3));
                        });
                    }
                });
            }
        }
    }
}

fn integration_ui(ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("egui running inside ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });

    #[cfg(target_arch = "wasm32")]
    ui.collapsing("Web info (location)", |ui| {
        ui.style_mut().wrap = Some(false);
        ui.monospace(format!("{:#?}", _frame.info().web_info.location));
    });

    #[cfg(not(target_arch = "wasm32"))]
    {
        ui.horizontal(|ui| {
            {
                let mut fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                if ui
                    .checkbox(&mut fullscreen, "🗖 Fullscreen (F11)")
                    .on_hover_text("Fullscreen the window")
                    .changed()
                {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Fullscreen(fullscreen));
                }
            }

            if ui
                .button("📱 Phone Size")
                .on_hover_text("Resize the window to be small like a phone.")
                .clicked()
            {
                // let size = egui::vec2(375.0, 812.0); // iPhone 12 mini
                let size = egui::vec2(375.0, 667.0); //  iPhone SE 2nd gen

                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                ui.close_menu();
            }
        });

        let fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if !fullscreen
            && ui
                .button("Drag me to drag window")
                .is_pointer_button_down_on()
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct EguiWindows {
    // egui stuff:
    settings: bool,
    inspection: bool,
    memory: bool,
    output_events: bool,

    #[cfg_attr(feature = "serde", serde(skip))]
    output_event_history: std::collections::VecDeque<egui::output::OutputEvent>,
}

impl Default for EguiWindows {
    fn default() -> Self {
        EguiWindows::none()
    }
}

impl EguiWindows {
    fn none() -> Self {
        Self {
            settings: false,
            inspection: false,
            memory: false,
            output_events: false,
            output_event_history: Default::default(),
        }
    }

    fn checkboxes(&mut self, ui: &mut egui::Ui) {
        let Self {
            settings,
            inspection,
            memory,
            output_events,
            output_event_history: _,
        } = self;

        ui.checkbox(settings, "🔧 Settings");
        ui.checkbox(inspection, "🔍 Inspection");
        ui.checkbox(memory, "📝 Memory");
        ui.checkbox(output_events, "📤 Output Events");
    }

    fn windows(&mut self, ctx: &egui::Context) {
        let Self {
            settings,
            inspection,
            memory,
            output_events,
            output_event_history,
        } = self;

        ctx.output(|o| {
            for event in &o.events {
                output_event_history.push_back(event.clone());
            }
        });
        while output_event_history.len() > 1000 {
            output_event_history.pop_front();
        }

        egui::Window::new("🔧 Settings")
            .open(settings)
            .vscroll(true)
            .show(ctx, |ui| {
                ctx.settings_ui(ui);
            });

        egui::Window::new("🔍 Inspection")
            .open(inspection)
            .vscroll(true)
            .show(ctx, |ui| {
                ctx.inspection_ui(ui);
            });

        egui::Window::new("📝 Memory")
            .open(memory)
            .resizable(false)
            .show(ctx, |ui| {
                ctx.memory_ui(ui);
            });

        egui::Window::new("📤 Output Events")
            .open(output_events)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.label(
                    "Recent output events from egui. \
            These are emitted when you interact with widgets, or move focus between them with TAB. \
            They can be hooked up to a screen reader on supported platforms.",
                );

                ui.separator();

                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for event in output_event_history {
                            ui.label(format!("{event:?}"));
                        }
                    });
            });
    }
}

// ----------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
fn call_after_delay(delay: std::time::Duration, f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name("call_after_delay".to_owned())
        .spawn(move || {
            std::thread::sleep(delay);
            f();
        })
        .unwrap();
}

#[cfg(target_arch = "wasm32")]
fn call_after_delay(delay: std::time::Duration, f: impl FnOnce() + Send + 'static) {
    use wasm_bindgen::prelude::*;
    let window = web_sys::window().unwrap();
    let closure = Closure::once(f);
    let delay_ms = delay.as_millis() as _;
    window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            delay_ms,
        )
        .unwrap();
    closure.forget(); // We must forget it, or else the callback is canceled on drop
}
