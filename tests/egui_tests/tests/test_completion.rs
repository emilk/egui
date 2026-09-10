//! Tests for [`egui::CompletionPopup`], completing slash commands at the start of a prompt.

#![expect(clippy::unwrap_used)] // it's a test

use egui::accesskit::Role;
use egui::text_edit::TextEditState;
use egui::{
    CompletionPopup, CompletionQuery, Id, Key, KeyboardShortcut, Modifiers, Suggestion, TextEdit,
};
use egui_kittest::{Harness, kittest::Queryable as _};

const COMMANDS: &[&str] = &[
    "/clear",
    "/compact",
    "/config",
    "/grill-me",
    "/grill-with-docs",
    "/help",
];

fn id() -> Id {
    Id::unique("prompt")
}

/// Commands are only valid at the start of the prompt.
fn suggest_commands(query: &CompletionQuery<'_>) -> Vec<Suggestion> {
    if !query.is_at_start() || !query.word.starts_with('/') {
        return vec![];
    }
    COMMANDS
        .iter()
        .filter(|command| command.starts_with(query.word))
        .map(|command| Suggestion::new(format!("{command} ")))
        .collect()
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
        TextEdit::multiline(text).return_key(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter))
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
            if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) || enter {
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
        .unwrap()
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
