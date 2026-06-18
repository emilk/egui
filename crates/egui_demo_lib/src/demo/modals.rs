use egui::{ComboBox, Id, Modal, ProgressBar, Ui, Widget as _, Window};

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

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        use crate::View as _;
        Window::new(self.name())
            .open(open)
            .vscroll(false)
            .resizable(false)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| self.ui(ui));
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

        ui.horizontal(|ui| {
            if ui.button("Open User Modal").clicked() {
                *user_modal_open = true;
            }

            if ui.button("Open Save Modal").clicked() {
                *save_modal_open = true;
            }
        });

        ui.label("Click one of the buttons to open a modal.");
        ui.label("Modals have a backdrop and prevent interaction with the rest of the UI.");
        ui.label(
            "You can show modals on top of each other and close the topmost modal with \
            escape or by clicking outside the modal.",
        );

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

                egui::Sides::new().show(
                    ui,
                    |_ui| {},
                    |ui| {
                        if ui.button("Save").clicked() {
                            *save_modal_open = true;
                        }
                        if ui.button("Cancel").clicked() {
                            // You can call `ui.close()` to close the modal.
                            // (This causes the current modals `should_close` to return true)
                            ui.close();
                        }
                    },
                );
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

                egui::Sides::new().show(
                    ui,
                    |_ui| {},
                    |ui| {
                        if ui.button("Yes Please").clicked() {
                            *save_progress = Some(0.0);
                        }

                        if ui.button("No Thanks").clicked() {
                            ui.close();
                        }
                    },
                );
            });

            if modal.should_close() {
                *save_modal_open = false;
            }
        }

        if let Some(progress) = *save_progress {
            Modal::new(Id::new("Modal C")).show(ui.ctx(), |ui| {
                ui.set_width(70.0);
                ui.heading("Saving…");

                ProgressBar::new(progress).ui(ui);

                if progress >= 1.0 {
                    *save_progress = None;
                    *save_modal_open = false;
                    *user_modal_open = false;
                } else {
                    *save_progress = Some(progress + 0.003);
                    ui.request_repaint();
                }
            });
        }

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::Demo as _;
    use crate::demo::modals::Modals;
    use egui::accesskit::Role;
    use egui::{Key, Popup};
    use egui_kittest::kittest::Queryable as _;
    use egui_kittest::{Harness, SnapshotResults};

    #[test]
    fn clicking_escape_when_popup_open_should_not_close_modal() {
        let initial_state = Modals {
            user_modal_open: true,
            ..Modals::default()
        };

        let mut harness = Harness::new_ui_state(
            |ui, modals| {
                modals.show(ui, &mut true);
            },
            initial_state,
        );

        harness.get_by_role(Role::ComboBox).click();

        // Harness::run would fail because we keep requesting repaints to simulate progress.
        harness.run_ok();
        assert!(Popup::is_any_open(&harness.ctx));
        assert!(harness.state().user_modal_open);

        harness.key_press(Key::Escape);
        harness.run_ok();
        assert!(!Popup::is_any_open(&harness.ctx));
        assert!(harness.state().user_modal_open);
    }

    #[test]
    fn escape_should_close_top_modal() {
        let initial_state = Modals {
            user_modal_open: true,
            save_modal_open: true,
            ..Modals::default()
        };

        let mut harness = Harness::new_ui_state(
            |ui, modals| {
                modals.show(ui, &mut true);
            },
            initial_state,
        );

        assert!(harness.state().user_modal_open);
        assert!(harness.state().save_modal_open);

        harness.key_press(Key::Escape);
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

        let mut harness = Harness::new_ui_state(
            |ui, modals| {
                modals.show(ui, &mut true);
            },
            initial_state,
        );

        let mut results = SnapshotResults::new();

        harness.run();
        results.add(harness.try_snapshot("modals_1"));

        harness.get_by_label("Save").click();
        harness.run_ok();
        results.add(harness.try_snapshot("modals_2"));

        harness.get_by_label("Yes Please").click();
        harness.run_ok();
        results.add(harness.try_snapshot("modals_3"));
    }

    // This tests whether the backdrop actually prevents interaction with lower layers.
    #[test]
    fn backdrop_should_prevent_focusing_lower_area() {
        let initial_state = Modals {
            save_modal_open: true,
            save_progress: Some(0.0),
            ..Modals::default()
        };

        let mut harness = Harness::new_ui_state(
            |ui, modals| {
                modals.show(ui, &mut true);
            },
            initial_state,
        );

        harness.run_ok();

        harness.get_by_label("Yes Please").click();

        harness.run_ok();

        // This snapshots should show the progress bar modal on top of the save modal.
        harness.snapshot("modals_backdrop_should_prevent_focusing_lower_area");
    }
}
