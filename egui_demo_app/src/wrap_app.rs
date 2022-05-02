use egui_demo_lib::is_mobile;
use egui_glow::glow;

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct EasyMarkApp {
    editor: egui_demo_lib::easy_mark::EasyMarkEditor,
}

impl eframe::App for EasyMarkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.editor.panels(ctx);
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DemoApp {
    demo_windows: egui_demo_lib::DemoWindows,
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.demo_windows.ui(ctx);
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FractalClockApp {
    fractal_clock: crate::apps::FractalClock,
}

impl eframe::App for FractalClockApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::dark_canvas(&ctx.style()))
            .show(ctx, |ui| {
                self.fractal_clock.ui(ui, crate::seconds_since_midnight());
            });
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ColorTestApp {
    color_test: egui_demo_lib::ColorTest,
}

impl eframe::App for ColorTestApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if frame.is_web() {
                ui.label(
                    "NOTE: Some old browsers stuck on WebGL1 without sRGB support will not pass the color test.",
                );
                ui.separator();
            }
            egui::ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                self.color_test.ui(ui);
            });
        });
    }
}

// ----------------------------------------------------------------------------

/// The state that we persist (serialize).
#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct State {
    demo: DemoApp,
    easy_mark_editor: EasyMarkApp,
    #[cfg(feature = "http")]
    http: crate::apps::HttpApp,
    clock: FractalClockApp,
    color_test: ColorTestApp,

    selected_anchor: String,
    backend_panel: super::backend_panel::BackendPanel,
}

/// Wraps many demo/test apps into one.
pub struct WrapApp {
    state: State,
    // not serialized (because it contains OpenGL buffers etc)
    custom3d: crate::apps::Custom3d,
    dropped_files: Vec<egui::DroppedFile>,
}

impl WrapApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        #[allow(unused_mut)]
        let mut slf = Self {
            state: State::default(),
            custom3d: crate::apps::Custom3d::new(&cc.gl),
            dropped_files: Default::default(),
        };

        #[cfg(feature = "persistence")]
        if let Some(storage) = cc.storage {
            if let Some(state) = eframe::get_value(storage, eframe::APP_KEY) {
                slf.state = state;
            }
        }

        if cc.integration_info.prefer_dark_mode == Some(false) {
            cc.egui_ctx.set_visuals(egui::Visuals::light()); // use light mode if explicitly asked for
        } else {
            cc.egui_ctx.set_visuals(egui::Visuals::dark()); // use dark mode if there is no preference, or the preference is dark mode
        }

        slf
    }

    fn apps_iter_mut(&mut self) -> impl Iterator<Item = (&str, &str, &mut dyn eframe::App)> {
        vec![
            (
                "✨ Demos",
                "demo",
                &mut self.state.demo as &mut dyn eframe::App,
            ),
            (
                "🖹 EasyMark editor",
                "easymark",
                &mut self.state.easy_mark_editor as &mut dyn eframe::App,
            ),
            #[cfg(feature = "http")]
            (
                "⬇ HTTP",
                "http",
                &mut self.state.http as &mut dyn eframe::App,
            ),
            (
                "🕑 Fractal Clock",
                "clock",
                &mut self.state.clock as &mut dyn eframe::App,
            ),
            (
                "🔺 3D painting",
                "custom3e",
                &mut self.custom3d as &mut dyn eframe::App,
            ),
            (
                "🎨 Color test",
                "colors",
                &mut self.state.color_test as &mut dyn eframe::App,
            ),
        ]
        .into_iter()
    }
}

impl eframe::App for WrapApp {
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> egui::Rgba {
        visuals.window_fill().into()
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(web_info) = frame.info().web_info.as_ref() {
            if let Some(anchor) = web_info.location.hash.strip_prefix('#') {
                self.state.selected_anchor = anchor.to_owned();
            }
        }

        if self.state.selected_anchor.is_empty() {
            let selected_anchor = self.apps_iter_mut().next().unwrap().0.to_owned();
            self.state.selected_anchor = selected_anchor;
        }

        egui::TopBottomPanel::top("wrap_app_top_bar").show(ctx, |ui| {
            egui::trace!(ui);
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = false;
                self.bar_contents(ui, frame);
            });
        });

        self.state.backend_panel.update(ctx, frame);

        if !is_mobile(ctx)
            && (self.state.backend_panel.open || ctx.memory().everything_is_visible())
        {
            egui::SidePanel::left("backend_panel")
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("💻 Backend");
                    });

                    ui.separator();
                    self.backend_panel_contents(ui, frame);
                });
        }

        self.show_selected_app(ctx, frame);

        self.state.backend_panel.end_of_frame(ctx);

        self.ui_file_drag_and_drop(ctx);
    }

    fn on_exit(&mut self, gl: &glow::Context) {
        self.custom3d.on_exit(gl);
    }
}

