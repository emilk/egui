#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

//! A code completion popup over a [`egui::TextEdit`].
//!
//! Type `/` to get a list of matching commands.
//! Use the arrow keys to select one, and Enter or Tab to accept it.
//! Escape closes the popup.
//!
//! The text field keeps keyboard focus the whole time. This takes three ingredients:
//! * Consume the popup keys (arrows, Enter, Tab, Escape) _before_ showing the [`egui::TextEdit`],
//!   so it never sees them (see [`egui::InputState::consume_key`]).
//! * Tell the [`egui::TextEdit`] to keep focus on Tab and Escape using [`egui::TextEdit::event_filter`].
//!   Without this egui would move focus to the next widget on Tab, and drop it on Escape.
//! * Move the text cursor after accepting a completion by storing a new [`egui::text_edit::TextEditState`].

use eframe::egui::{
    self, Button, EventFilter, Id, Key, Modifiers, Popup, PopupKind, RectAlign, RichText,
    ScrollArea, TextEdit, text::CCursor, text::CCursorRange, text_edit::TextEditState,
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
    completion: Completion,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::bottom("input").show(ui, |ui| {
            ui.add_space(4.0);
            self.completion
                .text_edit(ui, Id::new("prompt"), &mut self.text, |text_edit| {
                    text_edit
                        .hint_text("Type / for commands…")
                        .desired_width(f32::INFINITY)
                });
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

        if let Some(submitted) = self.completion.take_submitted()
            && !submitted.trim().is_empty()
        {
            self.history.push(submitted);
        }
    }
}

/// State for the completion popup.
#[derive(Default)]
struct Completion {
    /// Index of the selected item, among the currently matching commands.
    selected: usize,

    /// Was the popup open at the end of the last frame?
    was_open: bool,

    /// The word for which the user dismissed the popup with Escape.
    /// The popup stays hidden until the word changes.
    dismissed_word: Option<String>,

    /// Text submitted by pressing Enter without the popup open.
    submitted: Option<String>,
}

impl Completion {
    fn take_submitted(&mut self) -> Option<String> {
        self.submitted.take()
    }

    /// Show a single-line [`TextEdit`] with a completion popup above it.
    fn text_edit(
        &mut self,
        ui: &mut egui::Ui,
        id: Id,
        text: &mut String,
        customize: impl FnOnce(TextEdit<'_>) -> TextEdit<'_>,
    ) {
        let ctx = ui.ctx().clone();

        // Where is the cursor? We use the state from the last frame, before the `TextEdit` runs.
        let cursor = TextEditState::load(&ctx, id)
            .and_then(|state| state.cursor.char_range())
            .map_or_else(|| text.chars().count(), |range| range.primary.index.0);
        let word = word_before_cursor(text, cursor);
        let matches = self.matches(&word);

        // Handle the popup keys before the `TextEdit` sees them:
        let mut selection_changed = false;
        if self.was_open && !matches.is_empty() {
            let mut accept = false;
            ui.input_mut(|input| {
                if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    self.selected = (self.selected + 1) % matches.len();
                    selection_changed = true;
                }
                if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    self.selected = (self.selected + matches.len() - 1) % matches.len();
                    selection_changed = true;
                }
                if input.consume_key(Modifiers::NONE, Key::Enter)
                    || input.consume_key(Modifiers::NONE, Key::Tab)
                {
                    accept = true;
                }
                if input.consume_key(Modifiers::NONE, Key::Escape) {
                    self.dismissed_word = Some(word.clone());
                }
            });

            if accept && let Some(command) = matches.get(self.selected) {
                self.accept(&ctx, id, text, cursor, &word, command);
            }
        }

        let text_edit = customize(TextEdit::singleline(text).id(id)).event_filter(EventFilter {
            horizontal_arrows: true,
            vertical_arrows: true,
            // Keep focus on Tab and Escape while the popup is open, so they can act on the popup:
            tab: self.was_open,
            escape: self.was_open,
        });
        let output = text_edit.show(ui);
        let response = output.response.response;

        // Enter with no popup open submits the text:
        if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
            self.submitted = Some(core::mem::take(text));
            response.request_focus();
        }

        // Recompute with the up-to-date text and cursor, so the popup reacts to typing without a frame delay:
        let cursor = output
            .cursor_range
            .map_or_else(|| text.chars().count(), |range| range.primary.index.0);
        let word = word_before_cursor(text, cursor);
        let matches = self.matches(&word);
        self.selected = self.selected.min(matches.len().saturating_sub(1));

        let is_open = response.has_focus() && !matches.is_empty();
        self.was_open = is_open;

        let clicked = Popup::from_response(&response)
            .kind(PopupKind::Popup)
            .open(is_open)
            .align(RectAlign::TOP_START)
            .align_alternatives(&[RectAlign::BOTTOM_START])
            .width(response.rect.width())
            .show(|ui| {
                ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| {
                        let mut clicked = None;
                        for (i, command) in matches.iter().enumerate() {
                            let is_selected = i == self.selected;
                            let response = ui.add(
                                Button::selectable(
                                    is_selected,
                                    RichText::new(command.name).monospace(),
                                )
                                .right_text(RichText::new(command.description).weak())
                                .min_size(egui::vec2(ui.available_width(), 0.0)),
                            );
                            if is_selected && selection_changed {
                                response.scroll_to_me(None);
                            }
                            if response.clicked() {
                                clicked = Some(i);
                            }
                        }
                        clicked
                    })
                    .inner
            })
            .and_then(|inner| inner.inner);

        if let Some(i) = clicked
            && let Some(command) = matches.get(i)
        {
            self.accept(&ctx, id, text, cursor, &word, command);
            // Clicking the popup took focus from the text field, so give it back:
            ctx.memory_mut(|mem| mem.request_focus(id));
        }
    }

    fn matches(&self, word: &str) -> Vec<&'static Command> {
        if !word.starts_with('/') || self.dismissed_word.as_deref() == Some(word) {
            return vec![];
        }
        COMMANDS
            .iter()
            .filter(|command| command.name.starts_with(word))
            .collect()
    }

    /// Replace `word` (which ends at `cursor`) with the command name, and move the cursor after it.
    fn accept(
        &mut self,
        ctx: &egui::Context,
        id: Id,
        text: &mut String,
        cursor: usize,
        word: &str,
        command: &Command,
    ) {
        let word_chars = word.chars().count();
        let start = cursor - word_chars;
        let byte_start = char_to_byte(text, start);
        let byte_end = char_to_byte(text, cursor);
        let replacement = format!("{} ", command.name);
        text.replace_range(byte_start..byte_end, &replacement);

        let new_cursor = start + replacement.chars().count();
        let mut state = TextEditState::load(ctx, id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(CCursorRange::one(CCursor::new(new_cursor))));
        state.store(ctx, id);

        self.dismissed_word = None;
        self.selected = 0;
    }
}

