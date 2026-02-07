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

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        use crate::View as _;
        egui::Window::new("Tooltips")
            .constrain(false) // So we can test how tooltips behave close to the screen edge
            .resizable(true)
            .default_size([450.0, 300.0])
            .scroll(false)
            .open(open)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| self.ui(ui));
    }
}

impl crate::View for Tooltips {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 8.0;

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file_line!());
        });

        egui::Panel::right("scroll_test").show_inside(ui, |ui| {
            ui.label(
                "The scroll area below has many labels with interactive tooltips. \
                 The purpose is to test that the tooltips close when you scroll.",
            )
            .on_hover_text("Try hovering a label below, then scroll!");
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    for i in 0..1000 {
                        ui.label(format!("This is line {i}")).on_hover_ui(|ui| {
                            ui.style_mut().interaction.selectable_labels = true;
                            ui.label(
                            "This tooltip is interactive, because the text in it is selectable.",
                        );
                        });
                    }
                });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.misc_tests(ui);
        });
    }
}

impl Tooltips {
    fn misc_tests(&mut self, ui: &mut egui::Ui) {
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

        ui.label("This tooltip shows at the mouse cursor.")
            .on_hover_text_at_pointer("Move me around!!");

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

        ui.label("You can have different tooltips depending on whether or not a widget is enabled:")
            .on_hover_text("Check the tooltip of the button below, and see how it changes depending on whether or not it is enabled.");

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.enabled, "Enabled")
                .on_hover_text("Controls whether or not the following button is enabled.");

            ui.add_enabled(self.enabled, egui::Button::new("Sometimes clickable"))
                .on_hover_ui(tooltip_ui)
                .on_disabled_hover_ui(disabled_tooltip_ui);
        });
    }
}
