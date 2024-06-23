#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Tooltips {
    enabled: bool,
}

impl Default for Tooltips {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl crate::Demo for Tooltips {
    fn name(&self) -> &'static str {
        "🗖 Tooltips"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        use crate::View as _;
        let window = egui::Window::new("Tooltips")
            .constrain(false) // So we can test how tooltips behave close to the screen edge
            .resizable(false)
            .scroll(false)
            .open(open);
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl crate::View for Tooltips {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 8.0;

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file_line!());
        });

        ui.label("All labels in this demo have tooltips.")
            .on_hover_text("Yes, even this one.");

        ui.label("Some widgets have multiple tooltips!")
            .on_hover_text("The first tooltip.")
            .on_hover_text("The second tooltip.");

        ui.label("Tooltips can contain interactive widgets.")
            .on_hover_ui(|ui| {
                ui.label("This tooltip contains a link:");
                ui.hyperlink_to("www.egui.rs", "https://www.egui.rs/")
                    .on_hover_text("The tooltip has a tooltip in it!");
            });

        ui.label("You can put selectable text in tooltips too.")
            .on_hover_ui(|ui| {
                ui.style_mut().interaction.selectable_labels = true;
                ui.label("You can select this text.");
            });

        ui.separator(); // ---------------------------------------------------------

        let tooltip_ui = |ui: &mut egui::Ui| {
            ui.horizontal(|ui| {
                ui.label("This tooltip was created with");
                ui.code(".on_hover_ui(…)");
            });
        };
        let disabled_tooltip_ui = |ui: &mut egui::Ui| {
            ui.label("A different tooltip when widget is disabled.");
            ui.horizontal(|ui| {
                ui.label("This tooltip was created with");
                ui.code(".on_disabled_hover_ui(…)");
            });
        };

        ui.label("You can have different tooltips depending on whether or not a widget is enabled or not:")
            .on_hover_text("Check the tooltip of the button below, and see how it changes dependning on whether or not it is enabled.");

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.enabled, "Enabled")
                .on_hover_text("Controls whether or not the following button is enabled.");

            ui.add_enabled(self.enabled, egui::Button::new("Sometimes clickable"))
                .on_hover_ui(tooltip_ui)
                .on_disabled_hover_ui(disabled_tooltip_ui);
        });
    }
}
