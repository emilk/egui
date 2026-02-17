use egui::{Frame, Label, RichText, Sense, UiBuilder, Widget as _};

/// Showcase [`egui::Ui::response`].
#[derive(PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct InteractiveContainerDemo {
    count: usize,
}

impl crate::Demo for InteractiveContainerDemo {
    fn name(&self) -> &'static str {
        "\u{20E3} Interactive Container"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .default_width(250.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for InteractiveContainerDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("This demo showcases how to use ");
            ui.code("Ui::response");
            ui.label(" to create interactive container widgets that may contain other widgets.");
        });

        let response = ui
            .scope_builder(
                UiBuilder::new()
                    .id_salt("interactive_container")
                    .sense(Sense::click()),
                |ui| {
                    let response = ui.response();
                    let visuals = ui.style().interact(&response);
                    let text_color = visuals.text_color();

                    Frame::canvas(ui.style())
                        .fill(visuals.bg_fill.gamma_multiply(0.3))
                        .stroke(visuals.bg_stroke)
                        .inner_margin(ui.spacing().menu_margin)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());

                            ui.add_space(32.0);
                            ui.vertical_centered(|ui| {
                                Label::new(
                                    RichText::new(format!("{}", self.count))
                                        .color(text_color)
                                        .size(32.0),
                                )
                                .selectable(false)
                                .ui(ui);
                            });
                            ui.add_space(32.0);

                            ui.horizontal(|ui| {
                                if ui.button("Reset").clicked() {
                                    self.count = 0;
                                }
                                if ui.button("+ 100").clicked() {
                                    self.count += 100;
                                }
                            });
                        });
                },
            )
            .response;

        if response.clicked() {
            self.count += 1;
        }
    }
}
