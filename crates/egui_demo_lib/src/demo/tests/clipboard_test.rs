pub struct ClipboardTest {
    text: String,
}

impl Default for ClipboardTest {
    fn default() -> Self {
        Self {
            text: "Example text you can copy-and-paste".to_owned(),
        }
    }
}

impl crate::Demo for ClipboardTest {
    fn name(&self) -> &'static str {
        "Clipboard Test"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for ClipboardTest {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("egui integrates with the system clipboard.");
        ui.label("Try copy-cut-pasting text in the text edit below.");

        let text_edit_response = ui
            .horizontal(|ui| {
                let text_edit_response = ui.text_edit_singleline(&mut self.text);
                if ui.button("📋").clicked() {
                    ui.copy_text(self.text.clone());
                }
                text_edit_response
            })
            .inner;

        if !cfg!(target_arch = "wasm32") {
            // These commands are not yet implemented on web
            ui.horizontal(|ui| {
                for (name, cmd) in [
                    ("Copy", egui::ViewportCommand::RequestCopy),
                    ("Cut", egui::ViewportCommand::RequestCut),
                    ("Paste", egui::ViewportCommand::RequestPaste),
                ] {
                    if ui.button(name).clicked() {
                        // Next frame we should get a copy/cut/paste-event…
                        ui.send_viewport_cmd(cmd);

                        // …that should en up here:
                        text_edit_response.request_focus();
                    }
                }
            });
        }

        ui.separator();

        ui.label("You can also copy images:");
        ui.horizontal(|ui| {
            let image_source = egui::include_image!("../../../data/icon.png");
            let uri = image_source.uri().unwrap().to_owned();
            ui.image(image_source);

            if let Ok(egui::load::ImagePoll::Ready { image }) =
                ui.ctx().try_load_image(&uri, Default::default())
                && ui.button("📋").clicked()
            {
                ui.copy_image((*image).clone());
            }
        });

        ui.vertical_centered_justified(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}
