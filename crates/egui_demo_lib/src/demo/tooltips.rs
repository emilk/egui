use crate::i18n;

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
        "💬 Tooltips"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        use crate::View as _;
        egui::Window::new(i18n::t("Tooltips"))
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

        // `end` anchors at the reading end (the physical right under LTR, the
        // physical left under RTL), so the second-read column mirrors with the
        // reading direction. The id salt is kept so a persisted size survives a
        // direction flip:
        egui::Panel::end("scroll_test").show(ui, |ui| {
            ui.label(i18n::t(
                "The scroll area below has many labels with interactive tooltips. \
                 The purpose is to test that the tooltips close when you scroll.",
            ))
            .on_hover_text(i18n::t("Try hovering a label below, then scroll!"));
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    for i in 0..1000 {
                        // The `{i}` placeholder is filled by `i18n::fmt`, not by `format!`.
                        #[expect(clippy::literal_string_with_formatting_args)]
                        ui.label(i18n::fmt(
                            i18n::t("This is line {i}"),
                            &[("i", i.to_string().as_str())],
                        ))
                        .on_hover_ui(|ui| {
                            ui.style_mut().interaction.selectable_labels = true;
                            ui.label(i18n::t(
                                "This tooltip is interactive, because the text in it is selectable.",
                            ));
                        });
                    }
                });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            self.misc_tests(ui);
        });
    }
}

impl Tooltips {
    fn misc_tests(&mut self, ui: &mut egui::Ui) {
        ui.label(i18n::t("All labels in this demo have tooltips."))
            .on_hover_text(i18n::t("Yes, even this one."));

        ui.label(i18n::t("Some widgets have multiple tooltips!"))
            .on_hover_text(i18n::t("The first tooltip."))
            .on_hover_text(i18n::t("The second tooltip."));

        ui.label(i18n::t("Tooltips can contain interactive widgets."))
            .on_hover_ui(|ui| {
                ui.label(i18n::t("This tooltip contains a link:"));
                ui.hyperlink_to("www.egui.rs", "https://www.egui.rs/")
                    .on_hover_text(i18n::t("The tooltip has a tooltip in it!"));
            });

        ui.label(i18n::t("You can put selectable text in tooltips too."))
            .on_hover_ui(|ui| {
                ui.style_mut().interaction.selectable_labels = true;
                ui.label(i18n::t("You can select this text."));
            });

        ui.label(i18n::t("This tooltip shows at the mouse cursor."))
            .on_hover_text_at_pointer(i18n::t("Move me around!!"));

        ui.separator(); // ---------------------------------------------------------

        let tooltip_ui = |ui: &mut egui::Ui| {
            ui.horizontal(|ui| {
                ui.label(i18n::t("This tooltip was created with"));
                ui.code(".on_hover_ui(…)");
            });
        };
        let disabled_tooltip_ui = |ui: &mut egui::Ui| {
            ui.label(i18n::t("A different tooltip when widget is disabled."));
            ui.horizontal(|ui| {
                ui.label(i18n::t("This tooltip was created with"));
                ui.code(".on_disabled_hover_ui(…)");
            });
        };

        ui.label(i18n::t(
            "You can have different tooltips depending on whether or not a widget is enabled:",
        ))
        .on_hover_text(i18n::t(
            "Check the tooltip of the button below, and see how it changes depending on whether or not it is enabled.",
        ));

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.enabled, i18n::t("Enabled"))
                .on_hover_text(i18n::t(
                    "Controls whether or not the following button is enabled.",
                ));

            ui.add_enabled(
                self.enabled,
                egui::Button::new(i18n::t("Sometimes clickable")),
            )
            .on_hover_ui(tooltip_ui)
            .on_disabled_hover_ui(disabled_tooltip_ui);
        });
    }
}
