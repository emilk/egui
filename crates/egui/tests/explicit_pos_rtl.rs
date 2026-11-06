//! Explicitly positioned ([`egui::Window::fixed_pos`] / [`egui::Window::current_pos`])
//! windows under the whole-UI direction flag ([`egui::Style::right_to_left`]):
//! the reading-start edge is held while the window grows, a reading-end resize
//! drag sticks, and without an explicit position the reading-start edge is
//! held for any pivot — with an explicit position the explicit pivot wins.
//!
//! Every RTL case has an LTR twin asserting the unchanged behavior. No
//! snapshot assertions; geometry only.

use egui::{
    Align2, Context, Event, FullOutput, Id, Modifiers, PointerButton, Pos2, RawInput, Rect, Ui,
    Window, pos2, vec2,
};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(2000.0, 1200.0))
}

fn input(time: f64, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(screen()),
        time: Some(time),
        events,
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, time: f64, events: Vec<Event>, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input(time, events), run_ui);
    output.textures_delta.clear();
    output
}

fn pointer_move(pos: Pos2) -> Event {
    Event::PointerMoved(pos)
}

fn pointer_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// Show a window whose content claims `width` and record its rect.
fn window_rect_at_width(
    ctx: &Context,
    id: Id,
    window: impl Fn() -> Window<'static> + Copy,
    time: f64,
    width: f32,
) -> Rect {
    let mut rect = Rect::NOTHING;
    for frame in 0..3 {
        let time = time + frame as f64;
        run(ctx, time, vec![], |ui| {
            let response = window().id(id).show(ui, |ui| {
                ui.set_min_size(vec2(width, 50.0));
            });
            if let Some(response) = response {
                rect = response.response.rect;
            }
        });
    }
    rect
}

/// The settled rects of a window whose content grows from 200 to 500.
fn grown_rects(
    ctx: &Context,
    id: Id,
    window: impl Fn() -> Window<'static> + Copy,
    time: f64,
) -> (Rect, Rect) {
    let narrow = window_rect_at_width(ctx, id, window, time, 200.0);
    let wide = window_rect_at_width(ctx, id, window, time + 3.0, 500.0);
    (narrow, wide)
}

/// The reading-end edge must move by exactly the width delta when the
/// reading-start edge is held.
fn assert_growth_fills_the_width_delta(narrow: Rect, wide: Rect, rtl: bool) {
    let width_delta = wide.width() - narrow.width();
    let edge_delta = if rtl {
        narrow.left() - wide.left()
    } else {
        wide.right() - narrow.right()
    };
    assert!(
        width_delta > 100.0 && (edge_delta - width_delta).abs() < 2.0,
        "the reading-end edge should move by the width delta: narrow {narrow:?} wide {wide:?}"
    );
}

/// A `fixed_pos` window under RTL grows reading-forward (leftward) with its
/// reading-start (right) edge pinned at the position.
#[test]
fn fixed_pos_window_grows_reading_forward_with_the_reading_start_edge_fixed() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let id = Id::unique("explicit-pos-rtl-fixed");
    let p = pos2(1000.0, 100.0);
    let (narrow, wide) = grown_rects(
        &ctx,
        id,
        || Window::new("fixed").title_bar(false).fixed_pos(p),
        0.0,
    );
    assert!(
        (narrow.right() - p.x).abs() < 0.5 && (wide.right() - p.x).abs() < 0.5,
        "the reading-start (right) edge should stay pinned at the position: {narrow:?} {wide:?}"
    );
    assert_growth_fills_the_width_delta(narrow, wide, true);
}

/// Twin: under LTR a `fixed_pos` window keeps its left edge pinned and widens
/// rightward.
#[test]
fn fixed_pos_window_grows_rightward_from_the_left_edge_under_ltr() {
    let ctx = Context::default();
    let id = Id::unique("explicit-pos-rtl-fixed");
    let p = pos2(1000.0, 100.0);
    let (narrow, wide) = grown_rects(
        &ctx,
        id,
        || Window::new("fixed").title_bar(false).fixed_pos(p),
        0.0,
    );
    assert!(
        (narrow.left() - p.x).abs() < 0.5 && (wide.left() - p.x).abs() < 0.5,
        "the left edge should stay pinned at the position: {narrow:?} {wide:?}"
    );
    assert_growth_fills_the_width_delta(narrow, wide, false);
}

