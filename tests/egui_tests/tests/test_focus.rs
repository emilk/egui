//! Tests for [`egui::Response::lost_focus`] and [`egui::Response::gained_focus`].
//!
//! Each of them must be `true` for exactly one frame per focus change,
//! no matter how the focus changed.

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::{Harness, kittest::Queryable as _};

/// Enough frames for a late or repeated focus signal to show up.
const FRAMES: usize = 4;

#[derive(Default)]
struct State {
    first: String,
    second: String,

    /// The number of frames in which each text edit reported a focus change.
    first_gained: usize,
    first_lost: usize,
    second_gained: usize,
    second_lost: usize,
}

/// Two text edits with a label between them, which can be clicked to remove the focus.
fn harness<'a>() -> Harness<'a, State> {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut State| {
            let label = ui.label("First");
            let first = ui
                .text_edit_singleline(&mut state.first)
                .labelled_by(label.id);
            state.first_gained += usize::from(first.gained_focus());
            state.first_lost += usize::from(first.lost_focus());

            ui.label("Somewhere else");

            let label = ui.label("Second");
            let second = ui
                .text_edit_singleline(&mut state.second)
                .labelled_by(label.id);
            state.second_gained += usize::from(second.gained_focus());
            state.second_lost += usize::from(second.lost_focus());
        },
        State::default(),
    );
    harness.run_steps(FRAMES);
    harness
}

fn click_text_edit(harness: &mut Harness<'_, State>, label: &str) {
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .click();
    harness.run_steps(FRAMES);
}

fn press(harness: &mut Harness<'_, State>, key: Key) {
    harness.key_press(key);
    harness.run_steps(FRAMES);
}

/// <https://github.com/emilk/egui/issues/8679>
#[test]
fn lost_focus_once_when_clicking_elsewhere() {
    let mut harness = harness();
    click_text_edit(&mut harness, "First");
    assert_eq!(harness.state().first_gained, 1);
    assert_eq!(harness.state().first_lost, 0);

    harness.get_by_label("Somewhere else").click();
    harness.run_steps(FRAMES);
    assert_eq!(harness.state().first_lost, 1);
}

#[test]
fn lost_focus_once_on_enter() {
    let mut harness = harness();
    click_text_edit(&mut harness, "First");
    press(&mut harness, Key::Enter);
    assert_eq!(harness.state().first_lost, 1);
}

#[test]
fn lost_focus_once_on_escape() {
    let mut harness = harness();
    click_text_edit(&mut harness, "First");
    press(&mut harness, Key::Escape);
    assert_eq!(harness.state().first_lost, 1);
}

#[test]
fn lost_focus_once_on_tab() {
    let mut harness = harness();
    click_text_edit(&mut harness, "First");
    press(&mut harness, Key::Tab);
    assert_eq!(harness.state().first_lost, 1);
    assert_eq!(harness.state().second_gained, 1);
}

/// <https://github.com/emilk/egui/issues/2142>
#[test]
fn lost_focus_once_when_a_later_widget_takes_focus() {
    let mut harness = harness();
    click_text_edit(&mut harness, "First");
    click_text_edit(&mut harness, "Second");
    assert_eq!(harness.state().first_lost, 1);
    assert_eq!(harness.state().second_gained, 1);
}

#[test]
fn lost_focus_once_when_an_earlier_widget_takes_focus() {
    let mut harness = harness();
    click_text_edit(&mut harness, "Second");
    click_text_edit(&mut harness, "First");
    assert_eq!(harness.state().second_lost, 1);
    assert_eq!(harness.state().first_gained, 1);
}
