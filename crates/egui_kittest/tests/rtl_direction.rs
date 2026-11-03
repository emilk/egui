//! Widget-chrome regression tests for the whole-UI direction flag
//! ([`egui::Style::right_to_left`]), driven through the headless
//! [`egui_kittest::Harness`] with accessibility queries only
//! (no snapshot assertions; renderers are platform-dependent).
//!
//! Every mirrored case asserts its LTR counterpart in the same test, so this file
//! doubles as the LTR-unchanged regression.

use egui::{Event, Modifiers, PointerButton, Pos2, Rect, vec2};
use egui_kittest::Harness;
use kittest::Queryable as _;

fn rtl_harness<State>(rtl: bool) -> egui_kittest::HarnessBuilder<State> {
    Harness::builder()
        .with_size(vec2(400.0, 300.0))
        // Short frames, so that press-hold-release is still a click
        // (and a second click within the double-click delay):
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(move |ctx| ctx.set_right_to_left(rtl))
}

/// Run two more frames: the window follows its content with a frame delay, so the
/// accessibility tree needs one extra frame past an animation to be fully settled.
fn settle(harness: &mut Harness<'_, impl Sized>) {
    harness.run();
    harness.run();
}

/// Press, hold for a few frames, then release, like a real user would.
fn press_and_release(harness: &mut Harness<'_, impl Sized>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    harness.run();
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(2);
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

fn window_app(ui: &mut egui::Ui, open: &mut bool) {
    egui::Window::new("Test window")
        .open(open)
        .show(ui.ctx(), |ui| {
            ui.label("Body");
        });
}

/// A5: the window close button sits at the reading end's mirror
/// (top-left under RTL, top-right under LTR), and clicking it closes the window.
#[test]
fn window_close_button_side_follows_direction() {
    for (rtl, want_left) in [(false, false), (true, true)] {
        let mut harness = rtl_harness(rtl).build_ui_state(window_app, true);
        settle(&mut harness);

        let close = harness.get_by_label("Close window").rect();
        let window = harness.get_by_role(egui::accesskit::Role::Window).rect();
        assert_eq!(
            close.center().x < window.center().x,
            want_left,
            "rtl = {rtl}: close button {close:?}, window {window:?}"
        );

        // Clicking the close button still closes the window:
        harness.get_by_label("Close window").click();
        settle(&mut harness);
        assert!(
            harness.query_by_label("Close window").is_none(),
            "rtl = {rtl}: the window should be closed"
        );
    }
}

/// A5 (Round 1 fix #1): the title-bar hit-test carve is mirrored together with the
/// atom order, so the title band between the buttons stays interactive in both
/// directions: double-clicking it still toggles collapse.
#[test]
fn window_title_bar_stays_interactive() {
    for rtl in [false, true] {
        let mut harness = rtl_harness(rtl).build_ui_state(window_app, true);
        settle(&mut harness);
        assert!(harness.query_by_label("Body").is_some(), "starts open");

        // The title band between the close button and the collapse button:
        let close = harness.get_by_label("Close window").rect();
        let collapse = harness.get_by_label("Hide").rect();
        let band = Rect::from_two_pos(close.center(), collapse.center()).center();

        // Double-click on the band toggles collapse:
        press_and_release(&mut harness, band);
        press_and_release(&mut harness, band);
        settle(&mut harness);
        assert!(
            harness.query_by_label("Body").is_none(),
            "rtl = {rtl}: double-clicking the title band should collapse the window \
             (close {close:?}, collapse {collapse:?})"
        );

        // The collapse button itself still reopens the body:
        harness.get_by_label("Show").click();
        settle(&mut harness);
        assert!(
            harness.query_by_label("Body").is_some(),
            "rtl = {rtl}: the collapse button should reopen the window"
        );
    }
}

/// A5/A7 (Round 1 fix #6b): at the window's default and minimum size, the first body
/// widget does not overlap the title bar under RTL (nor under LTR).
#[test]
fn window_content_does_not_overlap_title_at_default_size() {
    for rtl in [false, true] {
        for min_width in [0.0, 60.0] {
            struct Captured {
                open: bool,
                body: Rect,
            }

            let mut harness = rtl_harness(rtl).build_ui_state(
                |ui, captured: &mut Captured| {
                    let mut window = egui::Window::new("Test window").open(&mut captured.open);
                    if min_width > 0.0 {
                        window = window.min_width(min_width);
                    }
                    window.show(ui.ctx(), |ui| {
                        captured.body = ui.label("Body").rect;
                    });
                },
                Captured {
                    open: true,
                    body: Rect::NOTHING,
                },
            );

            settle(&mut harness);

            let close = harness.get_by_label("Close window").rect();
            let body = harness.state().body;
            assert!(
                body.top() >= close.bottom() - 1.0,
                "rtl = {rtl}, min_width = {min_width}: the first body widget \
                 ({body:?}) must sit below the title-bar buttons ({close:?})"
            );
            assert!(
                !close.intersects(body),
                "rtl = {rtl}, min_width = {min_width}: the title bar must not overlap the body"
            );
        }
    }
}

/// A6: a submenu opens on the reading-end side under RTL (its first item sits to the
/// LEFT of the submenu button) and on the right under LTR. The buttons are centred so
/// that both reading-end sides have room on screen.
#[test]
fn submenu_opens_reading_end_first() {
    for (rtl, want_left) in [(false, false), (true, true)] {
        let mut harness = rtl_harness(rtl).build_ui(|ui| {
            ui.vertical_centered(|ui| {
                ui.menu_button("Top", |ui| {
                    ui.menu_button("Sub", |ui| {
                        _ = ui.button("Item in submenu");
                    });
                });
            });
        });
        settle(&mut harness);

        harness.get_by_label("Top").click();
        settle(&mut harness);
        harness.get_by_label("Sub").click();
        settle(&mut harness);

        let item = harness.get_by_label("Item in submenu").rect();
        let button = harness.get_by_label("Sub").rect();
        assert_eq!(
            item.right() <= button.left() + 1.0,
            want_left,
            "rtl = {rtl}: submenu item {item:?} vs button {button:?}"
        );
    }
}

/// A3: with the window collapse triangle mirrored, the body still opens and closes
/// under RTL (via the "Hide"/"Show" accessibility button).
#[test]
fn collapse_triangle_toggle_opens_and_closes_under_rtl() {
    for rtl in [false, true] {
        let mut harness = rtl_harness(rtl).build_ui_state(window_app, true);
        settle(&mut harness);
        assert!(harness.query_by_label("Body").is_some(), "starts open");

        harness.get_by_label("Hide").click();
        settle(&mut harness);
        assert!(
            harness.query_by_label("Body").is_none(),
            "rtl = {rtl}: clicking the collapse triangle should hide the body"
        );
        assert!(
            harness.query_by_label("Show").is_some(),
            "rtl = {rtl}: the collapsed window should advertise a 'Show' button"
        );

        harness.get_by_label("Show").click();
        settle(&mut harness);
        assert!(
            harness.query_by_label("Body").is_some(),
            "rtl = {rtl}: clicking the collapse triangle again should show the body"
        );
    }
}