/// A `current_pos` window under RTL grows reading-forward (leftward) with its
/// reading-start (right) edge pinned at the position.
#[test]
fn current_pos_window_grows_reading_forward_with_the_reading_start_edge_fixed() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let id = Id::unique("explicit-pos-rtl-current");
    let p = pos2(1000.0, 100.0);
    let (narrow, wide) = grown_rects(
        &ctx,
        id,
        || Window::new("current").title_bar(false).current_pos(p),
        0.0,
    );
    assert!(
        (narrow.right() - p.x).abs() < 0.5 && (wide.right() - p.x).abs() < 0.5,
        "the reading-start (right) edge should stay pinned at the position: {narrow:?} {wide:?}"
    );
    assert_growth_fills_the_width_delta(narrow, wide, true);
}

/// Twin: under LTR a `current_pos` window keeps its left edge pinned and
/// widens rightward.
#[test]
fn current_pos_window_grows_rightward_from_the_left_edge_under_ltr() {
    let ctx = Context::default();
    let id = Id::unique("explicit-pos-rtl-current");
    let p = pos2(1000.0, 100.0);
    let (narrow, wide) = grown_rects(
        &ctx,
        id,
        || Window::new("current").title_bar(false).current_pos(p),
        0.0,
    );
    assert!(
        (narrow.left() - p.x).abs() < 0.5 && (wide.left() - p.x).abs() < 0.5,
        "the left edge should stay pinned at the position: {narrow:?} {wide:?}"
    );
    assert_growth_fills_the_width_delta(narrow, wide, false);
}

/// Show the harness window once and record its rect.
fn drag_frame(ctx: &Context, id: Id, p: Pos2, time: f64, events: Vec<Event>, width: f32) -> Rect {
    let mut rect = Rect::NOTHING;
    run(ctx, time, events, |ui| {
        let response = Window::new("drag").id(id).fixed_pos(p).show(ui, |ui| {
            ui.set_min_size(vec2(width, 50.0));
        });
        if let Some(response) = response {
            rect = response.response.rect;
        }
    });
    rect
}

/// A `fixed_pos` window under RTL keeps a resize drag on its reading-end
/// (left) edge: the resized rect persists across release and idle frames with
/// the reading-start (right) edge fixed.
#[test]
fn fixed_pos_window_keeps_a_reading_end_resize_drag() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let id = Id::unique("explicit-pos-rtl-drag");
    let p = pos2(1000.0, 100.0);
    let width = 200.0;
    let mut before = drag_frame(&ctx, id, p, 0.0, vec![], width);
    for frame in 1..3 {
        before = drag_frame(&ctx, id, p, frame as f64, vec![], width);
    }

    // Drag the reading-end (left) edge leftward by 60 px.
    let grab = pos2(before.left() + 1.0, before.center().y);
    let mut time = 10.0;
    drag_frame(&ctx, id, p, time, vec![pointer_move(grab)], width);
    time += 0.1;
    drag_frame(&ctx, id, p, time, vec![pointer_button(grab, true)], width);
    time += 0.1;
    drag_frame(
        &ctx,
        id,
        p,
        time,
        vec![pointer_move(grab + vec2(-30.0, 0.0))],
        width,
    );
    time += 0.1;
    drag_frame(
        &ctx,
        id,
        p,
        time,
        vec![pointer_move(grab + vec2(-60.0, 0.0))],
        width,
    );
    time += 0.1;
    drag_frame(
        &ctx,
        id,
        p,
        time,
        vec![pointer_button(grab + vec2(-60.0, 0.0), false)],
        width,
    );

    let mut after = Rect::NOTHING;
    for idle in 0..2 {
        time += 0.3;
        after = drag_frame(&ctx, id, p, time, vec![], width);
        if idle == 0 {
            assert!(
                (after.left() - (before.left() - 60.0)).abs() < 2.0,
                "the resized rect should persist across release: before {before:?} after {after:?}"
            );
            assert!(
                (after.right() - p.x).abs() < 0.5,
                "the reading-start (right) edge should stay fixed: {after:?}"
            );
        }
    }
    assert!(
        (after.left() - (before.left() - 60.0)).abs() < 2.0,
        "the resized rect should persist across idle frames: {after:?}"
    );
}

