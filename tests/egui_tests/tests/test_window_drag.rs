//! Tests for [`Window::drag_area`].
//!
//! Covers all three [`WindowDrag`] variants by checking whether the window's
//! area rect moves in response to a drag.

use egui::{Id, Pos2, Sense, Vec2, Window, WindowDrag};
use egui_kittest::Harness;

struct State {
    drag_area: WindowDrag,
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

#[test]
fn title_bar_drag_on_titlebar_moves_window() {
    let mut harness = build(State {
        drag_area: WindowDrag::TitleBar,
    });

    let before = window_rect(&harness);
    // Just inside the title bar:
    let titlebar = Pos2::new(before.center().x, before.top() + 8.0);
    let target = titlebar + Vec2::new(60.0, 40.0);

    drag(&mut harness, titlebar, target);

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
    });
    harness.run();

    let before = window_rect(&harness);
    // In the body area, well below the title bar:
    let body = Pos2::new(before.center().x, before.bottom() - 30.0);
    let target = body + Vec2::new(60.0, -40.0);

    drag(&mut harness, body, target);

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
    });
    harness.run();

    let before = window_rect(&harness);
    let body = Pos2::new(before.center().x, before.bottom() - 30.0);
    let target = body + Vec2::new(60.0, -40.0);

    drag(&mut harness, body, target);

    let after = window_rect(&harness);
    let moved = after.min - before.min;
    assert!(
        20.0 < moved.x && moved.y < -20.0,
        "Anywhere + drag anywhere should move the window (delta = {moved:?})"
    );
}
