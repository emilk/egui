use egui::{Button, util::undoer::Undoer};

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct State {
    pub toggle_value: bool,
    pub text: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            toggle_value: Default::default(),
            text: "Text with undo/redo".to_owned(),
        }
    }
}

/// Showcase [`egui::util::undoer::Undoer`]
#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct UndoRedoDemo {
    pub state: State,
    pub undoer: Undoer<State>,
}

impl crate::Demo for UndoRedoDemo {
    fn name(&self) -> &'static str {
        "⟲ Undo Redo"
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

impl crate::View for UndoRedoDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.checkbox(&mut self.state.toggle_value, "Checkbox with undo/redo");
        ui.text_edit_singleline(&mut self.state.text);

        ui.separator();

        let can_undo = self.undoer.has_undo(&self.state);
        let can_redo = self.undoer.has_redo(&self.state);

        ui.horizontal(|ui| {
            let undo = ui.add_enabled(can_undo, Button::new("⟲ Undo")).clicked();
            let redo = ui.add_enabled(can_redo, Button::new("⟳ Redo")).clicked();

            if undo && let Some(undo_text) = self.undoer.undo(&self.state) {
                self.state = undo_text.clone();
            }
            if redo && let Some(redo_text) = self.undoer.redo(&self.state) {
                self.state = redo_text.clone();
            }
        });

        self.undoer
            .feed_state(ui.input(|input| input.time), &self.state);
    }
}
