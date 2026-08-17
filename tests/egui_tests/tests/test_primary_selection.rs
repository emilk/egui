//! Tests for the X11/Wayland PRIMARY selection: the text that is pasted into
//! other applications with the middle mouse button.

use egui::accesskit::Role;
use egui::{Event, Modifiers, OutputCommand, PointerButton, Pos2, Vec2, os::OperatingSystem};
use egui_kittest::{Harness, kittest::Queryable as _};

const TEXT: &str = "hello world";

/// A harness with a single selectable label, pretending to run on `os`.
///
/// Steps at 60Hz: with the default `step_dt` of 0.25s a press turns into a drag
/// before the pointer has moved, which is not how a user selects text.
fn harness_on(os: OperatingSystem) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 100.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui(|ui| {
            ui.label(TEXT);
        });
    harness.ctx.set_os(os);
    harness.run();
    harness
}

/// The text published to PRIMARY in the last pass, if any.
fn published_to_primary(harness: &Harness<'_>) -> Option<String> {
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            OutputCommand::CopyTextToPrimary(text) => Some(text.clone()),
            _ => None,
        })
}

/// Press at `from`, drag to `to`, and release there.
///
/// This is [`Harness::drag_at`] plus a release, rather than
/// [`Harness::drop_at`], because `drop_at` also sends a `PointerGone`. Every
/// queued event gets a pass of its own, so that extra event would replace the
/// output of the pass we want to look at.
fn press_and_release(harness: &mut Harness<'_>, from: Pos2, to: Pos2) {
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

/// The horizontal ends of the label, a pixel inside it.
fn label_ends(harness: &Harness<'_>) -> (Pos2, Pos2) {
    let rect = harness.get_by_label(TEXT).rect();
    (
        Pos2::new(rect.left() + 1.0, rect.center().y),
        Pos2::new(rect.right() - 1.0, rect.center().y),
    )
}

#[test]
fn drag_selecting_a_label_publishes_to_primary() {
    let mut harness = harness_on(OperatingSystem::Nix);
    let (from, to) = label_ends(&harness);

    press_and_release(&mut harness, from, to);

    assert_eq!(
        published_to_primary(&harness).as_deref(),
        Some(TEXT),
        "releasing after a drag-selection should hand the text to PRIMARY"
    );
}

#[test]
fn drag_selecting_in_a_text_edit_publishes_to_primary() {
    let mut text = String::from(TEXT);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 100.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui(move |ui| {
            ui.text_edit_singleline(&mut text);
        });
    harness.ctx.set_os(OperatingSystem::Nix);
    harness.run();

    let rect = harness.get_by_role(Role::TextInput).rect();
    let from = Pos2::new(rect.left() + 1.0, rect.center().y);
    let to = Pos2::new(rect.right() - 1.0, rect.center().y);

    press_and_release(&mut harness, from, to);

    assert_eq!(
        published_to_primary(&harness).as_deref(),
        Some(TEXT),
        "a drag-selection in a TextEdit should hand the text to PRIMARY too"
    );
}

/// A password is never copied to the clipboard, and PRIMARY is no different.
#[test]
fn a_password_is_never_published() {
    let mut text = String::from(TEXT);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 100.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui(move |ui| {
            ui.add(egui::TextEdit::singleline(&mut text).password(true));
        });
    harness.ctx.set_os(OperatingSystem::Nix);
    harness.run();

    let rect = harness.get_by_role(Role::PasswordInput).rect();
    let from = Pos2::new(rect.left() + 1.0, rect.center().y);
    let to = Pos2::new(rect.right() - 1.0, rect.center().y);

    press_and_release(&mut harness, from, to);

    assert_eq!(published_to_primary(&harness), None);
}

/// Claiming PRIMARY makes this process its owner, so re-claiming an unchanged
/// selection would steal it back from whatever application the user selected in
/// most recently. Only a selection that actually changed may be published.
#[test]
fn an_unchanged_selection_is_not_published_again() {
    let mut harness = harness_on(OperatingSystem::Nix);
    let (from, to) = label_ends(&harness);

    press_and_release(&mut harness, from, to);
    assert!(published_to_primary(&harness).is_some());

    // Press and release again without moving: the selection is unchanged.
    press_and_release(&mut harness, to, to);

    assert_eq!(
        published_to_primary(&harness),
        None,
        "a release that did not change the selection must not re-claim PRIMARY"
    );
}

/// PRIMARY only exists on X11 and Wayland. Elsewhere the integration would
/// throw the text away, so egui should not even assemble it.
#[test]
fn nothing_is_published_on_platforms_without_primary() {
    for os in [
        OperatingSystem::Windows,
        OperatingSystem::Mac,
        OperatingSystem::Unknown,
    ] {
        let mut harness = harness_on(os);
        let (from, to) = label_ends(&harness);

        press_and_release(&mut harness, from, to);

        assert_eq!(
            published_to_primary(&harness),
            None,
            "{os:?} has no PRIMARY selection"
        );
    }
}
