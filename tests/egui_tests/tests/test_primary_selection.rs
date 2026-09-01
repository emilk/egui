//! Tests for the X11/Wayland PRIMARY selection: the text that is pasted into
//! other applications with the middle mouse button.

use egui::accesskit::Role;
use egui::{Event, Modifiers, OutputCommand, PointerButton, Pos2, Vec2};
use egui_kittest::{Harness, kittest::Queryable as _};

const TEXT: &str = "hello world";

/// A harness with a single selectable label.
///
/// `report` sets [`egui::Options::report_text_selection`], whose default
/// depends on the platform, so every test states what it needs.
///
/// Steps at 60Hz: with the default `step_dt` of 0.25s a press turns into a drag
/// before the pointer has moved, which is not how a user selects text.
fn label_harness(report: bool) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 100.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui(|ui| {
            ui.label(TEXT);
        });
    harness
        .ctx
        .options_mut(|options| options.report_text_selection = report);
    harness.run();
    harness
}

/// The text reported as a settled selection in the last pass, if any.
fn reported_selection<S>(harness: &Harness<'_, S>) -> Option<String> {
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
/// This is [`Harness::drag_at`] plus a release, rather than
/// [`Harness::drop_at`], because `drop_at` also sends a `PointerGone`. Every
/// queued event gets a pass of its own, so that extra event would replace the
/// output of the pass we want to look at.
fn press_and_release<S>(harness: &mut Harness<'_, S>, from: Pos2, to: Pos2) {
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
fn label_ends<S>(harness: &Harness<'_, S>) -> (Pos2, Pos2) {
    let rect = harness.get_by_label(TEXT).rect();
    (
        Pos2::new(rect.left() + 1.0, rect.center().y),
        Pos2::new(rect.right() - 1.0, rect.center().y),
    )
}

#[test]
fn drag_selecting_a_label_reports_the_selection() {
    let mut harness = label_harness(true);
    let (from, to) = label_ends(&harness);

    press_and_release(&mut harness, from, to);

    assert_eq!(
        reported_selection(&harness).as_deref(),
        Some(TEXT),
        "releasing after a drag-selection should report the text"
    );
}

#[test]
fn drag_selecting_in_a_text_edit_reports_the_selection() {
    let mut text = String::from(TEXT);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 100.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui(move |ui| {
            ui.text_edit_singleline(&mut text);
        });
    harness
        .ctx
        .options_mut(|options| options.report_text_selection = true);
    harness.run();

    let rect = harness.get_by_role(Role::TextInput).rect();
    let from = Pos2::new(rect.left() + 1.0, rect.center().y);
    let to = Pos2::new(rect.right() - 1.0, rect.center().y);

    press_and_release(&mut harness, from, to);

    assert_eq!(
        reported_selection(&harness).as_deref(),
        Some(TEXT),
        "a drag-selection in a TextEdit should be reported too"
    );
}

/// A `TextEdit` holding `text`, on a platform with a PRIMARY selection.
fn text_edit_harness(text: &str) -> Harness<'static, String> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 100.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            |ui, text: &mut String| {
                ui.text_edit_singleline(text);
            },
            text.to_owned(),
        );
    harness
        .ctx
        .options_mut(|options| options.report_text_selection = true);
    harness.run();
    harness
}

/// Middle-click pastes where you clicked, not where the text cursor was. That
/// is the X11 convention, and the reason the event carries a position.
#[test]
fn middle_click_pastes_at_the_click_position() {
    let mut harness = text_edit_harness("ac");
    let rect = harness.get_by_role(Role::TextInput).rect();
    let start = Pos2::new(rect.left() + 1.0, rect.center().y);
    let end = Pos2::new(rect.right() - 1.0, rect.center().y);

    // Put the text cursor at the end, so the two candidate positions differ.
    press_and_release(&mut harness, end, end);

    // A real middle-click has the pointer where it clicked.
    harness.hover_at(start);
    harness.step();
    harness.event(egui::Event::MiddleClickPaste {
        pos: start,
        text: "b".to_owned(),
    });
    harness.run();

    assert_eq!(
        harness.state().as_str(),
        "bac",
        "the paste should land where the middle-click was, not at the text cursor"
    );
}

/// The click has to land in the widget: a middle-click somewhere else must not
/// dump the selection into whatever `TextEdit` happens to be on screen.
#[test]
fn a_middle_click_outside_the_widget_pastes_nothing() {
    let mut harness = text_edit_harness("ac");

    let outside = Pos2::new(280.0, 90.0);
    harness.hover_at(outside);
    harness.step();
    harness.event(egui::Event::MiddleClickPaste {
        pos: outside,
        text: "b".to_owned(),
    });
    harness.run();

    assert_eq!(harness.state().as_str(), "ac");
}

/// A password is never copied to the clipboard, and PRIMARY is no different.
#[test]
fn a_password_is_never_reported() {
    let mut text = String::from(TEXT);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 100.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui(move |ui| {
            ui.add(egui::TextEdit::singleline(&mut text).password(true));
        });
    harness
        .ctx
        .options_mut(|options| options.report_text_selection = true);
    harness.run();

    let rect = harness.get_by_role(Role::PasswordInput).rect();
    let from = Pos2::new(rect.left() + 1.0, rect.center().y);
    let to = Pos2::new(rect.right() - 1.0, rect.center().y);

    press_and_release(&mut harness, from, to);

    assert_eq!(reported_selection(&harness), None);
}

/// Claiming PRIMARY makes this process its owner, so re-claiming an unchanged
/// selection would steal it back from whatever application the user selected in
/// most recently. Only a selection that actually changed may be published.
#[test]
fn an_unchanged_selection_is_not_reported_again() {
    let mut harness = label_harness(true);
    let (from, to) = label_ends(&harness);

    press_and_release(&mut harness, from, to);
    assert!(reported_selection(&harness).is_some());

    // Press and release again without moving: the selection is unchanged.
    press_and_release(&mut harness, to, to);

    assert_eq!(
        reported_selection(&harness),
        None,
        "a release that did not change the selection must not be reported again"
    );
}

/// Assembling the selected text costs an allocation, so egui stays quiet when
/// no integration has asked for it.
#[test]
fn nothing_is_reported_when_the_option_is_off() {
    let mut harness = label_harness(false);
    let (from, to) = label_ends(&harness);

    press_and_release(&mut harness, from, to);

    assert_eq!(reported_selection(&harness), None);
}

/// The default follows the platform: on X11 and Wayland the integration feeds
/// the report to the PRIMARY selection, elsewhere nobody listens.
#[test]
fn the_default_follows_the_platform() {
    assert_eq!(
        egui::Options::default().report_text_selection,
        cfg!(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))
    );
}
