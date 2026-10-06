//! Tests for the X11/Wayland PRIMARY selection: select text to copy it, middle-click to paste it.

use egui::accesskit::Role;
use egui::{Event, Modifiers, OutputCommand, PointerButton, Pos2, Rect, Vec2};
use egui_kittest::{Harness, kittest::Queryable as _};

const TEXT: &str = "hello world";

/// Steps at 60Hz: with the default `step_dt` of 0.25s a press turns into a drag
/// before the pointer has moved, which is not how a user selects text.
fn harness(add_widget: fn(&mut egui::Ui, &mut String), text: &str) -> Harness<'static, String> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 100.0))
        .with_step_dt(1.0 / 60.0)
        .with_accessibility_check(false) // the text edits are unlabelled
        .build_ui_state(add_widget, text.to_owned());
    harness.run();
    harness
}

fn label_harness() -> Harness<'static, String> {
    harness(|ui, text| _ = ui.label(text.as_str()), TEXT)
}

fn text_edit_harness(text: &str) -> Harness<'static, String> {
    harness(|ui, text| _ = ui.text_edit_singleline(text), text)
}

/// The text reported as a settled selection in the last pass, if any.
fn reported_selection(harness: &Harness<'_, String>) -> Option<String> {
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            OutputCommand::TextSelectionSettled(text) => Some(text.clone()),
            _ => None,
        })
}

/// Press at `from`, drag to `to`, and release there.
///
/// Not [`Harness::drop_at`], because that also sends a `PointerGone`, which
/// would get a pass of its own and replace the output we want to look at.
fn press_and_release(harness: &mut Harness<'_, String>, from: Pos2, to: Pos2) {
    harness.hover_at(from);
    harness.step();
    harness.drag_at(from);
    harness.step();

    if to != from {
        harness.hover_at(to);
        harness.step();
    }

    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
}

/// The horizontal ends of `rect`, a pixel inside it.
fn ends(rect: Rect) -> (Pos2, Pos2) {
    (
        Pos2::new(rect.left() + 1.0, rect.center().y),
        Pos2::new(rect.right() - 1.0, rect.center().y),
    )
}

/// Middle-click at `pos` with `text` in the PRIMARY selection.
fn middle_click_paste(harness: &mut Harness<'_, String>, pos: Pos2, text: &str) {
    // A real middle-click has the pointer where it clicked.
    harness.hover_at(pos);
    harness.step();
    harness.event(Event::MiddleClickPaste {
        pos,
        text: text.to_owned(),
    });
    harness.run();
}

#[test]
fn drag_selecting_a_label_reports_the_selection() {
    let mut harness = label_harness();
    let (from, to) = ends(harness.get_by_label(TEXT).rect());

    press_and_release(&mut harness, from, to);

    assert_eq!(reported_selection(&harness).as_deref(), Some(TEXT));
}

#[test]
fn drag_selecting_in_a_text_edit_reports_the_selection() {
    let mut harness = text_edit_harness(TEXT);
    let (from, to) = ends(harness.get_by_role(Role::TextInput).rect());

    press_and_release(&mut harness, from, to);

    assert_eq!(reported_selection(&harness).as_deref(), Some(TEXT));
}

/// A password is never copied to the clipboard, and PRIMARY is no different.
#[test]
fn a_password_is_never_reported() {
    let mut harness = harness(
        |ui, text| _ = ui.add(egui::TextEdit::singleline(text).password(true)),
        TEXT,
    );
    let (from, to) = ends(harness.get_by_role(Role::PasswordInput).rect());

    press_and_release(&mut harness, from, to);

    assert_eq!(reported_selection(&harness), None);
}

/// Claiming PRIMARY makes this process its owner, so re-claiming an unchanged
/// selection would steal it back from whatever application the user selected in
/// most recently.
#[test]
fn an_unchanged_selection_is_not_reported_again() {
    let mut harness = label_harness();
    let (from, to) = ends(harness.get_by_label(TEXT).rect());

    press_and_release(&mut harness, from, to);
    assert!(reported_selection(&harness).is_some());

    // Press and release again without moving: the selection is unchanged.
    press_and_release(&mut harness, to, to);
    assert_eq!(reported_selection(&harness), None);
}

/// Middle-click pastes where you clicked, not where the text cursor was.
#[test]
fn middle_click_pastes_at_the_click_position() {
    let mut harness = text_edit_harness("ac");
    let (start, end) = ends(harness.get_by_role(Role::TextInput).rect());

    // Put the text cursor at the end, so the two candidate positions differ.
    press_and_release(&mut harness, end, end);

    middle_click_paste(&mut harness, start, "b");
    assert_eq!(harness.state(), "bac");
}

/// A middle-click somewhere else must not paste into whatever `TextEdit` is on screen.
#[test]
fn a_middle_click_outside_the_widget_pastes_nothing() {
    let mut harness = text_edit_harness("ac");

    middle_click_paste(&mut harness, Pos2::new(280.0, 90.0), "b");
    assert_eq!(harness.state(), "ac");
}

#[test]
fn middle_click_does_not_paste_into_a_non_interactive_or_disabled_text_edit() {
    let paste_into = |add_text_edit: fn(&mut egui::Ui, &mut String)| {
        let mut harness = harness(add_text_edit, "ac");
        let (start, _) = ends(harness.get_by_role(Role::TextInput).rect());
        middle_click_paste(&mut harness, start, "b");
        harness.state().clone()
    };

    assert_eq!(
        paste_into(|ui, text| _ = ui.add(egui::TextEdit::singleline(text))),
        "bac",
        "sanity check: an ordinary TextEdit accepts the paste"
    );
    assert_eq!(
        paste_into(|ui, text| _ = ui.add(egui::TextEdit::singleline(text).interactive(false))),
        "ac",
        "a non-interactive TextEdit must not accept the paste"
    );
    assert_eq!(
        paste_into(|ui, text| _ = ui.add_enabled(false, egui::TextEdit::singleline(text))),
        "ac",
        "a disabled TextEdit must not accept the paste"
    );
}
