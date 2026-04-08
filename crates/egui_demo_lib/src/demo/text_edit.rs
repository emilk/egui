use egui::{Align, Align2, AtomExt};

/// Showcase [`egui::TextEdit`].
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextEditDemo {
    pub text: String,
    halign: egui::Align,
    valign: egui::Align,
}

impl Default for TextEditDemo {
    fn default() -> Self {
        Self {
            text: "Edit this text".to_owned(),
            halign: egui::Align::LEFT,
            valign: egui::Align::TOP,
        }
    }
}

impl crate::Demo for TextEditDemo {
    fn name(&self) -> &'static str {
        "🖹 TextEdit"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for TextEditDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        let Self {
            text,
            halign,
            valign,
        } = self;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("Advanced usage of ");
            ui.code("TextEdit");
            ui.label(".");
        });

        ui.horizontal(|ui| {
            ui.label("Horizontal align:");
            ui.selectable_value(halign, egui::Align::LEFT, "Left");
            ui.selectable_value(halign, egui::Align::Center, "Center");
            ui.selectable_value(halign, egui::Align::RIGHT, "Right");
        });
        ui.horizontal(|ui| {
            ui.label("Vertical align:");
            ui.selectable_value(valign, egui::Align::TOP, "Top");
            ui.selectable_value(valign, egui::Align::Center, "Center");
            ui.selectable_value(valign, egui::Align::BOTTOM, "Bottom");
        });

        let output = egui::TextEdit::multiline(text)
            .hint_text("Type something!")
            // Atoms are centered by default, so we need to pass the right align here:
            .prefix("🔎".atom_align(Align2([Align::LEFT, *valign])))
            .suffix("❌".atom_align(Align2([Align::RIGHT, *valign])))
            .horizontal_align(*halign)
            .vertical_align(*valign)
            .show(ui);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("Selected text: ");
            if let Some(text_cursor_range) = output.cursor_range {
                let selected_text = text_cursor_range.slice_str(text);
                ui.code(selected_text);
            }
        });

        let anything_selected = output.cursor_range.is_some_and(|cursor| !cursor.is_empty());

        ui.add_enabled(
            anything_selected,
            egui::Label::new("Press ctrl+Y to toggle the case of selected text (cmd+Y on Mac)"),
        );

        if output.response.has_focus()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y))
            && let Some(text_cursor_range) = output.cursor_range
        {
            use egui::TextBuffer as _;
            let selected_chars = text_cursor_range.as_sorted_char_range();
            let selected_text = text.char_range(selected_chars.clone());
            let upper_case = selected_text.to_uppercase();
            let new_text = if selected_text == upper_case {
                selected_text.to_lowercase()
            } else {
                upper_case
            };
            text.delete_char_range(selected_chars.clone());
            text.insert_text(&new_text, selected_chars.start);
        }

        ui.horizontal(|ui| {
            ui.label("Move cursor to the:");

            if ui.button("start").clicked() {
                let text_edit_id = output.response.id;
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id) {
                    let ccursor = egui::text::CCursor::new(0);
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                    state.store(ui.ctx(), text_edit_id);
                    ui.memory_mut(|mem| mem.request_focus(text_edit_id)); // give focus back to the [`TextEdit`].
                }
            }

            if ui.button("end").clicked() {
                let text_edit_id = output.response.id;
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id) {
                    let ccursor = egui::text::CCursor::new(text.chars().count());
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                    state.store(ui.ctx(), text_edit_id);
                    ui.memory_mut(|mem| mem.request_focus(text_edit_id)); // give focus back to the [`TextEdit`].
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use egui::{CentralPanel, Key, Modifiers, accesskit};
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    #[test]
    pub fn should_type() {
        let text = "Hello, world!".to_owned();
        let mut harness = Harness::new_ui_state(
            move |ui, text| {
                CentralPanel::default().show_inside(ui, |ui| {
                    ui.text_edit_singleline(text);
                });
            },
            text,
        );

        harness.run();

        let text_edit = harness.get_by_role(accesskit::Role::TextInput);
        assert_eq!(text_edit.value().as_deref(), Some("Hello, world!"));
        text_edit.focus();

        harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
        text_edit.type_text("Hi ");

        harness.run();
        harness
            .get_by_role(accesskit::Role::TextInput)
            .type_text("there!");

        harness.run();
        let text_edit = harness.get_by_role(accesskit::Role::TextInput);
        assert_eq!(text_edit.value().as_deref(), Some("Hi there!"));
        assert_eq!(harness.state(), "Hi there!");
    }
}
