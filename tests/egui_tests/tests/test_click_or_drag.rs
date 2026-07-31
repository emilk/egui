//! Tests for how egui decides whether a press on a click-and-drag widget
//! is a click or a drag.

use egui::{Id, Pos2, Rect, Sense, Vec2};
use egui_kittest::Harness;

/// Must match `InputOptions::max_click_dist`.
const MAX_CLICK_DIST: f32 = 6.0;

fn widget_id() -> Id {
    Id::new("click_and_drag")
}

/// A harness with one click-and-drag widget of the given size at the top-left,
/// and a second one filling the rest.
///
/// The second one matters: without something under the pointer to take over the
/// hover, the first widget keeps it even after the pointer leaves.
///
/// Steps at 60Hz. The default `step_dt` of 0.25s would blow past
/// `max_click_duration` within a couple of frames, turning every press into a
/// drag before the distance rules get a chance to matter.
fn harness_with_widget(size: Vec2) -> Harness<'static, ()> {
    Harness::builder()
        .with_step_dt(1.0 / 60.0)
        .with_size(Vec2::new(300.0, 200.0))
        .build_ui(move |ui| {
            let rect = Rect::from_min_size(ui.max_rect().min, size);
            ui.interact(rect, widget_id(), Sense::click_and_drag());
            ui.advance_cursor_after_rect(rect);
            ui.allocate_response(ui.available_size(), Sense::click_and_drag());
        })
}

/// The widget's `(hovered, dragged)` as of the last completed pass.
fn widget_state(harness: &Harness<'_, ()>) -> (bool, bool) {
    harness
        .ctx
        .read_response(widget_id())
        .map(|r| (r.hovered(), r.dragged()))
        .expect("the widget should have been registered")
}

/// The widget's rect as of the last completed pass.
fn widget_rect(harness: &Harness<'_, ()>) -> Rect {
    harness
        .ctx
        .read_response(widget_id())
        .expect("the widget should have been registered")
        .rect
}

/// A press that leaves the widget can no longer become a click, so it counts as a
/// drag right away — without waiting for `max_click_dist`.
///
/// This matters for widgets thinner than `max_click_dist` (panel resize handles,
/// say): waiting would leave them neither hovered nor dragged for a few frames,
/// which shows up as a flickering highlight.
#[test]
fn press_that_leaves_a_thin_widget_becomes_a_drag_immediately() {
    let size = Vec2::new(3.0, 100.0); // thinner than `max_click_dist`
    let mut harness = harness_with_widget(size);
    harness.step();

    let grab = widget_rect(&harness).center();
    harness.hover_at(grab);
    harness.step();
    harness.drag_at(grab);
    harness.step();

    let (hovered, dragged) = widget_state(&harness);
    assert!(hovered && !dragged, "the press starts out undecided");

    // Creep outward in 1px steps, staying well inside `max_click_dist`.
    let mut saw_drag = false;
    for step in 1..=4 {
        let offset = step as f32;
        assert!(
            offset < MAX_CLICK_DIST,
            "the test must stay inside max_click_dist, or it proves nothing"
        );
        harness.hover_at(Pos2::new(grab.x + offset, grab.y));
        harness.step();

        let (hovered, dragged) = widget_state(&harness);
        saw_drag |= dragged;
        assert!(
            hovered || dragged,
            "at +{offset}px the widget was neither hovered nor dragged, \
             so anything highlighting on `hovered || dragged` would blink out"
        );
    }

    assert!(
        saw_drag,
        "leaving the widget should have started a drag, even within max_click_dist"
    );
}

/// While the pointer is still on the widget, a press stays undecided: hovered,
/// but not yet dragged, so it can still become a click.
#[test]
fn press_inside_a_wide_widget_stays_undecided() {
    let mut harness = harness_with_widget(Vec2::new(100.0, 100.0));
    harness.step();

    let grab = widget_rect(&harness).center();
    harness.hover_at(grab);
    harness.step();
    harness.drag_at(grab);
    harness.step();

    // A 2px twitch: inside the widget, and inside `max_click_dist`.
    harness.hover_at(Pos2::new(grab.x + 2.0, grab.y));
    harness.step();

    let (hovered, dragged) = widget_state(&harness);
    assert!(hovered, "the pointer is still over the widget");
    assert!(
        !dragged,
        "a small twitch inside the widget should still be able to become a click"
    );
}

/// A press and release inside the widget is still a click, not a drag.
#[test]
fn click_inside_a_widget_still_clicks() {
    let mut harness = harness_with_widget(Vec2::new(100.0, 100.0));
    harness.step();

    let grab = widget_rect(&harness).center();
    harness.hover_at(grab);
    harness.step();
    harness.drag_at(grab);
    harness.step();
    // Release without `drop_at`, which would also fire `PointerGone` and so
    // discard the click.
    harness.event(egui::Event::PointerButton {
        pos: grab,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.step();

    assert!(
        harness
            .ctx
            .read_response(widget_id())
            .is_some_and(|r| r.clicked()),
        "press and release without moving should be a click"
    );
}
