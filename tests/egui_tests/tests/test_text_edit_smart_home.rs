//! Tests for smart `Home` and smart `End` in [`TextEdit`].

#![expect(clippy::unwrap_used)] // it's a test

use egui::os::OperatingSystem;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{Id, Key, Modifiers, TextEdit, Vec2};
use egui_kittest::Harness;

fn id() -> Id {
    Id::unique("smart_home")
}

fn harness(
    make: fn(&mut String) -> TextEdit<'_>,
    text: &str,
    cursor: usize,
) -> Harness<'static, String> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 400.0))
        .with_accessibility_check(false) // the text edit is unlabelled
        .build_ui_state(
            move |ui, text| {
                ui.add(make(text).id(id()).cursor_at_end(false));
            },
            text.to_owned(),
        );
    harness.run();
    harness.ctx.memory_mut(|mem| mem.request_focus(id()));
    harness.run();

    let mut state = TextEditState::load(&harness.ctx, id()).unwrap();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(cursor))));
    state.store(&harness.ctx, id());
    harness.run();
    assert_eq!(
        primary(&harness),
        cursor,
        "the cursor should have been placed"
    );

    harness
}

fn plain(text: &mut String) -> TextEdit<'_> {
    TextEdit::multiline(text)
}

fn code_editor(text: &mut String) -> TextEdit<'_> {
    TextEdit::multiline(text).code_editor()
}

fn smart_end(text: &mut String) -> TextEdit<'_> {
    TextEdit::multiline(text).smart_end(true)
}

fn cursor(harness: &Harness<'_, String>) -> (usize, usize) {
    let range = TextEditState::load(&harness.ctx, id())
        .and_then(|state| state.cursor.char_range())
        .unwrap();
    (range.primary.index.0, range.secondary.index.0)
}

fn primary(harness: &Harness<'_, String>) -> usize {
    cursor(harness).0
}

fn press(harness: &mut Harness<'_, String>, key: Key) {
    harness.key_press(key);
    harness.run();
}

fn press_modifiers(harness: &mut Harness<'_, String>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

// ---- Smart `Home` key ----

#[test]
fn home_should_toggle_between_first_non_blank_and_line_start() {
    let mut harness = harness(code_editor, "    indented", 12);

    press(&mut harness, Key::Home);
    assert_eq!(cursor(&harness), (4, 4));

    press(&mut harness, Key::Home);
    assert_eq!(cursor(&harness), (0, 0));

    press(&mut harness, Key::Home);
    assert_eq!(cursor(&harness), (4, 4));
}

#[test]
fn home_inside_the_indentation_should_go_to_the_first_non_blank() {
    let mut harness = harness(code_editor, "    indented", 2);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 4);
}

#[test]
fn home_should_work_on_a_later_line() {
    let mut harness = harness(code_editor, "a\n  b c\nd", 7);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 4);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 2);
}

#[test]
fn home_should_skip_tabs() {
    let mut harness = harness(code_editor, "\t\tx", 3);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 2);
}

#[test]
fn home_should_skip_non_breaking_spaces() {
    let mut harness = harness(code_editor, "\u{a0}\u{a0}x", 3);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 2);
}

#[test]
fn home_on_a_blank_line_should_go_to_the_line_start() {
    let mut harness = harness(code_editor, "a\n      \nb", 8);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 2);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 2);
}

#[test]
fn home_without_indentation_should_be_a_plain_home() {
    let mut harness = harness(code_editor, "plain", 3);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 0);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 0);
}

#[test]
fn shift_home_should_extend_the_selection() {
    let mut harness = harness(code_editor, "    indented", 12);

    press_modifiers(&mut harness, Modifiers::SHIFT, Key::Home);
    assert_eq!(cursor(&harness), (4, 12));

    press_modifiers(&mut harness, Modifiers::SHIFT, Key::Home);
    assert_eq!(cursor(&harness), (0, 12));
}

#[test]
fn ctrl_home_should_still_go_to_the_start_of_the_text() {
    let mut harness = harness(code_editor, "  a\n  b", 7);

    press_modifiers(&mut harness, Modifiers::CTRL, Key::Home);
    assert_eq!(primary(&harness), 0);
}

#[test]
fn cmd_left_should_be_a_smart_home_on_mac() {
    let mut harness = harness(code_editor, "    indented", 12);

    press_modifiers(&mut harness, Modifiers::MAC_CMD, Key::ArrowLeft);
    assert_eq!(cursor(&harness), (4, 4));

    press_modifiers(&mut harness, Modifiers::MAC_CMD, Key::ArrowLeft);
    assert_eq!(cursor(&harness), (0, 0));
}

#[test]
fn shift_cmd_left_should_extend_the_selection_on_mac() {
    let mut harness = harness(code_editor, "    indented", 12);

    press_modifiers(
        &mut harness,
        Modifiers::MAC_CMD | Modifiers::SHIFT,
        Key::ArrowLeft,
    );
    assert_eq!(cursor(&harness), (4, 12));
}

#[test]
fn ctrl_a_should_still_go_to_the_row_start_on_mac() {
    let mut harness = harness(code_editor, "    indented", 12);
    harness.ctx.set_os(OperatingSystem::Mac);

    press_modifiers(&mut harness, Modifiers::CTRL, Key::A);
    assert_eq!(primary(&harness), 0);
}

