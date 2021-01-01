/// Wraps many demo/test apps into one
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct WrapApp {
    selected_anchor: String,
    apps: Apps,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Apps {
    demo: crate::apps::DemoApp,
    http: crate::apps::HttpApp,
    clock: crate::apps::FractalClock,
    color_test: crate::apps::ColorTest,
}

impl Apps {
    fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut dyn epi::App)> {
        vec![
            ("demo", &mut self.demo as &mut dyn epi::App),
            ("http", &mut self.http as &mut dyn epi::App),
            ("clock", &mut self.clock as &mut dyn epi::App),
            ("colors", &mut self.color_test as &mut dyn epi::App),
        ]
        .into_iter()
    }
}

impl epi::App for WrapApp {
    fn name(&self) -> &str {
        "Egui Demo Apps"
    }

    fn load(&mut self, storage: &dyn epi::Storage) {
        *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
    }

    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
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

        egui::TopPanel::top("wrap_app").show(ctx, |ui| {
            // A menu-bar is a horizontal layout with some special styles applied.
            egui::menu::bar(ui, |ui| {
                for (anchor, app) in self.apps.iter_mut() {
                    if ui
                        .selectable_label(self.selected_anchor == anchor, app.name())
                        .clicked
                    {
                        self.selected_anchor = anchor.to_owned();
                        if frame.is_web() {
                            ui.output().open_url = Some(format!("#{}", anchor));
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(), |ui| {
                    if let Some(seconds_since_midnight) = frame.info().seconds_since_midnight {
                        if clock_button(ui, seconds_since_midnight).clicked {
                            self.selected_anchor = "clock".to_owned();
                            if frame.is_web() {
                                ui.output().open_url = Some("#clock".to_owned());
                            }
                        }
                    }

                    egui::warn_if_debug_build(ui);
                });
            });
        });

        for (anchor, app) in self.apps.iter_mut() {
            if anchor == self.selected_anchor {
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
