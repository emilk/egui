//! Tests for [`Window::drag_area`] and [`Window::movable`].
//!
//! Each test sets up a window with a particular drag configuration, drags
//! either inside or outside the title bar, and asserts on the area rect's
//! delta. `WindowDrag::OnTouch` is not exercised here since it just resolves
//! to `TitleBar` (no touch screen in headless tests).

use egui::{Id, Pos2, Sense, Vec2, Window, WindowDrag};
use egui_kittest::Harness;

struct State {
    drag_area: WindowDrag,
    movable: bool,
}

fn build(state: State) -> Harness<'static, State> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(500.0, 400.0))
        .with_max_steps(40) // Area requests a repaint every frame while pressed.
        .build_ui_state(
            move |ui, state: &mut State| {
                Window::new("test_win")
                    .id(Id::new("test_win"))
                    .drag_area(state.drag_area)
                    .movable(state.movable)
                    .default_pos([100.0, 80.0])
                    .default_size([180.0, 140.0])
                    .show(ui.ctx(), |ui| {
                        // A passive widget fills the body; it has no drag sense
                        // of its own, so the Area / title-bar widget is what
                        // decides whether a drag moves the window.
                        ui.allocate_response(ui.available_size(), Sense::hover());
                    });
            },
            state,
        );
    // Let the window settle (auto-position / size, then idle).
    harness.run_steps(4);
    harness
}

fn window_rect(harness: &Harness<'_, State>) -> egui::Rect {
    egui::AreaState::load(&harness.ctx, Id::new("test_win"))
        .expect("window area should be persisted after the first frame")
        .rect()
}

/// Drag the pointer from `from` to `to` over multiple frames; release at the end.
fn drag(harness: &mut Harness<'_, State>, from: Pos2, to: Pos2) {
    harness.drag_at(from);
    harness.run_steps(4);
    harness.hover_at(to);
    harness.run_steps(4);
    harness.drop_at(to);
    harness.run_steps(4);
}

fn titlebar_pos(rect: egui::Rect) -> Pos2 {
    // Just inside the title bar:
    Pos2::new(rect.center().x, rect.top() + 8.0)
}

fn body_pos(rect: egui::Rect) -> Pos2 {
    // Well below the title bar:
    Pos2::new(rect.center().x, rect.bottom() - 30.0)
}

#[test]
fn title_bar_drag_on_titlebar_moves_window() {
    let mut harness = build(State {
        drag_area: WindowDrag::TitleBar,
        movable: true,
    });

    let before = window_rect(&harness);
    let from = titlebar_pos(before);
    let to = from + Vec2::new(60.0, 40.0);

    drag(&mut harness, from, to);

    let after = window_rect(&harness);
    let moved = after.min - before.min;
    assert!(
        20.0 < moved.x && 20.0 < moved.y,
        "TitleBar + drag on titlebar should move the window (delta = {moved:?})"
    );
}

#[test]
fn title_bar_drag_outside_titlebar_keeps_window_put() {
    let mut harness = build(State {
        drag_area: WindowDrag::TitleBar,
        movable: true,
    });

    let before = window_rect(&harness);
    let from = body_pos(before);
    let to = from + Vec2::new(60.0, -40.0);

    drag(&mut harness, from, to);

    let after = window_rect(&harness);
    let moved = after.min - before.min;
    assert!(
        moved.length() < 1.0,
        "TitleBar + drag in the body should NOT move the window (delta = {moved:?})"
    );
}

#[test]
fn anywhere_drag_in_body_moves_window() {
    let mut harness = build(State {
        drag_area: WindowDrag::Anywhere,
        movable: true,
    });

    let before = window_rect(&harness);
    let from = body_pos(before);
    let to = from + Vec2::new(60.0, -40.0);

    drag(&mut harness, from, to);

    let after = window_rect(&harness);
    let moved = after.min - before.min;
    assert!(
        20.0 < moved.x && moved.y < -20.0,
        "Anywhere + drag anywhere should move the window (delta = {moved:?})"
    );
}

#[test]
fn movable_false_keeps_window_put_even_on_titlebar() {
    // Regression: a `movable(false)` window used to still move when the user
    // dragged the title bar in `TitleBar` mode.
    let mut harness = build(State {
        drag_area: WindowDrag::TitleBar,
        movable: false,
    });

    let before = window_rect(&harness);
    let from = titlebar_pos(before);
    let to = from + Vec2::new(60.0, 40.0);

    drag(&mut harness, from, to);

    let after = window_rect(&harness);
    let moved = after.min - before.min;
    assert!(
        moved.length() < 1.0,
        "TitleBar + movable(false) should NOT move the window (delta = {moved:?})"
    );
}

#[test]
fn off_keeps_window_put_on_body_drag() {
    // `WindowDrag::Off` should freeze the window regardless of `movable`.
    let mut harness = build(State {
        drag_area: WindowDrag::Off,
        movable: true,
    });

    let before = window_rect(&harness);
    let from = body_pos(before);
    let to = from + Vec2::new(60.0, -40.0);

    drag(&mut harness, from, to);

    let after = window_rect(&harness);
    let moved = after.min - before.min;
    assert!(
        moved.length() < 1.0,
        "Off + drag in the body should NOT move the window (delta = {moved:?})"
    );
}

#[test]
fn off_keeps_window_put_on_titlebar_drag() {
    let mut harness = build(State {
        drag_area: WindowDrag::Off,
        movable: true,
    });

    let before = window_rect(&harness);
    let from = titlebar_pos(before);
    let to = from + Vec2::new(60.0, 40.0);

    drag(&mut harness, from, to);

    let after = window_rect(&harness);
    let moved = after.min - before.min;
    assert!(
        moved.length() < 1.0,
        "Off + drag on titlebar should NOT move the window (delta = {moved:?})"
    );
}
