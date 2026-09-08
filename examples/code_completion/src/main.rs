#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

//! A code completion popup over a [`egui::TextEdit`], using [`egui::CompletionPopup`].
//!
//! Type `/` to get a list of matching commands.
//! Use the arrow keys to select one, and Enter or Tab to accept it.
//! Escape closes the popup.
//! The text field keeps keyboard focus the whole time.

use eframe::egui::{self, CompletionPopup, Id, Key, RichText, ScrollArea, Suggestion, TextEdit};

/// A command that can be completed.
struct Command {
    name: &'static str,
    description: &'static str,
}

const COMMANDS: &[Command] = &[
    Command {
        name: "/clear",
        description: "Clear the conversation history",
    },
    Command {
        name: "/compact",
        description: "Summarize the conversation to save context",
    },
    Command {
        name: "/config",
        description: "Open the settings",
    },
    Command {
        name: "/grill-me",
        description: "Interview me about my plan",
    },
    Command {
        name: "/grill-with-docs",
        description: "Interview me and update the docs as we go",
    },
    Command {
        name: "/help",
        description: "Show help and available commands",
    },
    Command {
        name: "/model",
        description: "Pick a model",
    },
    Command {
        name: "/review",
        description: "Review the current changes",
    },
];

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Code completion",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

#[derive(Default)]
struct MyApp {
    text: String,
    history: Vec<String>,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::bottom("input").show(ui, |ui| {
            ui.add_space(4.0);
            if let Some(submitted) = prompt(ui, &mut self.text)
                && !submitted.trim().is_empty()
            {
                self.history.push(submitted);
            }
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Code completion");
            ui.label("Type / to open the completion popup. ↑/↓ selects, Enter or Tab accepts, Escape closes.");
            ui.separator();
            ScrollArea::vertical()
                .auto_shrink(false)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.history {
                        ui.label(RichText::new(format!("> {line}")).monospace());
                    }
                });
        });
    }
}

/// A prompt with command completion. Returns the text when the user presses Enter (with no popup open).
fn prompt(ui: &mut egui::Ui, text: &mut String) -> Option<String> {
    let output = CompletionPopup::new(Id::new("prompt")).show(
        ui,
        text,
        |text| {
            TextEdit::singleline(text)
                .hint_text("Type / for commands…")
                .desired_width(f32::INFINITY)
        },
        suggest_commands,
    );

    let response = &output.text_edit.response.response;
    if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
        response.request_focus();
        Some(core::mem::take(text))
    } else {
        None
    }
}

fn suggest_commands(word: &str) -> Vec<Suggestion> {
    if !word.starts_with('/') {
        return vec![];
    }
    COMMANDS
        .iter()
        .filter(|command| command.name.starts_with(word))
        .map(|command| {
            Suggestion::new(format!("{} ", command.name))
                .label(RichText::new(command.name).monospace())
                .description(command.description)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Role;
    use egui::text_edit::TextEditState;
    use egui_kittest::{Harness, kittest::Queryable as _};

    #[derive(Default)]
    struct State {
        text: String,
        submitted: Option<String>,
        popup_open: bool,
    }

    fn harness<'a>() -> Harness<'a, State> {
        let mut harness = Harness::new_ui_state(
            |ui, state: &mut State| {
                let output = CompletionPopup::new(Id::new("prompt")).show(
                    ui,
                    &mut state.text,
                    |text| TextEdit::singleline(text),
                    suggest_commands,
                );
                state.popup_open = output.is_open;
                let response = &output.text_edit.response.response;
                if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                    response.request_focus();
                    state.submitted = Some(core::mem::take(&mut state.text));
                }
            },
            State::default(),
        );
        harness.run();
        harness.get_by_role(Role::TextInput).focus();
        harness.run();
        harness
    }

    fn text_input_has_focus(harness: &Harness<'_, State>) -> bool {
        harness.get_by_role(Role::TextInput).is_focused()
    }

    fn cursor(harness: &Harness<'_, State>) -> Option<usize> {
        TextEditState::load(&harness.ctx, Id::new("prompt"))
            .and_then(|state| state.cursor.char_range())
            .map(|range| range.primary.index.0)
    }

    #[test]
    fn arrows_and_enter_accept_completion() {
        let mut harness = harness();
        harness.get_by_role(Role::TextInput).type_text("/gr");
        harness.run();
        assert!(harness.state().popup_open);

        harness.key_press(Key::ArrowDown);
        harness.run();
        assert!(text_input_has_focus(&harness));

        harness.key_press(Key::ArrowUp);
        harness.key_press(Key::ArrowUp);
        harness.run();

        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(
            harness.state().text,
            "/grill-with-docs ",
            "should wrap around"
        );
        assert_eq!(cursor(&harness), Some("/grill-with-docs ".len()));
        assert!(!harness.state().popup_open);
        assert!(text_input_has_focus(&harness));
        assert!(harness.state().submitted.is_none());
    }

    #[test]
    fn tab_accepts_completion_and_keeps_focus() {
        let mut harness = harness();
        harness.get_by_role(Role::TextInput).type_text("hello /he");
        harness.run();
        harness.key_press(Key::Tab);
        harness.run();
        assert_eq!(harness.state().text, "hello /help ");
        assert!(text_input_has_focus(&harness));
    }

    #[test]
    fn escape_closes_popup_and_keeps_focus() {
        let mut harness = harness();
        harness.get_by_role(Role::TextInput).type_text("/c");
        harness.run();
        assert!(harness.state().popup_open);

        harness.key_press(Key::Escape);
        harness.run();
        assert!(!harness.state().popup_open);
        assert!(text_input_has_focus(&harness));
        assert_eq!(harness.state().text, "/c");

        // Typing more reopens the popup:
        harness.get_by_role(Role::TextInput).type_text("o");
        harness.run();
        assert!(harness.state().popup_open);
    }

    #[test]
    fn enter_without_popup_submits() {
        let mut harness = harness();
        harness.get_by_role(Role::TextInput).type_text("hello");
        harness.run();
        assert!(!harness.state().popup_open);

        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().submitted.as_deref(), Some("hello"));
        assert_eq!(harness.state().text, "");
        assert!(text_input_has_focus(&harness));
    }
}
