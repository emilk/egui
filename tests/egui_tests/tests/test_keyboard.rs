//! Tests for how egui tracks and matches keys, using both logical and physical keys.

use egui::accesskit::Role;
use egui::{Event, FullKey, Key, KeyCode, KeyEvent, Modifiers, PhysicalKey};
use egui_kittest::{Harness, kittest::Queryable as _};

fn key(logical: &'static str, physical: KeyCode) -> FullKey {
    FullKey::new(Key::character(logical), Some(PhysicalKey::Code(physical)))
}

/// Records every key event egui sees, after [`egui::InputState`] has processed it.
fn key_event_recorder() -> Harness<'static, Vec<KeyEvent>> {
    Harness::new_ui_state(
        |ui, events: &mut Vec<KeyEvent>| {
            ui.input(|i| {
                events.extend(i.events.iter().filter_map(|event| match event {
                    Event::Key(key_event) => Some(key_event.clone()),
                    _ => None,
                }));
            });
        },
        Vec::new(),
    )
}

#[test]
fn release_with_different_logical_key_releases_the_pressed_key() {
    let mut harness = key_event_recorder();

    harness.key_down(key("a", KeyCode::KeyA));
    harness.run();
    assert!(harness.ctx.input(|i| i.key_down(Key::character("a"))));

    // E.g. on web, pressing `Shift` before releasing `a` reports the release as `"A"`.
    harness.key_up(key("A", KeyCode::KeyA));
    harness.run();

    assert!(harness.ctx.input(|i| i.keys_down.is_empty()));
    let released: Vec<_> = harness
        .state()
        .iter()
        .filter(|event| !event.pressed)
        .map(|event| &event.key)
        .collect();
    assert_eq!(
        released,
        [&key("a", KeyCode::KeyA)],
        "the release should be reported with the key as it was pressed"
    );
}

#[test]
fn repeat_is_detected_by_physical_key() {
    let mut harness = key_event_recorder();

    harness.key_down(key("a", KeyCode::KeyA));
    harness.key_down(key("A", KeyCode::KeyA));
    harness.run();

    let repeats: Vec<_> = harness.state().iter().map(|event| event.repeat).collect();
    assert_eq!(repeats, [false, true]);
}

#[test]
fn keys_with_same_logical_key_are_tracked_separately() {
    let mut harness = key_event_recorder();

    harness.key_down(key("1", KeyCode::Digit1));
    harness.key_down(key("1", KeyCode::Numpad1));
    harness.key_up(key("1", KeyCode::Numpad1));
    harness.run();

    harness.ctx.input(|i| {
        assert!(i.key_down(Key::character("1")));
        assert!(i.key_down(PhysicalKey::Code(KeyCode::Digit1)));
        assert!(!i.key_down(PhysicalKey::Code(KeyCode::Numpad1)));
    });
}

#[test]
fn text_edit_select_all_on_non_latin_layout() {
    let mut harness = Harness::new_ui_state(
        |ui, text: &mut String| {
            ui.add(egui::TextEdit::singleline(text).hint_text("Write something"));
        },
        "hello".to_owned(),
    );
    harness.get_by_role(Role::TextInput).focus();
    harness.run();

    // `Cmd+A` on a Russian layout: the `KeyA` key produces "ф".
    harness.key_press_modifiers(Modifiers::COMMAND, key("ф", KeyCode::KeyA));
    harness.get_by_role(Role::TextInput).type_text("x");
    harness.run();

    assert_eq!(harness.state(), "x");
}
