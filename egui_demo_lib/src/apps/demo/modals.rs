use egui::*;

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ModalOptions {
    id_source: String,
    click_away_dismisses: bool,
    background_color: Color32,
    close_key_opt: Option<Key>,
    is_modal_showing: bool,
    id_error_message_opt: Option<String>,
}

impl Default for ModalOptions {
    fn default() -> Self {
        Self {
            click_away_dismisses: true,
            background_color: Color32::from_rgba_unmultiplied(64, 64, 64, 192),
            close_key_opt: Some(Key::Escape),
            id_source: String::from("demo_modal_options"),
            is_modal_showing: false,
            id_error_message_opt: None,
        }
    }
}

impl super::Demo for ModalOptions {
    fn name(&self) -> &'static str {
        "! Modal Options"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        use super::View as _;

        // Create a window for controlling modal details
        egui::Window::new("demo_modal_options")
            .open(open)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for ModalOptions {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            click_away_dismisses,
            background_color,
            close_key_opt,
            id_source,
            is_modal_showing,
            id_error_message_opt,
        } = self;

        ui.group(|ui| {
            ui.horizontal(|ui| {
                if ui.button("show modal").clicked() {
                    *is_modal_showing = true;
                }
            });
        });

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Id:");
                        ui.text_edit_singleline(id_source)
                            .on_hover_text("Enter a custom id");
                    });
                    if let Some(id_error_message) = id_error_message_opt.as_ref() {
                        *is_modal_showing = false;
                        ui.colored_label(Color32::RED, id_error_message);
                        if ui.button("Relinquish id-based control").clicked() {
                            egui::modal::relinquish_modal(ui.ctx());
                            *id_error_message_opt = None;
                        }
                    }
                    // ui.checkbox(
                    //     click_away_dismisses,
                    //     "click_away_dismisses"
                    // );
                });
            });
        });
        let outer_ctx = ui.ctx();
        if *is_modal_showing {
            let id = Id::new(id_source);
            if let Some(mut response) = egui::modal::show_custom_modal(
                outer_ctx,
                id,
                Some(*background_color),
                |_ui| {
                    // Note that the inner Area needs to be shown with the outer context to appear above the modal interceptor
                    // Also, the area needs to be in the foreground to appear atop the modal's inner ui
                    Area::new("An area for some modal content")
                        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                        .order(Order::Foreground)
                        .show(outer_ctx, |ui| {
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(format!("This modal was created with the egui Id `{:?}`", id) );
                                    // ui.separator();
                                    ui.label("You cannot interact with the items behind the modal, but you can interact with the ui here.");
                                    if let Some(close_key) = close_key_opt.as_ref() {
                                        // ui.separator();
                                        ui.label(format!("This modal can be closed using the `{:?}` key", close_key) );
                                        if ui.ctx().input().key_pressed(*close_key) {
                                            *is_modal_showing = false;
                                        }
                                    }
                                    if ui.button("hide modal").clicked() {
                                        *is_modal_showing = false;
                                    }
                                    if ui.button("hide modal and relinquish id-based control").clicked() {
                                        egui::modal::relinquish_modal(ui.ctx());
                                        *is_modal_showing = false;
                                    }
                                    egui::widgets::color_picker::color_edit_button_srgba(
                                        ui,
                                        background_color, color_picker::Alpha::BlendOrAdditive
                                    );
                                }).response
                            }).inner
                        }).inner
                },
            ) {
                response = response.interact(Sense::click());
                if response.has_focus() && response.clicked_elsewhere() && *click_away_dismisses {
                    println!("clicked away {:#?}", response);
                    *is_modal_showing = false;
                }
                response.request_focus();
                *id_error_message_opt = None;
            } else {
                *id_error_message_opt = Some(
                    "relinquish_modal must be called to show a modal with a new id ".to_string(),
                );
            }
        }
        ui.separator();

        ui.horizontal(|ui| {
            egui::reset_button(ui, self);
            ui.add(crate::__egui_github_link_file!());
        });
    }
}
