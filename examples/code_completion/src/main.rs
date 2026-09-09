#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! A code completion popup over a [`egui::TextEdit`], using [`egui::CompletionPopup`].
//!
//! Type `/` to get a list of matching commands.
//! Use the arrow keys to select one, and Enter or Tab to accept it.
//! Escape closes the popup.
//! The text field keeps keyboard focus the whole time.

use eframe::egui::{
    self, CompletionPopup, CompletionQuery, Key, RichText, ScrollArea, Suggestion, TextEdit,
};

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
    let output = CompletionPopup::new(ui.make_persistent_id("prompt")).show(
        ui,
        TextEdit::singleline(text)
            .hint_text("Type / for commands…")
            .desired_width(f32::INFINITY),
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

/// Commands are only valid at the start of the prompt.
fn suggest_commands(query: &CompletionQuery<'_>) -> Vec<Suggestion> {
    if !query.is_at_start() || !query.word.starts_with('/') {
        return vec![];
    }
    COMMANDS
        .iter()
        .filter(|command| command.name.starts_with(query.word))
        .map(|command| {
            Suggestion::new(format!("{} ", command.name))
                .content(RichText::new(command.name).monospace())
                .description(command.description)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Role;
    use egui::text_edit::TextEditState;
    use egui::{Id, KeyboardShortcut, Modifiers};
    use egui_kittest::{Harness, kittest::Queryable as _};

    fn id() -> Id {
        Id::new("prompt")
    }

    #[derive(Default)]
    struct State {
        text: String,
        submitted: Option<String>,
        popup_open: bool,
    }

    fn harness<'a>() -> Harness<'a, State> {
        harness_with(|text| TextEdit::singleline(text))
    }

    /// A chat composer: multiline, Shift+Enter for newline, Enter to send.
    fn chat_harness<'a>() -> Harness<'a, State> {
        harness_with(|text| {
            TextEdit::multiline(text)
                .return_key(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter))
        })
    }

    fn harness_with<'a>(
        make_text_edit: impl for<'t> Fn(&'t mut String) -> TextEdit<'t> + 'a,
    ) -> Harness<'a, State> {
        let mut harness = Harness::new_ui_state(
            move |ui, state: &mut State| {
                let output = CompletionPopup::new(id()).show(
                    ui,
                    make_text_edit(&mut state.text),
                    suggest_commands,
                );
                state.popup_open = output.is_open;
                let response = &output.text_edit.response.response;
                let enter = response.has_focus()
                    && ui.input_mut(|input| {
                        !input.modifiers.shift && input.consume_key(Modifiers::NONE, Key::Enter)
                    });
                if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) || enter
                {
                    response.request_focus();
                    state.submitted = Some(core::mem::take(&mut state.text));
                }
            },
            State::default(),
        );
        harness.run();
        text_input(&harness).focus();
        harness.run();
        harness
    }

    fn text_input<'h>(harness: &'h Harness<'_, State>) -> egui_kittest::Node<'h> {
        harness
            .query_by_role(Role::TextInput)
            .or_else(|| harness.query_by_role(Role::MultilineTextInput))
            .expect("no text input")
    }

    fn text_input_has_focus(harness: &Harness<'_, State>) -> bool {
        text_input(harness).is_focused()
    }

    fn cursor(harness: &Harness<'_, State>) -> Option<usize> {
        TextEditState::load(&harness.ctx, id())
            .and_then(|state| state.cursor.char_range())
            .map(|range| range.primary.index.0)
    }

    #[test]
    fn arrows_and_enter_accept_completion() {
        let mut harness = harness();
        text_input(&harness).type_text("/gr");
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
        text_input(&harness).type_text("/he");
        harness.run();
        harness.key_press(Key::Tab);
        harness.run();
        assert_eq!(harness.state().text, "/help ");
        assert!(text_input_has_focus(&harness));
    }

    #[test]
    fn escape_closes_popup_and_keeps_focus() {
        let mut harness = harness();
        text_input(&harness).type_text("/c");
        harness.run();
        assert!(harness.state().popup_open);

        harness.key_press(Key::Escape);
        harness.run();
        assert!(!harness.state().popup_open);
        assert!(text_input_has_focus(&harness));
        assert_eq!(harness.state().text, "/c");

        // Typing more reopens the popup:
        text_input(&harness).type_text("o");
        harness.run();
        assert!(harness.state().popup_open);
    }

    #[test]
    fn selection_resets_when_word_changes() {
        let mut harness = harness();
        text_input(&harness).type_text("/c");
        harness.run();
        harness.key_press(Key::ArrowDown); // selects /compact
        harness.run();

        text_input(&harness).type_text("o"); // /compact, /config
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().text, "/compact ");
    }

    #[test]
    fn typing_and_accepting_in_the_same_frame() {
        let mut harness = harness();
        text_input(&harness).type_text("/gr");
        harness.run();
        text_input(&harness).type_text("i");
        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().text, "/grill-me ");
        assert_eq!(cursor(&harness), Some("/grill-me ".len()));
    }

    #[test]
    fn commands_only_complete_at_start_of_prompt() {
        let mut harness = harness();
        text_input(&harness).type_text("fix /he");
        harness.run();
        assert!(!harness.state().popup_open);
    }

    #[test]
    fn multiline_chat_composer() {
        let mut harness = chat_harness();
        text_input(&harness).type_text("/he");
        harness.run();
        assert!(harness.state().popup_open);

        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().text, "/help ", "Enter accepts, no newline");
        assert!(text_input_has_focus(&harness));
        assert!(harness.state().submitted.is_none());

        harness.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
        harness.run();
        assert_eq!(harness.state().text, "/help \n");

        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().submitted.as_deref(), Some("/help \n"));
        assert!(text_input_has_focus(&harness));
    }

    #[test]
    fn enter_without_popup_submits() {
        let mut harness = harness();
        text_input(&harness).type_text("hello");
        harness.run();
        assert!(!harness.state().popup_open);

        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().submitted.as_deref(), Some("hello"));
        assert_eq!(harness.state().text, "");
        assert!(text_input_has_focus(&harness));
    }
}
