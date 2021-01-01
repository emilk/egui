/// Wraps many demo/test apps into one
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct WrapApp {
    selectable_demo_name: String,

    demo: crate::apps::DemoApp,
    http: crate::apps::HttpApp,
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

    fn ui(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let web_location_hash = frame
            .info()
            .web_info
            .as_ref()
            .map(|info| info.web_location_hash.clone())
            .unwrap_or_default();

        if web_location_hash == "#clock" {
            // TODO
        } else if web_location_hash == "#http" {
            self.selectable_demo_name = self.http.name().to_owned();
        }

        egui::TopPanel::top("wrap_app").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("web_location_hash: {:?}", web_location_hash));
                ui.label("Demo Apps:");
                ui.selectable_value(
                    &mut self.selectable_demo_name,
                    self.demo.name().to_owned(),
                    self.demo.name(),
                );
                ui.selectable_value(
                    &mut self.selectable_demo_name,
                    self.http.name().to_owned(),
                    self.http.name(),
                );

                ui.with_layout(egui::Layout::right_to_left(), |ui| {
                    egui::warn_if_debug_build(ui);
                });
            });
        });

        if self.selectable_demo_name == self.demo.name() {
            self.demo.ui(ctx, frame);
        } else if self.selectable_demo_name == self.http.name() {
            self.http.ui(ctx, frame);
        } else {
            self.selectable_demo_name = self.demo.name().to_owned();
        }
    }
}