impl WrapApp {
    fn backend_panel_contents(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.state.backend_panel.ui(ui, frame);

        ui.separator();

        ui.horizontal(|ui| {
            if ui
                .button("Reset egui")
                .on_hover_text("Forget scroll, positions, sizes etc")
                .clicked()
            {
                *ui.ctx().memory() = Default::default();
                ui.close_menu();
            }

            if ui.button("Reset everything").clicked() {
                self.state = Default::default();
                *ui.ctx().memory() = Default::default();
                ui.close_menu();
            }
        });
    }

    fn show_selected_app(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut found_anchor = false;
        let selected_anchor = self.state.selected_anchor.clone();
        for (_name, anchor, app) in self.apps_iter_mut() {
            if anchor == selected_anchor || ctx.memory().everything_is_visible() {
                app.update(ctx, frame);
                found_anchor = true;
            }
        }
        if !found_anchor {
            self.state.selected_anchor = "demo".into();
        }
    }

    fn bar_contents(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::widgets::global_dark_light_mode_switch(ui);

        ui.separator();

        if is_mobile(ui.ctx()) {
            ui.menu_button("💻 Backend", |ui| {
                ui.set_style(ui.ctx().style()); // ignore the "menu" style set by `menu_button`.
                self.backend_panel_contents(ui, frame);
            });
        } else {
            ui.toggle_value(&mut self.state.backend_panel.open, "💻 Backend");
        }

        ui.separator();

        let mut selected_anchor = self.state.selected_anchor.clone();
        for (name, anchor, _app) in self.apps_iter_mut() {
            if ui
                .selectable_label(selected_anchor == anchor, name)
                .clicked()
            {
                selected_anchor = anchor.to_owned();
                if frame.is_web() {
                    ui.output().open_url(format!("#{}", anchor));
                }
            }
        }
        self.state.selected_anchor = selected_anchor;

        ui.with_layout(egui::Layout::right_to_left(), |ui| {
            if false {
                // TODO: fix the overlap on small screens
                if let Some(seconds_since_midnight) = crate::seconds_since_midnight() {
                    if clock_button(ui, seconds_since_midnight).clicked() {
                        self.state.selected_anchor = "clock".to_owned();
                        if frame.is_web() {
                            ui.output().open_url("#clock");
                        }
                    }
                }
            }

            egui::warn_if_debug_build(ui);
        });
    }

    fn ui_file_drag_and_drop(&mut self, ctx: &egui::Context) {
        use egui::*;

        // Preview hovering files:
        if !ctx.input().raw.hovered_files.is_empty() {
            let mut text = "Dropping files:\n".to_owned();
            for file in &ctx.input().raw.hovered_files {
                if let Some(path) = &file.path {
                    text += &format!("\n{}", path.display());
                } else if !file.mime.is_empty() {
                    text += &format!("\n{}", file.mime);
                } else {
                    text += "\n???";
                }
            }

            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

            let screen_rect = ctx.input().screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                text,
                TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }

        // Collect dropped files:
        if !ctx.input().raw.dropped_files.is_empty() {
            self.dropped_files = ctx.input().raw.dropped_files.clone();
        }

        // Show dropped files (if any):
        if !self.dropped_files.is_empty() {
            let mut open = true;
            egui::Window::new("Dropped files")
                .open(&mut open)
                .show(ctx, |ui| {
                    for file in &self.dropped_files {
                        let mut info = if let Some(path) = &file.path {
                            path.display().to_string()
                        } else if !file.name.is_empty() {
                            file.name.clone()
                        } else {
                            "???".to_owned()
                        };
                        if let Some(bytes) = &file.bytes {
                            info += &format!(" ({} bytes)", bytes.len());
                        }
                        ui.label(info);
                    }
                });
            if !open {
                self.dropped_files.clear();
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

    ui.button(egui::RichText::new(time).monospace())
}
