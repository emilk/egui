//! Persisted [`egui::Area`] position state across a runtime direction flip
//! ([`egui::Style::right_to_left`]): the stored pivot position is encoded in
//! the previous pivot's space, so a direction flip (the default pivot follows
//! the interface direction) must re-express it — a window keeps its rect, a
//! restored window keeps its left-top, explicit positions keep re-pinning,
//! and ordinary frames never touch anything.
//!
//! Every flip direction has its twin (LTR→RTL and RTL→LTR). No snapshot
//! assertions; geometry only.

use egui::{Context, FullOutput, Id, Pos2, RawInput, Rect, Ui, Window, pos2, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(2000.0, 1200.0))
}

fn input(time: f64) -> RawInput {
    RawInput {
        screen_rect: Some(screen()),
        time: Some(time),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, time: f64, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input(time), run_ui);
    output.textures_delta.clear();
    output
}

/// Show the harness window (content fixed at 200x50 via `set_min_size`) and
/// record its settled rect.
fn settled_window_rect(ctx: &Context, id: Id, window: impl Fn() -> Window<'static> + Copy) -> Rect {
    let mut rect = Rect::NOTHING;
    for frame in 0..6 {
        let time = frame as f64;
        run(ctx, time, |ui| {
            let response = window().id(id).resizable(false).show(ui, |ui| {
                ui.set_min_size(vec2(200.0, 50.0));
            });
            if let Some(response) = response {
                rect = response.response.rect;
            }
        });
    }
    rect
}

fn same_rect(a: Rect, b: Rect) -> bool {
    (a.left() - b.left()).abs() <= 0.5
        && (a.right() - b.right()).abs() <= 0.5
        && (a.top() - b.top()).abs() <= 0.5
        && (a.bottom() - b.bottom()).abs() <= 0.5
}

/// A window keeps its exact rect when the interface direction flips at
/// runtime, in both flip directions.
#[test]
fn window_rect_is_preserved_across_a_runtime_direction_flip() {
    for (start_rtl, flipped_rtl) in [(false, true), (true, false)] {
        let ctx = Context::default();
        ctx.set_right_to_left(start_rtl);
        let id = Id::unique("area-state-flip-window");
        let before = settled_window_rect(&ctx, id, || {
            Window::new("flip win").default_pos(pos2(1000.0, 100.0))
        });
        ctx.set_right_to_left(flipped_rtl);
        let after = settled_window_rect(&ctx, id, || {
            Window::new("flip win").default_pos(pos2(1000.0, 100.0))
        });
        assert!(
            same_rect(before, after),
            "start_rtl={start_rtl}: the window rect should be preserved across the flip: \
             before {before:?} after {after:?}"
        );
    }
}

/// With an explicit position the explicit position wins: a `current_pos`
/// window re-pins its pivot every frame, so the pinned corner moves from the
/// reading-start corner to the far corner across the flip and back.
#[test]
fn explicit_position_wins_over_the_pivot_migration() {
    let ctx = Context::default();
    let id = Id::unique("area-state-flip-current-pos");
    let p = pos2(1000.0, 100.0);

    let rect = settled_window_rect(&ctx, id, || Window::new("current pos").current_pos(p));
    assert!(
        (rect.left() - p.x).abs() < 0.5 && (rect.top() - p.y).abs() < 0.5,
        "LTR: the left-top corner should sit at the explicit position: {rect:?}"
    );

    ctx.set_right_to_left(true);
    let rect = settled_window_rect(&ctx, id, || Window::new("current pos").current_pos(p));
    assert!(
        (rect.right() - p.x).abs() < 0.5 && (rect.top() - p.y).abs() < 0.5,
        "RTL: the reading-start (right-top) corner should sit at the explicit position: {rect:?}"
    );

    ctx.set_right_to_left(false);
    let rect = settled_window_rect(&ctx, id, || Window::new("current pos").current_pos(p));
    assert!(
        (rect.left() - p.x).abs() < 0.5 && (rect.top() - p.y).abs() < 0.5,
        "back to LTR: the left-top corner should sit at the explicit position again: {rect:?}"
    );
}

/// Ordinary frames never re-encode the pivot position: a steady-state window
/// does not drift over many frames.
#[test]
fn ordinary_frames_never_migrate() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let id = Id::unique("area-state-flip-steady");
    let settled = settled_window_rect(&ctx, id, || {
        Window::new("steady").default_pos(pos2(1000.0, 100.0))
    });
    for frame in 6..26 {
        let time = frame as f64;
        run(&ctx, time, |ui| {
            Window::new("steady")
                .id(id)
                .default_pos(pos2(1000.0, 100.0))
                .show(ui, |ui| {
                    ui.set_min_size(vec2(200.0, 50.0));
                });
        });
        let rect = ctx.memory(|memory| memory.area_rect(id)).unwrap();
        assert!(
            same_rect(rect, settled),
            "frame {frame}: the window should not drift: {rect:?} vs {settled:?}"
        );
    }
}

/// An automatically placed window stays on the screen across a flip and keeps
/// its settled geometry afterwards.
#[test]
fn automatic_placement_flip_stays_on_screen() {
    let ctx = Context::default();
    let id = Id::unique("area-state-flip-auto");
    let before = settled_window_rect(&ctx, id, || Window::new("auto win"));
    assert!(
        screen().contains_rect(before),
        "the auto-placed window should be on-screen in LTR: {before:?}"
    );
    ctx.set_right_to_left(true);
    let after = settled_window_rect(&ctx, id, || Window::new("auto win"));
    assert!(
        screen().contains_rect(after),
        "the flipped window should be on-screen in RTL: {after:?}"
    );
    let later = settled_window_rect(&ctx, id, || Window::new("auto win"));
    assert!(
        same_rect(after, later),
        "the flipped window should keep its geometry: {after:?} vs {later:?}"
    );
}

/// A window restored through the persistence path (pivot position and pivot
/// persist, size does not) keeps its left-top across a direction flip recorded
/// in the saved state: the stored point is the old left-top, and the new pivot
/// position is derived from the first known size.
#[cfg(feature = "persistence")]
#[test]
fn persisted_pivot_pos_is_preserved_across_a_flip_without_a_size() {
    let saved = {
        let ctx = Context::default();
        let id = Id::unique("area-state-flip-restored");
        let rect = settled_window_rect(&ctx, id, || {
            Window::new("restored").default_pos(pos2(1000.0, 100.0))
        });
        let memory = ctx.memory(|memory| ron::to_string(memory).unwrap());
        (memory, rect)
    };
    let (saved, old_rect) = saved;

    let ctx = Context::default();
    ctx.memory_mut(|memory| *memory = ron::from_str(&saved).unwrap());
    ctx.set_right_to_left(true);
    let id = Id::unique("area-state-flip-restored");
    let rect = settled_window_rect(&ctx, id, || {
        Window::new("restored").default_size(old_rect.size())
    });
    assert!(
        (rect.left() - old_rect.left()).abs() < 0.5 && (rect.top() - old_rect.top()).abs() < 0.5,
        "the restored window's left-top should land at the stored point: \
         old {old_rect:?} new {rect:?}"
    );
}
