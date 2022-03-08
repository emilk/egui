/// Showcase [`TextEdit`].
#[derive(PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextEdit {
    pub text: String,
}

impl Default for TextEdit {
    fn default() -> Self {
        Self {
            text: "Edit this text".to_owned(),
        }
    }
}

impl super::Demo for TextEdit {
    fn name(&self) -> &'static str {
        "🖹 TextEdit"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .show(ctx, |ui| {
                use super::View as _;
                self.ui(ui);
            });
    }
}

impl super::View for TextEdit {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self { text } = self;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("Advanced usage of ");
            ui.code("TextEdit");
            ui.label(".");
        });

        let output = egui::TextEdit::multiline(text)
            .hint_text("Type something!")
            .show(ui);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("Selected text: ");
            if let Some(text_cursor_range) = output.cursor_range {
                use egui::TextBuffer as _;
                let selected_chars = text_cursor_range.as_sorted_char_range();
                let selected_text = text.char_range(selected_chars);
                ui.code(selected_text);
            }
        });

        let anything_selected = output
            .cursor_range
            .map_or(false, |cursor| !cursor.is_empty());

        ui.add_enabled(
            anything_selected,
            egui::Label::new("Press ctrl+T to toggle the case of selected text (cmd+T on Mac)"),
        );
        if ui
            .input_mut()
            .consume_key(egui::Modifiers::COMMAND, egui::Key::T)
        {
            if let Some(text_cursor_range) = output.cursor_range {
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
        }
    }
}
