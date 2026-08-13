//! Tests for how egui decides whether a press on a click-and-drag widget
//! is a click or a drag.

use egui::{Id, InputOptions, Pos2, Rect, Sense, Style, Vec2};
use egui_kittest::Harness;

/// How far the pointer may move before a press is decidedly a drag.
fn max_click_dist() -> f32 {
    InputOptions::default().max_click_dist
}

/// How far outside its rect a widget can still be hit.
fn interact_radius() -> f32 {
    Style::default().interaction.interact_radius
}

fn widget_id() -> Id {
    Id::new("click_and_drag")
}

/// A harness with one click-and-drag widget of the given size at the top-left.
///
/// If `with_background`, a second click-and-drag widget covers the whole area
/// _beneath_ it. That one matters: without something under the pointer to take
/// over the hover, the first widget keeps it even after the pointer leaves.
///
/// Steps at 60Hz. The default `step_dt` of 0.25s would blow past
/// `max_click_duration` within a couple of frames, turning every press into a
/// drag before the distance rules get a chance to matter.
fn harness_with_widget(size: Vec2, with_background: bool) -> Harness<'static, ()> {
    Harness::builder()
        .with_step_dt(1.0 / 60.0)
        .with_size(Vec2::new(300.0, 200.0))
        .build_ui(move |ui| {
            if with_background {
                // Allocated first, so it ends up _behind_ the widget under test.
                ui.interact(
                    ui.max_rect(),
                    Id::new("background"),
                    Sense::click_and_drag(),
                );
            }
            let rect = Rect::from_min_size(ui.max_rect().min, size);
            ui.interact(rect, widget_id(), Sense::click_and_drag());
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

/// Press the primary button at `pos`, without releasing it.
fn press_at(harness: &mut Harness<'_, ()>, pos: Pos2) {
    harness.hover_at(pos);
    harness.step();
    harness.drag_at(pos);
    harness.step();
}

/// Once a release can no longer land on the widget, the press can no longer become
/// a click, so it counts as a drag right away — without waiting for `max_click_dist`.
///
/// This matters for widgets thinner than `max_click_dist` (panel resize handles,
/// say): waiting would leave them neither hovered nor dragged for a few frames,
/// which shows up as a flickering highlight.
#[test]
fn press_that_leaves_a_thin_widget_becomes_a_drag_immediately() {
    let width = max_click_dist() / 2.0; // thinner than `max_click_dist`
    let mut harness = harness_with_widget(Vec2::new(width, 100.0), true);
    harness.step();

    let grab = widget_rect(&harness).center();
    press_at(&mut harness, grab);

    let (hovered, dragged) = widget_state(&harness);
    assert!(hovered && !dragged, "the press starts out undecided");

    // Creep outward in 1px steps, never reaching `max_click_dist` —
    // if we did, `is_decidedly_dragging` would explain the drag on its own
    // and the test would prove nothing.
    let mut saw_drag = false;
    for step in 1..max_click_dist().ceil() as i32 {
        let offset = step as f32;
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
    let mut harness = harness_with_widget(Vec2::new(100.0, 100.0), true);
    harness.step();

    let grab = widget_rect(&harness).center();
    press_at(&mut harness, grab);

    // A small twitch: inside the widget, and inside `max_click_dist`.
    harness.hover_at(Pos2::new(grab.x + max_click_dist() / 2.0, grab.y));
    harness.step();

    let (hovered, dragged) = widget_state(&harness);
    assert!(hovered, "the pointer is still over the widget");
    assert!(
        !dragged,
        "a small twitch inside the widget should still be able to become a click"
    );
}

/// A press just _outside_ the widget still hits it, thanks to `interact_radius`.
/// The pointer hasn't moved at all, so this must not count as leaving the widget.
#[test]
fn press_just_outside_a_widget_stays_undecided() {
    // No background: we want the widget to win the hit-test from a distance.
    let mut harness = harness_with_widget(Vec2::new(100.0, 100.0), false);
    harness.step();

    let rect = widget_rect(&harness);
    let offset = interact_radius() - 1.0;
    let grab = Pos2::new(rect.right() + offset, rect.center().y);
    press_at(&mut harness, grab);

    let (hovered, dragged) = widget_state(&harness);
    assert!(
        hovered,
        "a press within interact_radius still hits the widget"
    );
    assert!(
        !dragged,
        "the pointer never moved, so this press must still be able to become a click"
    );
}

/// A press and release inside the widget is still a click, not a drag.
#[test]
fn click_inside_a_widget_still_clicks() {
    let mut harness = harness_with_widget(Vec2::new(100.0, 100.0), true);
    harness.step();

    let grab = widget_rect(&harness).center();
    press_at(&mut harness, grab);
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

/// A button inside a draggable row takes the click hit, because it is on top.
/// The pointer is still inside the row though, so the press must stay undecided —
/// otherwise the row starts dragging the moment the user touches the button.
#[test]
fn press_on_a_button_inside_a_draggable_row_stays_undecided() {
    let button_id = Id::new("button");
    let button_size = Vec2::new(50.0, 20.0);

    let mut harness = Harness::builder()
        .with_step_dt(1.0 / 60.0)
        .with_size(Vec2::new(300.0, 200.0))
        .build_ui(move |ui| {
            let row_rect = ui.max_rect();
            ui.interact(row_rect, widget_id(), Sense::click_and_drag());

            // Allocated after the row, so it ends up _on top_ of it.
            let button_rect = Rect::from_min_size(row_rect.min, button_size);
            ui.interact(button_rect, button_id, Sense::click());
        });
    harness.step();

    let grab = Rect::from_min_size(widget_rect(&harness).min, button_size).center();
    press_at(&mut harness, grab);

    let (_hovered, dragged) = widget_state(&harness);
    assert!(
        !dragged,
        "pressing a button inside the row must not start dragging the row"
    );

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
            .read_response(button_id)
            .is_some_and(|r| r.clicked()),
        "the button should have been clicked"
    );
}
