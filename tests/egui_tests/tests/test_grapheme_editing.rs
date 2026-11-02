//! Tests that [`TextEdit`] moves the cursor and deletes by grapheme cluster (UAX #29),
//! so a multi-code-point cluster such as an emoji ZWJ sequence is never split.

#![expect(clippy::unwrap_used)] // it's a test

use egui::accesskit::Role;
use egui::text_edit::TextEditState;
use egui::{Id, Key, TextEdit};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

fn id() -> Id {
    Id::unique("grapheme-edit")
}

fn harness(text: &str) -> Harness<'static, String> {
    let mut harness = Harness::new_ui_state(
        move |ui, text: &mut String| {
            let label = ui.label("Text:");
            let output = TextEdit::singleline(text).id(id()).show(ui);
            output.response.response.clone().labelled_by(label.id);
        },
        text.to_owned(),
    );
    harness.run();
    harness.query_by_role(Role::TextInput).unwrap().focus();
    harness.run();
    harness
}

/// Cursor position in char indices, as the widget sees it.
fn cursor(harness: &Harness<'_, String>) -> Option<usize> {
    TextEditState::load(&harness.ctx, id())
        .and_then(|state| state.cursor.char_range())
        .map(|range| range.primary.index.0)
}

fn key(harness: &mut Harness<'_, String>, key: Key) {
    harness.key_press(key);
    harness.run();
}

/// "a" + family emoji (man ZWJ woman ZWJ girl, 5 chars) + "b": three grapheme
/// clusters at char boundaries 0, 1, 6, 7.
const TEXT: &str = "a\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}b";
const BOUNDARIES: [usize; 4] = [0, 1, 6, 7];

#[test]
fn arrow_keys_move_by_grapheme_cluster() {
    let mut harness = harness(TEXT);

    key(&mut harness, Key::End);
    assert_eq!(cursor(&harness), Some(7));

    for &expected in BOUNDARIES.iter().rev().skip(1) {
        key(&mut harness, Key::ArrowLeft);
        assert_eq!(cursor(&harness), Some(expected));
    }

    for &expected in BOUNDARIES.iter().skip(1) {
        key(&mut harness, Key::ArrowRight);
        assert_eq!(cursor(&harness), Some(expected));
    }
}

#[test]
fn backspace_removes_whole_emoji_cluster() {
    let mut harness = harness(TEXT);
    key(&mut harness, Key::End);

    // Backspace on "b" removes the single char:
    key(&mut harness, Key::Backspace);
    assert_eq!(
        harness.state(),
        "a\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"
    );
    assert_eq!(cursor(&harness), Some(6));

    // Backspace on the ZWJ family emoji removes the whole cluster:
    key(&mut harness, Key::Backspace);
    assert_eq!(harness.state(), "a");
    assert_eq!(cursor(&harness), Some(1));
}

#[test]
fn delete_removes_whole_cluster_after_cursor() {
    let mut harness = harness(TEXT);
    key(&mut harness, Key::Home);
    key(&mut harness, Key::ArrowRight); // cursor at 1, inside no cluster: before the family
    assert_eq!(cursor(&harness), Some(1));

    key(&mut harness, Key::Delete);
    assert_eq!(harness.state(), "ab");
    assert_eq!(cursor(&harness), Some(1));
}