/// The whitespace-delimited word that ends at the cursor (a char index).
fn word_before_cursor(text: &str, cursor: usize) -> String {
    text.chars()
        .take(cursor)
        .collect::<Vec<_>>()
        .rsplit(|c| c.is_whitespace())
        .next()
        .unwrap_or_default()
        .iter()
        .collect()
}

fn char_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(byte, _)| byte)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Role;
    use egui_kittest::{Harness, kittest::Queryable as _};

    #[derive(Default)]
    struct State {
        text: String,
        completion: Completion,
    }

    fn harness<'a>() -> Harness<'a, State> {
        let mut harness = Harness::new_ui_state(
            |ui, state: &mut State| {
                state
                    .completion
                    .text_edit(ui, Id::new("prompt"), &mut state.text, |text_edit| {
                        text_edit
                    });
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
        assert!(harness.state().completion.was_open);

        harness.key_press(Key::ArrowDown);
        harness.run();
        assert_eq!(harness.state().completion.selected, 1);
        assert!(text_input_has_focus(&harness));

        harness.key_press(Key::ArrowUp);
        harness.key_press(Key::ArrowUp);
        harness.run();
        assert_eq!(harness.state().completion.selected, 1, "should wrap around");

        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(harness.state().text, "/grill-with-docs ");
        assert_eq!(cursor(&harness), Some("/grill-with-docs ".len()));
        assert!(!harness.state().completion.was_open);
        assert!(text_input_has_focus(&harness));
        assert!(harness.state().completion.submitted.is_none());
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
        assert!(harness.state().completion.was_open);

        harness.key_press(Key::Escape);
        harness.run();
        assert!(!harness.state().completion.was_open);
        assert!(text_input_has_focus(&harness));
        assert_eq!(harness.state().text, "/c");

        // Typing more reopens the popup:
        harness.get_by_role(Role::TextInput).type_text("o");
        harness.run();
        assert!(harness.state().completion.was_open);
    }

    #[test]
    fn enter_without_popup_submits() {
        let mut harness = harness();
        harness.get_by_role(Role::TextInput).type_text("hello");
        harness.run();
        assert!(!harness.state().completion.was_open);

        harness.key_press(Key::Enter);
        harness.run();
        assert_eq!(
            harness.state().completion.submitted.as_deref(),
            Some("hello")
        );
        assert_eq!(harness.state().text, "");
        assert!(text_input_has_focus(&harness));
    }
}
