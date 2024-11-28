use egui::{vec2, Align, ComboBox, Context, Id, Layout, Modal, ProgressBar, Ui, Widget, Window};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Modals {
    user_modal_open: bool,
    save_modal_open: bool,
    save_progress: Option<f32>,

    role: &'static str,
    name: String,
}

impl Default for Modals {
    fn default() -> Self {
        Self {
            user_modal_open: false,
            save_modal_open: false,
            save_progress: None,
            role: Self::ROLES[0],
            name: "John Doe".to_owned(),
        }
    }
}

impl Modals {
    const ROLES: [&'static str; 2] = ["user", "admin"];
}

impl crate::Demo for Modals {
    fn name(&self) -> &'static str {
        "🗖 Modals"
    }

    fn show(&mut self, ctx: &Context, open: &mut bool) {
        use crate::View as _;
        Window::new(self.name())
            .open(open)
            .default_size(vec2(512.0, 512.0))
            .vscroll(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl crate::View for Modals {
    fn ui(&mut self, ui: &mut Ui) {
        let Self {
            user_modal_open,
            save_modal_open,
            save_progress,
            role,
            name,
        } = self;

        if ui.button("Open User Modal").clicked() {
            *user_modal_open = true;
        }

        if ui.button("Open Save Modal").clicked() {
            *save_modal_open = true;
        }

        if *user_modal_open {
            let modal = Modal::new(Id::new("Modal A")).show(ui.ctx(), |ui| {
                ui.set_width(250.0);

                ui.heading("Edit User");

                ui.label("Name:");
                ui.text_edit_singleline(name);

                ComboBox::new("role", "Role")
                    .selected_text(*role)
                    .show_ui(ui, |ui| {
                        for r in Self::ROLES {
                            ui.selectable_value(role, r, r);
                        }
                    });

                ui.separator();

                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui.button("Save").clicked() {
                        *save_modal_open = true;
                    }
                    if ui.button("Cancel").clicked() {
                        *user_modal_open = false;
                    }
                });
            });

            if modal.should_close() {
                *user_modal_open = false;
            }
        }

        if *save_modal_open {
            let modal = Modal::new(Id::new("Modal B")).show(ui.ctx(), |ui| {
                ui.set_width(200.0);
                ui.heading("Save? Are you sure?");

                ui.add_space(32.0);

                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui.button("Yes Please").clicked() {
                        *save_progress = Some(0.0);
                    }

                    if ui.button("No Thanks").clicked() {
                        *save_modal_open = false;
                    }
                });
            });

            if modal.should_close() {
                *save_modal_open = false;
            }
        }

        if let Some(progress) = *save_progress {
            Modal::new(Id::new("Modal C")).show(ui.ctx(), |ui| {
                ui.set_width(70.0);
                ui.heading("Saving...");

                ProgressBar::new(progress).ui(ui);

                if progress >= 1.0 {
                    *save_progress = None;
                    *save_modal_open = false;
                    *user_modal_open = false;
                } else {
                    *save_progress = Some(progress + 0.003);
                    ui.ctx().request_repaint();
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::demo::modals::Modals;
    use crate::Demo;
    use egui::accesskit::Role;
    use egui::Key;
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;

    #[test]
    fn clicking_escape_when_popup_open_should_not_close_modal() {
        let initial_state = Modals {
            user_modal_open: true,
            ..Modals::default()
        };

        let mut harness = Harness::new_state(
            |ctx, modals| {
                modals.show(ctx, &mut true);
            },
            initial_state,
        );

        harness.get_by_role(Role::ComboBox).click();

        harness.run();
        assert!(harness.ctx.memory(|mem| mem.any_popup_open()));
        assert!(harness.state().user_modal_open);

        harness.press_key(Key::Escape);
        harness.run();
        assert!(!harness.ctx.memory(|mem| mem.any_popup_open()));
        assert!(harness.state().user_modal_open);
    }

    #[test]
    fn escape_should_close_top_modal() {
        let initial_state = Modals {
            user_modal_open: true,
            save_modal_open: true,
            ..Modals::default()
        };

        let mut harness = Harness::new_state(
            |ctx, modals| {
                modals.show(ctx, &mut true);
            },
            initial_state,
        );

        assert!(harness.state().user_modal_open);
        assert!(harness.state().save_modal_open);

        harness.press_key(Key::Escape);
        harness.run();

        assert!(harness.state().user_modal_open);
        assert!(!harness.state().save_modal_open);
    }

    #[test]
    fn should_match_snapshot() {
        let initial_state = Modals {
            user_modal_open: true,
            ..Modals::default()
        };

        let mut harness = Harness::new_state(
            |ctx, modals| {
                modals.show(ctx, &mut true);
            },
            initial_state,
        );

        let mut results = Vec::new();

        harness.run();
        results.push(harness.try_wgpu_snapshot("modals_1"));

        harness.get_by_name("Save").click();
        // TODO(lucasmerlin): Remove these extra runs once run checks for repaint requests
        harness.run();
        harness.run();
        harness.run();
        results.push(harness.try_wgpu_snapshot("modals_2"));

        harness.get_by_name("Yes Please").click();
        // TODO(lucasmerlin): Remove these extra runs once run checks for repaint requests
        harness.run();
        harness.run();
        harness.run();
        results.push(harness.try_wgpu_snapshot("modals_3"));

        for result in results {
            result.unwrap();
        }
    }
}
