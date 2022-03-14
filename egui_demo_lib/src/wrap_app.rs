/// All the different demo apps.
#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Apps {
    demo: crate::apps::DemoApp,
    easy_mark_editor: crate::easy_mark::EasyMarkEditor,
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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct WrapApp {
    selected_anchor: String,
    apps: Apps,
    backend_panel: super::backend_panel::BackendPanel,
    #[cfg_attr(feature = "serde", serde(skip))]
    dropped_files: Vec<egui::DroppedFile>,
}

impl epi::App for WrapApp {
    fn name(&self) -> &str {
        "egui demo apps"
    }

    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
        _gl: &std::rc::Rc<epi::glow::Context>,
    ) {
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default();
        }
    }

    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    fn max_size_points(&self) -> egui::Vec2 {
        self.backend_panel.max_size_points_active
    }

    fn clear_color(&self) -> egui::Rgba {
        egui::Rgba::TRANSPARENT // we set a `CentralPanel` fill color in `demo_windows.rs`
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        if let Some(web_info) = frame.info().web_info.as_ref() {
            if let Some(anchor) = web_info.location.hash.strip_prefix('#') {
                self.selected_anchor = anchor.to_owned();
            }
        }

        if self.selected_anchor.is_empty() {
            self.selected_anchor = self.apps.iter_mut().next().unwrap().0.to_owned();
        }

        egui::TopBottomPanel::top("wrap_app_top_bar").show(ctx, |ui| {
            egui::trace!(ui);
            self.bar_contents(ui, frame);
        });

        self.backend_panel.update(ctx, frame);

        if self.backend_panel.open || ctx.memory().everything_is_visible() {
            egui::SidePanel::left("backend_panel").show(ctx, |ui| {
                self.backend_panel.ui(ui, frame);

                ui.separator();

                ui.horizontal(|ui| {
                    if ui
                        .button("Reset egui")
                        .on_hover_text("Forget scroll, positions, sizes etc")
                        .clicked()
                    {
                        *ui.ctx().memory() = Default::default();
                    }

                    if ui.button("Reset everything").clicked() {
                        *self = Default::default();
                        *ui.ctx().memory() = Default::default();
                    }
                });
            });
        }

        let mut found_anchor = false;

        for (anchor, app) in self.apps.iter_mut() {
            if anchor == self.selected_anchor || ctx.memory().everything_is_visible() {
                app.update(ctx, frame);
                found_anchor = true;
            }
        }

        if !found_anchor {
            self.selected_anchor = "demo".into();
        }

        self.backend_panel.end_of_frame(ctx);

        self.ui_file_drag_and_drop(ctx);
    }
}

impl WrapApp {
    fn bar_contents(&mut self, ui: &mut egui::Ui, frame: &epi::Frame) {
        // A menu-bar is a horizontal layout with some special styles applied.
        // egui::menu::bar(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            egui::widgets::global_dark_light_mode_switch(ui);

            ui.checkbox(&mut self.backend_panel.open, "💻 Backend");
            ui.separator();

            for (anchor, app) in self.apps.iter_mut() {
                if ui
                    .selectable_label(self.selected_anchor == anchor, app.name())
                    .clicked()
                {
                    self.selected_anchor = anchor.to_owned();
                    if frame.is_web() {
                        ui.output().open_url(format!("#{}", anchor));
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                if false {
                    // TODO: fix the overlap on small screens
                    if let Some(seconds_since_midnight) = crate::seconds_since_midnight() {
                        if clock_button(ui, seconds_since_midnight).clicked() {
                            self.selected_anchor = "clock".to_owned();
                            if frame.is_web() {
                                ui.output().open_url("#clock");
                            }
                        }
                    }
                }

                egui::warn_if_debug_build(ui);
            });
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