#[test]
fn home_should_be_plain_unless_turned_on() {
    let mut harness = harness(plain, "    indented", 12);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 0);
}

#[test]
fn smart_home_should_work_in_a_singleline_text_edit() {
    let mut harness = harness(
        |text| TextEdit::singleline(text).smart_home(true),
        "  name",
        6,
    );

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 2);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 0);
}

#[test]
fn home_on_a_wrapped_row_should_go_to_the_row_start_first() {
    let text = format!("    {}", "word ".repeat(40));
    let mut harness = harness(code_editor, &text, 0);

    press(&mut harness, Key::ArrowDown);
    let second_row_start = primary(&harness);
    assert!(
        4 < second_row_start && second_row_start < text.len(),
        "the text should have wrapped: cursor at {second_row_start}"
    );
    press(&mut harness, Key::ArrowRight);
    press(&mut harness, Key::ArrowRight);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), second_row_start);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 4);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 0);
}

#[test]
fn home_should_not_stop_inside_an_indentation_that_wraps() {
    let text = format!("{}x", " ".repeat(200));
    let mut harness = harness(code_editor, &text, 0);

    press(&mut harness, Key::ArrowDown);
    let inside_the_indentation = primary(&harness);
    assert!(
        0 < inside_the_indentation && inside_the_indentation < 200,
        "the indentation should have wrapped: cursor at {inside_the_indentation}"
    );

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 200);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 0);

    press(&mut harness, Key::Home);
    assert_eq!(primary(&harness), 200);
}

// ---- Smart `End` key ----

#[test]
fn end_should_toggle_between_last_non_blank_and_line_end() {
    let mut harness = harness(smart_end, "code   \nx", 0);

    press(&mut harness, Key::End);
    assert_eq!(cursor(&harness), (4, 4));

    press(&mut harness, Key::End);
    assert_eq!(cursor(&harness), (7, 7));

    press(&mut harness, Key::End);
    assert_eq!(cursor(&harness), (4, 4));
}

#[test]
fn end_on_a_blank_line_should_go_to_the_line_end() {
    let mut harness = harness(smart_end, "a\n      \nb", 2);

    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), 8);

    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), 8);
}

#[test]
fn end_without_trailing_blanks_should_be_a_plain_end() {
    let mut harness = harness(smart_end, "plain\nx", 0);

    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), 5);

    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), 5);
}

#[test]
fn end_should_skip_non_breaking_spaces() {
    let mut harness = harness(smart_end, "x\u{a0}\u{a0}", 0);

    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), 1);

    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), 3);
}

#[test]
fn shift_end_should_extend_the_selection() {
    let mut harness = harness(smart_end, "code   ", 0);

    press_modifiers(&mut harness, Modifiers::SHIFT, Key::End);
    assert_eq!(cursor(&harness), (4, 0));

    press_modifiers(&mut harness, Modifiers::SHIFT, Key::End);
    assert_eq!(cursor(&harness), (7, 0));
}

#[test]
fn ctrl_end_should_still_go_to_the_end_of_the_text() {
    let mut harness = harness(smart_end, "a  \nb  ", 0);

    press_modifiers(&mut harness, Modifiers::CTRL, Key::End);
    assert_eq!(primary(&harness), 7);
}

#[test]
fn cmd_right_should_be_a_smart_end_on_mac() {
    let mut harness = harness(smart_end, "code   ", 0);

    press_modifiers(&mut harness, Modifiers::MAC_CMD, Key::ArrowRight);
    assert_eq!(primary(&harness), 4);

    press_modifiers(&mut harness, Modifiers::MAC_CMD, Key::ArrowRight);
    assert_eq!(primary(&harness), 7);
}

#[test]
fn code_editor_should_have_a_plain_end() {
    let mut harness = harness(code_editor, "code   ", 0);

    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), 7);
}

#[test]
fn end_on_a_wrapped_row_should_go_to_the_row_end_first() {
    let text = format!("{}   ", "word ".repeat(40).trim_end()); // wraps into several rows
    let last_non_blank_end = text.len() - 3;
    let mut harness = harness(smart_end, &text, 0);

    press(&mut harness, Key::End);
    let first_row_end = primary(&harness);
    assert!(
        0 < first_row_end && first_row_end < last_non_blank_end,
        "the text should have wrapped: cursor at {first_row_end}"
    );

    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), last_non_blank_end);

    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), text.len());
}

#[test]
fn end_should_not_stop_inside_trailing_blanks_that_wrap() {
    // The trailing blanks alone are wider than the text edit.
    let text = format!("word{}", " ".repeat(200));
    let mut harness = harness(smart_end, &text, 0);

    press(&mut harness, Key::ArrowDown);
    let inside_the_blanks = primary(&harness);
    assert!(
        4 < inside_the_blanks && inside_the_blanks < text.len(),
        "the trailing blanks should have wrapped: cursor at {inside_the_blanks}"
    );

    // Not the end of the wrapped row, which is also inside the blanks.
    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), 4);

    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), text.len());

    press(&mut harness, Key::End);
    assert_eq!(primary(&harness), 4);
}
