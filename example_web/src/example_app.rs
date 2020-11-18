use egui_web::fetch::Response;
use std::sync::mpsc::Receiver;

pub struct ExampleApp {
    url: String,
    receivers: Vec<Receiver<Result<Response, String>>>,
    fetch_result: Option<Result<Response, String>>,
}

impl Default for ExampleApp {
    fn default() -> Self {
        Self {
            url: "https://raw.githubusercontent.com/emilk/egui/master/README.md".to_owned(),
            receivers: Default::default(),
            fetch_result: Default::default(),
        }
    }
}

impl egui::app::App for ExampleApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn ui(
        &mut self,
        ctx: &std::sync::Arc<egui::Context>,
        _integration_context: &mut egui::app::IntegrationContext,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("HTTP Get inside of Egui");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/egui/blob/master/",
                "(source code)"
            ));

            {
                let mut trigger_fetch = false;

                ui.horizontal(|ui| {
                    ui.label("URL:");
                    trigger_fetch |= ui.text_edit_singleline(&mut self.url).lost_kb_focus;

                    if ui.button("Egui README.md").clicked {
                        self.url = "https://raw.githubusercontent.com/emilk/egui/master/README.md"
                            .to_owned();
                        trigger_fetch = true;
                    }
                    if ui.button("Source code for this file").clicked {
                        self.url =
                            format!("https://raw.githubusercontent.com/emilk/egui/{}", file!());
                        trigger_fetch = true;
                    }
                });
                trigger_fetch |= ui.button("GET").clicked;

                if trigger_fetch {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    self.receivers.push(receiver);
                    let url = self.url.clone();

                    let future = async move {
                        let result = egui_web::fetch::get_text(&url).await;
                        sender.send(result).ok();
                        // TODO: trigger egui repaint somehow
                    };

                    egui_web::spawn_future(future);
                }
            }

            // Show finished download (if any):
            if let Some(result) = &self.fetch_result {
                ui.separator();
                match result {
                    Ok(response) => {
                        ui_response(ui, response);
                    }
                    Err(error) => {
                        // This should only happen if the fetch API isn't available or something similar.
                        ui.add(egui::Label::new(error).text_color(egui::color::RED));
                    }
                }
            }
        });

        for i in (0..self.receivers.len()).rev() {
            if let Ok(result) = self.receivers[i].try_recv() {
                self.fetch_result = Some(result);
                let _ = self.receivers.swap_remove(i);
            }
        }
    }
}

fn ui_response(ui: &mut egui::Ui, response: &Response) {
    ui.monospace(format!("url:         {}", response.url));
    ui.monospace(format!(
        "status:      {} ({})",
        response.status, response.status_text
    ));

    ui.monospace("Body:");
    ui.separator();
    egui::ScrollArea::auto_sized().show(ui, |ui| {
        ui.monospace(&response.body);
    });
}