/// Twin: under LTR a `fixed_pos` window's pinned (left) corner is not
/// draggable, and dragging the reading-end (right) edge moves it for real.
#[test]
fn fixed_pos_window_keeps_a_reading_end_resize_drag_under_ltr() {
    let ctx = Context::default();
    let id = Id::unique("explicit-pos-rtl-drag");
    let p = pos2(1000.0, 100.0);
    let width = 200.0;
    let mut before = drag_frame(&ctx, id, p, 0.0, vec![], width);
    for frame in 1..3 {
        before = drag_frame(&ctx, id, p, frame as f64, vec![], width);
    }

    // Drag the reading-end (right) edge rightward by 60 px.
    let grab = pos2(before.right() - 1.0, before.center().y);
    let mut time = 10.0;
    drag_frame(&ctx, id, p, time, vec![pointer_move(grab)], width);
    time += 0.1;
    drag_frame(&ctx, id, p, time, vec![pointer_button(grab, true)], width);
    time += 0.1;
    drag_frame(
        &ctx,
        id,
        p,
        time,
        vec![pointer_move(grab + vec2(30.0, 0.0))],
        width,
    );
    time += 0.1;
    drag_frame(
        &ctx,
        id,
        p,
        time,
        vec![pointer_move(grab + vec2(60.0, 0.0))],
        width,
    );
    time += 0.1;
    drag_frame(
        &ctx,
        id,
        p,
        time,
        vec![pointer_button(grab + vec2(60.0, 0.0), false)],
        width,
    );

    time += 0.3;
    let after = drag_frame(&ctx, id, p, time, vec![], width);
    assert!(
        (after.right() - (before.right() + 60.0)).abs() < 2.0,
        "the reading-end (right) edge should really move: before {before:?} after {after:?}"
    );
    assert!(
        (after.left() - p.x).abs() < 0.5,
        "the pinned (left) corner should stay fixed: {after:?}"
    );
}

/// Without an explicit position the reading-start (right) edge is held for ANY
/// pivot under RTL — an explicit `LEFT_TOP` pivot included — and with an
/// explicit position the explicit pivot wins. The LTR control honors the
/// pivot.
#[test]
fn without_an_explicit_position_the_reading_start_edge_is_held_for_any_pivot() {
    // RTL, explicit LEFT_TOP pivot, no explicit position: the RIGHT
    // (reading-start) edge stays fixed and the left edge moves by the size
    // delta — growth does NOT follow the explicit pivot.
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let id = Id::unique("explicit-pos-rtl-any-pivot");
    let (narrow, wide) = grown_rects(
        &ctx,
        id,
        || {
            Window::new("pivot")
                .title_bar(false)
                .pivot(Align2::LEFT_TOP)
                .default_pos(pos2(1000.0, 100.0))
        },
        0.0,
    );
    assert!(
        (narrow.right() - wide.right()).abs() < 0.5,
        "the reading-start (right) edge should stay fixed for any pivot: {narrow:?} {wide:?}"
    );
    assert_growth_fills_the_width_delta(narrow, wide, true);

    // RTL, explicit LEFT_TOP pivot WITH an explicit position: the explicit
    // pivot wins — the left edge is pinned and the window widens rightward.
    let id = Id::unique("explicit-pos-rtl-any-pivot-pinned");
    let p = pos2(1000.0, 100.0);
    let (narrow, wide) = grown_rects(
        &ctx,
        id,
        || {
            Window::new("pivot")
                .title_bar(false)
                .pivot(Align2::LEFT_TOP)
                .current_pos(p)
        },
        0.0,
    );
    assert!(
        (narrow.left() - p.x).abs() < 0.5 && (wide.left() - p.x).abs() < 0.5,
        "with an explicit position the explicit pivot wins: {narrow:?} {wide:?}"
    );
    assert_growth_fills_the_width_delta(narrow, wide, false);

    // LTR control, explicit LEFT_TOP pivot, no explicit position: the pivot IS
    // honored — the left edge stays fixed and the window widens rightward.
    let ctx = Context::default();
    let id = Id::unique("explicit-pos-rtl-any-pivot-ltr");
    let (narrow, wide) = grown_rects(
        &ctx,
        id,
        || {
            Window::new("pivot")
                .title_bar(false)
                .pivot(Align2::LEFT_TOP)
                .default_pos(pos2(1000.0, 100.0))
        },
        0.0,
    );
    assert!(
        (narrow.left() - wide.left()).abs() < 0.5,
        "LTR: the left edge should stay fixed: {narrow:?} {wide:?}"
    );
    assert_growth_fills_the_width_delta(narrow, wide, false);
}
