#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Highlighting {}

impl super::Demo for Highlighting {
    fn name(&self) -> &'static str {
        "Highlighting"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .default_width(320.0)
            .open(open)
            .show(ctx, |ui| {
                use super::View as _;
                self.ui(ui);
            });
    }
}

impl super::View for Highlighting {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.label("This demo demonstrates highlighting a widget.");
        ui.add_space(4.0);
        let label_response = ui.label("Hover me to highlight the button!");
        ui.add_space(4.0);
        let mut button_response = ui.button("Hover the button to highlight the label!");

        if label_response.hovered() {
            button_response = button_response.highlight();
        }
        if button_response.hovered() {
            label_response.highlight();
        }
    }
}
