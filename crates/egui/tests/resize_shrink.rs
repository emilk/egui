//! `Resize` shrink semantics for plain `Resize` containers and resizable
//! [`egui::Window`]s: a negative-dx drag must shrink the container while the
//! reading-start edge stays fixed (the reading-start edge is the left edge in
//! left-to-right interfaces and the right edge in right-to-left ones), the
//! first drag frame must not re-pin a stale pre-drag content size, the
//! configured `min_size` floor must hold the anchored edge, growth and idle
//! grow-to-content must keep working, and a shrink must stop at the content's
//! measured minimum width.
//!
//! Every case runs an LTR and an RTL twin; the LTR twins double as the
//! LTR-unchanged regression. No snapshot assertions; geometry only.

use core::num::NonZeroUsize;

use egui::{
    CentralPanel, Context, Event, FullOutput, Id, Modifiers, PointerButton, Pos2, RawInput, Rect,
    Ui, Vec2, Window, pos2, vec2,
};

fn input_at(size: Vec2, time: f64, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(time),
        events,
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
    output
}

fn pointer_press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn same_rect(a: Rect, b: Rect) -> bool {
    (a.left() - b.left()).abs() <= 0.5
        && (a.right() - b.right()).abs() <= 0.5
        && (a.top() - b.top()).abs() <= 0.5
        && (a.bottom() - b.bottom()).abs() <= 0.5
}

/// The painted `Resize` container rect: the stroke outline is the container rect
/// expanded by 2.0 per side (LTR and RTL alike), so shrink it back.
fn container_rect_from_shapes(output: &FullOutput) -> Rect {
    use egui::epaint::RectShape;
    use egui::epaint::Shape;

    let mut container = Rect::NOTHING;
    for clipped in &output.shapes {
        if let Shape::Rect(RectShape {
            rect, fill, stroke, ..
        }) = &clipped.shape
            && *fill == egui::Color32::TRANSPARENT
            && !stroke.is_empty()
        {
            container = rect.shrink(2.0);
        }
    }
    container
}

/// Body content that fills the offered width in a laid-out frame (like the
/// title bar's grow atoms or a full-width row): this is the content shape whose
/// `last_content_size` equals the current size, the shape that used to pin the
/// grow-only re-pin.
fn width_filling_content(ui: &mut Ui) {
    ui.label("Body");
    ui.separator();
}

/// A reusable window harness: one context with a fixed screen, and frame /
/// settle / edge-drag helpers around a single window.
struct WindowHarness {
    ctx: Context,
    screen: Vec2,
}

impl WindowHarness {
    fn new(rtl: bool, screen: Vec2) -> Self {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        Self { ctx, screen }
    }

    fn frame_rect(
        &self,
        events: Vec<Event>,
        time: f64,
        win: impl Fn() -> Window<'static>,
        content: impl Fn(&mut Ui),
    ) -> Rect {
        let mut rect = Rect::NOTHING;
        run(&self.ctx, input_at(self.screen, time, events), |_| {
            if let Some(response) = win().show(&self.ctx, |ui| content(ui)) {
                rect = response.response.rect;
            }
        });
        rect
    }

    /// Run idle frames until the window stops changing (settling past any
    /// discarded sizing pass and content-driven growth).
    fn settle(
        &self,
        mut time: f64,
        win: impl Fn() -> Window<'static> + Copy,
        content: impl Fn(&mut Ui) + Copy,
    ) -> Rect {
        let mut prev = self.frame_rect(vec![], time, win, content);
        for _ in 0..8 {
            time += 0.1;
            let next = self.frame_rect(vec![], time, win, content);
            if same_rect(next, prev) {
                return next;
            }
            prev = next;
        }
        prev
    }

    /// Press on a window edge (`press_x` picks the x of the press point from
    /// the settled rect, mid-height, away from the corners), move the pointer
    /// by `delta`, release, settle, and return the before/after rects.
    fn drag_edge(
        &self,
        time: f64,
        win: impl Fn() -> Window<'static> + Copy,
        content: impl Fn(&mut Ui) + Copy,
        press_x: impl Fn(Rect) -> f32,
        delta: Vec2,
    ) -> (Rect, Rect) {
        let before = self.settle(time, win, content);
        assert!(
            before.is_positive(),
            "the window should be laid out before the drag (got {before:?})"
        );
        let grab = pos2(press_x(before), before.center().y);
        self.frame_rect(vec![Event::PointerMoved(grab)], time + 1.0, win, content);
        self.frame_rect(vec![pointer_press(grab, true)], time + 1.1, win, content);
        let moved = grab + delta;
        self.frame_rect(vec![Event::PointerMoved(moved)], time + 1.2, win, content);
        self.frame_rect(vec![pointer_press(moved, false)], time + 1.3, win, content);
        let after = self.settle(time + 1.4, win, content);
        (before, after)
    }
}

/// The window builder shared by the window tests: placed so the drag's before
/// and after rects both fit on screen in either direction.
fn shrink_window(pos: Pos2, size: Vec2) -> Window<'static> {
    Window::new("shrink win")
        .resizable(true)
        .default_size(size)
        .default_pos(pos)
}

// ---------------------------------------------------------------------------

/// T1: a `Window` with width-filling content shrinks under a negative-dx drag
/// with the reading-start edge fixed: under LTR the left edge stays put and the
/// right edge moves left; under RTL the right edge stays put and the left edge
/// moves right. This is the case the grow-only `desired_size` re-pin used to
/// block.
#[test]
fn window_shrink_drag_shrinks_with_reading_start_edge_fixed() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);
    const START_SIZE: Vec2 = vec2(400.0, 200.0);
    const DELTA: f32 = 150.0;

    // LTR: press the right (far) edge and drag left; the left (reading-start)
    // edge stays fixed:
    let ltr = WindowHarness::new(false, SCREEN);
    let win = || shrink_window(pos2(100.0, 50.0), START_SIZE);
    let (before, after) = ltr.drag_edge(
        0.0,
        win,
        width_filling_content,
        |r| r.right() - 1.0,
        vec2(-DELTA, 0.0),
    );
    assert!(
        (after.width() - (before.width() - DELTA)).abs() <= 1.0,
        "LTR: the shrink drag must shrink the window by ~{DELTA} \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.left() - before.left()).abs() <= 1.0,
        "LTR: the reading-start (left) edge must stay fixed \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.top() - before.top()).abs() <= 1.0,
        "LTR: the top edge must stay fixed (before {before:?}, after {after:?})"
    );

    // RTL: press the left (far) edge and drag right; the right (reading-start)
    // edge stays fixed:
    let rtl = WindowHarness::new(true, SCREEN);
    let win = || shrink_window(pos2(450.0, 50.0), START_SIZE);
    let (before, after) = rtl.drag_edge(
        0.0,
        win,
        width_filling_content,
        |r| r.left() + 1.0,
        vec2(DELTA, 0.0),
    );
    assert!(
        (after.width() - (before.width() - DELTA)).abs() <= 1.0,
        "RTL: the shrink drag must shrink the window by ~{DELTA} \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.right() - before.right()).abs() <= 1.0,
        "RTL: the reading-start (right) edge must stay fixed \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.top() - before.top()).abs() <= 1.0,
        "RTL: the top edge must stay fixed (before {before:?}, after {after:?})"
    );
}

/// T2: after a shrink drag, idle frames do not grow the window back to the
/// stale pre-drag content size (regression for the sizing-pass stale re-pin:
/// `last_content_size` must never feed a grow-only re-pin with a value bigger
/// than what the user dragged to).
#[test]
fn shrink_survives_idle_frames_without_growing_back() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);
    const START_SIZE: Vec2 = vec2(400.0, 200.0);
    const DELTA: f32 = 150.0;

    // LTR:
    let ltr = WindowHarness::new(false, SCREEN);
    let win = || shrink_window(pos2(100.0, 50.0), START_SIZE);
    let (_, after) = ltr.drag_edge(
        0.0,
        win,
        width_filling_content,
        |r| r.right() - 1.0,
        vec2(-DELTA, 0.0),
    );
    for i in 0..3 {
        let next = ltr.frame_rect(vec![], 10.0 + f64::from(i), win, width_filling_content);
        assert!(
            same_rect(next, after),
            "LTR: idle frames must not grow the shrunk window back \
             (after {after:?}, frame {i} {next:?})"
        );
    }

    // RTL:
    let rtl = WindowHarness::new(true, SCREEN);
    let win = || shrink_window(pos2(450.0, 50.0), START_SIZE);
    let (_, after) = rtl.drag_edge(
        0.0,
        win,
        width_filling_content,
        |r| r.left() + 1.0,
        vec2(DELTA, 0.0),
    );
    for i in 0..3 {
        let next = rtl.frame_rect(vec![], 10.0 + f64::from(i), win, width_filling_content);
        assert!(
            same_rect(next, after),
            "RTL: idle frames must not grow the shrunk window back \
             (after {after:?}, frame {i} {next:?})"
        );
    }
}

/// T3: an over-shrink drag clamps to exactly the configured minimum *width*
/// (+- 1 rounding) with the anchored (reading-start) edge fixed in both
/// directions — the floor is the constant min, never the current size.
/// Deviation from the plan's T3, measured at the base HEAD and kept: a
/// `Window` renders at its content's measured height (its inner `Resize` runs
/// `with_stroke(false)` + `resizable(false)`, so `Resize::end` reports
/// `last_content_size`), so the configured `min_size.y` never binds — the
/// window's height settles at the content height (74 < 150 here) in BOTH
/// directions, before and after this candidate. The width floor binds because
/// the body claims the offered width; it is the floor this test pins. The
/// both-dimension floor on plain `Resize` containers is covered by
/// `resize_rtl.rs` `min_size_floor_identical_in_both_directions` (A5).
#[test]
fn over_shrink_clamps_to_min_size_anchored_edge_fixed() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);
    const START_SIZE: Vec2 = vec2(500.0, 300.0);
    const MIN_SIZE: Vec2 = vec2(200.0, 150.0);

    let win_with_min = |pos: Pos2| {
        Window::new("min floor win")
            .resizable(true)
            .default_size(START_SIZE)
            .min_size(MIN_SIZE)
            .default_pos(pos)
    };

    // LTR: far over-shrink up and to the left:
    let ltr = WindowHarness::new(false, SCREEN);
    let win = || win_with_min(pos2(100.0, 50.0));
    let (before, after) = ltr.drag_edge(
        0.0,
        win,
        width_filling_content,
        |r| r.right() - 1.0,
        vec2(-400.0, -300.0),
    );
    assert!(
        (after.width() - MIN_SIZE.x).abs() <= 1.0,
        "LTR: the over-shrink drag must clamp to the configured min width \
         (before {before:?}, after {after:?}, min {MIN_SIZE:?})"
    );
    assert!(
        after.height() <= MIN_SIZE.y,
        "LTR: the window height is content-bound (see the test doc), so it must \
         settle at or below the min height, never above it \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.left() - before.left()).abs() <= 1.0,
        "LTR: the min floor must hold the reading-start (left) edge fixed \
         (before {before:?}, after {after:?})"
    );

    // RTL: far over-shrink up and to the right:
    let rtl = WindowHarness::new(true, SCREEN);
    let win = || win_with_min(pos2(550.0, 50.0));
    let (before, after) = rtl.drag_edge(
        0.0,
        win,
        width_filling_content,
        |r| r.left() + 1.0,
        vec2(400.0, -300.0),
    );
    assert!(
        (after.width() - MIN_SIZE.x).abs() <= 1.0,
        "RTL: the over-shrink drag must clamp to the configured min width \
         (before {before:?}, after {after:?}, min {MIN_SIZE:?})"
    );
    assert!(
        after.height() <= MIN_SIZE.y,
        "RTL: the window height is content-bound (see the test doc), so it must \
         settle at or below the min height, never above it \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.right() - before.right()).abs() <= 1.0,
        "RTL: the min floor must hold the reading-start (right) edge fixed \
         (before {before:?}, after {after:?})"
    );
}

/// T4: growth still works: dragging a window edge outward grows the window by
/// the drag delta with the reading-start edge fixed, and the grown size is
/// stable across idle frames (the idle grow-to-content re-pin is unchanged).
#[test]
fn growth_drag_still_grows_with_reading_start_edge_fixed() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);
    const START_SIZE: Vec2 = vec2(300.0, 150.0);
    const DELTA: f32 = 120.0;

    // LTR: drag the right edge rightward:
    let ltr = WindowHarness::new(false, SCREEN);
    let win = || shrink_window(pos2(100.0, 50.0), START_SIZE);
    let (before, after) = ltr.drag_edge(
        0.0,
        win,
        width_filling_content,
        |r| r.right() - 1.0,
        vec2(DELTA, 0.0),
    );
    assert!(
        (after.width() - (before.width() + DELTA)).abs() <= 1.0,
        "LTR: the growth drag must grow the window by ~{DELTA} \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.left() - before.left()).abs() <= 1.0,
        "LTR: the reading-start (left) edge must stay fixed \
         (before {before:?}, after {after:?})"
    );

    // RTL: drag the left edge leftward:
    let rtl = WindowHarness::new(true, SCREEN);
    let win = || shrink_window(pos2(450.0, 50.0), START_SIZE);
    let (before, after) = rtl.drag_edge(
        0.0,
        win,
        width_filling_content,
        |r| r.left() + 1.0,
        vec2(-DELTA, 0.0),
    );
    assert!(
        (after.width() - (before.width() + DELTA)).abs() <= 1.0,
        "RTL: the growth drag must grow the window by ~{DELTA} \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.right() - before.right()).abs() <= 1.0,
        "RTL: the reading-start (right) edge must stay fixed \
         (before {before:?}, after {after:?})"
    );
}

/// T5: a shrink stops at the content's measured minimum width: content wider
/// than the window (a long non-wrapping label) pins the shrink at the content
/// size, with the anchored edge fixed. This records the residual
/// content-min pin precisely: the floor is the content minimum measured at the
/// drag start, not the current size of a *smaller* window.
#[test]
fn shrink_stops_at_content_minimum_width_anchored_edge_fixed() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);
    const START_SIZE: Vec2 = vec2(250.0, 120.0);
    // ~500 points wide when not wrapped — far wider than the window.
    const WIDE: &str = "wwwwwwwwww wwwwwwwwww wwwwwwwwww wwwwwwwwww wwwwwwwwww";

    let content = |ui: &mut Ui| {
        ui.add(egui::Label::new(WIDE).wrap_mode(egui::TextWrapMode::Extend));
    };

    // LTR: the window settles at the content width; a large inward drag on the
    // right edge must not shrink below it, and the left edge stays fixed:
    let ltr = WindowHarness::new(false, SCREEN);
    let win = || shrink_window(pos2(100.0, 50.0), START_SIZE);
    let (before, after) = ltr.drag_edge(0.0, win, content, |r| r.right() - 1.0, vec2(-300.0, 0.0));
    assert!(
        (after.width() - before.width()).abs() <= 1.0,
        "LTR: the shrink must stop at the content minimum width \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.left() - before.left()).abs() <= 1.0,
        "LTR: the reading-start (left) edge must stay fixed \
         (before {before:?}, after {after:?})"
    );

    // RTL twin:
    let rtl = WindowHarness::new(true, SCREEN);
    let win = || shrink_window(pos2(450.0, 50.0), START_SIZE);
    let (before, after) = rtl.drag_edge(0.0, win, content, |r| r.left() + 1.0, vec2(300.0, 0.0));
    assert!(
        (after.width() - before.width()).abs() <= 1.0,
        "RTL: the shrink must stop at the content minimum width \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.right() - before.right()).abs() <= 1.0,
        "RTL: the reading-start (right) edge must stay fixed \
         (before {before:?}, after {after:?})"
    );
}

// ---------------------------------------------------------------------------

/// T6: behavioral pin for the corner-grip drag path with `max_passes = 1` (as
/// when the multi-pass budget is consumed by something else in a real frame):
/// with the pointer dragging the corner grip, the first frame after the move
/// already reports the requested shrink, with the reading-start edge fixed, and
/// the shrunk size survives the release.
///
/// This is a behavioral pin, not a causal one: the frame observed here is NOT
/// the sizing pass. The press frame is (the drag begins with `min_content_width`
/// still unset), and by the move frame the minimum is cached and
/// `last_content_size` was freshly refreshed in `end`, so this frame would be
/// green even without the active-resize gate in `Resize::end`. The gate's
/// genuinely observable sizing-pass effect is pinned by the two siblings: T7
/// (`window_edge_drag_requested_shrink_survives_a_final_sizing_pass`) drives
/// the window edge-drag path, where the consumed `requested_size` arrives from
/// the previous frame and the first consuming frame is the sizing pass (there
/// the stale `last_content_size` re-pins through the window render and the idle
/// grow-to-content, not through the gated re-pin, which only runs for containers
/// that show how large they are); T8
/// (`sizing_pass_final_reports_the_requested_shrink_on_a_plain_resize`) drives a
/// plain `Resize` whose sizing pass is the final pass, where the gated re-pin
/// itself is what is observed. A plain `Resize` with a width-filling separator
/// is the smallest reproducible shape.
#[test]
fn first_drag_frame_reports_the_requested_shrink_even_without_a_second_pass() {
    const SCREEN: Vec2 = vec2(400.0, 300.0);
    const START_SIZE: Vec2 = vec2(200.0, 100.0);
    const DELTA: f32 = 60.0;

    for (rtl, delta) in [(false, vec2(-DELTA, 0.0)), (true, vec2(DELTA, 0.0))] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        // One pass per frame: the discarded sizing pass is the visible output,
        // so the re-pin (or its absence) is observable on the first drag frame.
        ctx.options_mut(|o| o.max_passes = NonZeroUsize::MIN);

        let id = Id::unique("first_drag_frame_shrink");

        let container = |events: Vec<Event>, time: f64| {
            let output = run(&ctx, input_at(SCREEN, time, events), |ui| {
                CentralPanel::default().show(ui, |ui| {
                    egui::Resize::default()
                        .id(id)
                        .default_size(START_SIZE)
                        .show(ui, width_filling_content);
                });
            });
            container_rect_from_shapes(&output)
        };

        // Settle at the start size, then grab the grip:
        let mut time = 0.0;
        let mut prev = container(vec![], time);
        for _ in 0..8 {
            time += 0.1;
            let next = container(vec![], time);
            if same_rect(next, prev) {
                break;
            }
            prev = next;
        }
        assert!(
            prev.is_positive(),
            "rtl = {rtl}: the container should be laid out (got {prev:?})"
        );

        // The grip sits at the reading-end bottom corner (bottom-right in LTR,
        // bottom-left in RTL); read its real rect from the corner response.
        let grip_center = ctx
            .read_response(id.with("__resize_corner"))
            .map(|corner| corner.rect.center())
            .filter(|center| prev.contains(*center))
            .unwrap_or_else(|| {
                if rtl {
                    pos2(prev.left() + 8.0, prev.bottom() - 8.0)
                } else {
                    pos2(prev.right() - 8.0, prev.bottom() - 8.0)
                }
            });
        time += 1.0;
        container(vec![Event::PointerMoved(grip_center)], time);
        time += 0.1;
        container(vec![pointer_press(grip_center, true)], time);

        // The first active (dragging) frame: the requested shrink must be
        // reported immediately, not re-pinned to the stale pre-drag size.
        time += 0.1;
        let moved = grip_center + delta;
        let dragging = container(vec![Event::PointerMoved(moved)], time);
        let expected_width = START_SIZE.x - DELTA;
        assert!(
            (dragging.width() - expected_width).abs() <= 1.0,
            "rtl = {rtl}: the first drag frame must report the requested shrink \
             (expected ~{expected_width}, got {dragging:?})"
        );
        if rtl {
            assert!(
                (dragging.right() - prev.right()).abs() <= 1.0,
                "rtl = {rtl}: the reading-start (right) edge must stay fixed on the \
                 first drag frame (settled {prev:?}, dragging {dragging:?})"
            );
        } else {
            assert!(
                (dragging.left() - prev.left()).abs() <= 1.0,
                "rtl = {rtl}: the reading-start (left) edge must stay fixed on the \
                 first drag frame (settled {prev:?}, dragging {dragging:?})"
            );
        }

        // Release: the shrunk size must stick.
        time += 0.1;
        container(vec![pointer_press(moved, false)], time);
        time += 0.1;
        let released = container(vec![], time);
        assert!(
            (released.width() - expected_width).abs() <= 1.0,
            "rtl = {rtl}: the shrunk size must survive the release \
             (dragging {dragging:?}, released {released:?})"
        );
    }
}

// ---------------------------------------------------------------------------

/// T7: the active-resize gate's observable sizing-pass effect, through the
/// window edge-drag path with the sizing pass as the final pass (`max_passes =
/// 1`, as when the multi-pass budget is consumed by something else in a real
/// frame): pressing a window resize corner and moving the pointer inward within
/// the same frame makes the press frame itself store the shrunk
/// `requested_size` (`total_drag_delta` includes the intra-frame move), and the
/// next frame — the release, since no later drag frame rewrites the request —
/// consumes it as the sizing pass, because the drag's content minimum has not
/// been measured yet. On that frame the stale pre-drag `last_content_size` must
/// NOT re-pin the requested shrink back up: after the release, the idle frames
/// must keep the shrunk size instead of bouncing back to the pre-drag size.
/// This test pins the `!actively_resizing` gate in `Resize::end` itself; the
/// corner-grip behavioral twin is T6.
#[test]
fn window_edge_drag_requested_shrink_survives_a_final_sizing_pass() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);
    const START_SIZE: Vec2 = vec2(400.0, 200.0);
    // Small enough that the intra-frame move stays within the corner grab
    // square (corner grab radius 10), so the press frame still starts the
    // window resize drag; large enough that the stale re-pin's bounce back to
    // the pre-drag size exceeds the assertion tolerance.
    const DELTA: f32 = 6.0;

    // LTR: press the far (right) bottom corner and move the pointer inward
    // (leftward) within the same frame, so the press frame stores the shrunk
    // request; release the next frame, so the frame consuming the request is
    // the sizing pass:
    let ltr = WindowHarness::new(false, SCREEN);
    let win = || shrink_window(pos2(100.0, 50.0), START_SIZE);
    let before = ltr.settle(0.0, win, width_filling_content);
    assert!(
        before.is_positive(),
        "LTR: the window should be laid out before the drag (got {before:?})"
    );
    // From here every frame runs a single pass, so the sizing pass that
    // consumes the drag's request is the visible, final pass:
    ltr.ctx.options_mut(|o| o.max_passes = NonZeroUsize::MIN);
    let grab = pos2(before.right() - 2.0, before.bottom() - 2.0);
    let moved = grab + vec2(-DELTA, 0.0);
    ltr.frame_rect(
        vec![Event::PointerMoved(grab)],
        1.0,
        win,
        width_filling_content,
    );
    ltr.frame_rect(
        vec![pointer_press(grab, true), Event::PointerMoved(moved)],
        1.1,
        win,
        width_filling_content,
    );
    ltr.frame_rect(
        vec![pointer_press(moved, false)],
        1.2,
        win,
        width_filling_content,
    );
    let expected_width = before.width() - DELTA;
    for i in 0..3 {
        let idle = ltr.frame_rect(vec![], 2.0 + f64::from(i), win, width_filling_content);
        assert!(
            (idle.width() - expected_width).abs() <= 1.0,
            "LTR: after the release the idle frames must keep the shrunk size, not \
             bounce back to the stale pre-drag size \
             (before {before:?}, idle frame {i} {idle:?})"
        );
        assert!(
            (idle.left() - before.left()).abs() <= 1.0,
            "LTR: the reading-start (left) edge must stay fixed \
             (before {before:?}, idle frame {i} {idle:?})"
        );
        assert!(
            (idle.top() - before.top()).abs() <= 1.0,
            "LTR: the top edge must stay fixed (before {before:?}, idle frame {i} {idle:?})"
        );
    }

    // RTL: press the far (left) bottom corner and move the pointer inward
    // (rightward) within the same frame; the right (reading-start) edge stays
    // fixed. The window is placed with slack on its right so the frame that
    // still renders at the stale pre-drag size (the sizing pass that consumes
    // the request, positioned at the moved left edge) fits on screen — the
    // generic on-screen constrain must not come into play:
    let rtl = WindowHarness::new(true, SCREEN);
    let win = || shrink_window(pos2(250.0, 50.0), START_SIZE);
    let before = rtl.settle(0.0, win, width_filling_content);
    assert!(
        before.is_positive(),
        "RTL: the window should be laid out before the drag (got {before:?})"
    );
    rtl.ctx.options_mut(|o| o.max_passes = NonZeroUsize::MIN);
    let grab = pos2(before.left() + 2.0, before.bottom() - 2.0);
    let moved = grab + vec2(DELTA, 0.0);
    rtl.frame_rect(
        vec![Event::PointerMoved(grab)],
        1.0,
        win,
        width_filling_content,
    );
    rtl.frame_rect(
        vec![pointer_press(grab, true), Event::PointerMoved(moved)],
        1.1,
        win,
        width_filling_content,
    );
    rtl.frame_rect(
        vec![pointer_press(moved, false)],
        1.2,
        win,
        width_filling_content,
    );
    let expected_width = before.width() - DELTA;
    for i in 0..3 {
        let idle = rtl.frame_rect(vec![], 2.0 + f64::from(i), win, width_filling_content);
        assert!(
            (idle.width() - expected_width).abs() <= 1.0,
            "RTL: after the release the idle frames must keep the shrunk size, not \
             bounce back to the stale pre-drag size \
             (before {before:?}, idle frame {i} {idle:?})"
        );
        assert!(
            (idle.right() - before.right()).abs() <= 1.0,
            "RTL: the reading-start (right) edge must stay fixed \
             (before {before:?}, idle frame {i} {idle:?})"
        );
        assert!(
            (idle.top() - before.top()).abs() <= 1.0,
            "RTL: the top edge must stay fixed (before {before:?}, idle frame {i} {idle:?})"
        );
    }
}

// ---------------------------------------------------------------------------

/// T8: the active-resize gate itself, on a plain `Resize` whose sizing pass is
/// the final pass (`max_passes = 1`, as when the multi-pass budget is consumed
/// by something else in a real frame). Pressing the corner grip and moving the
/// pointer within the same frame makes that very frame the sizing pass that
/// consumes the shrunk request (the corner response is kept lag-less for the
/// widget being interacted with, so `begin` already computes the requested
/// size from the moved pointer). On the sizing-pass frame the grow-only
/// `desired_size` re-pin — which runs for containers that show how large they
/// are — must not clamp the just-requested shrink back up to the stale
/// pre-drag content size: the sizing-pass frame must already report the
/// requested shrink, with the reading-start edge fixed, and the shrunk size
/// must survive the release (the corner response keeps carrying the pointer
/// through the release frame, which re-applies the request and re-measures).
/// The corner-grip behavioral twin with a second pass is T6; the window
/// edge-drag twin is T7.
#[test]
fn sizing_pass_final_reports_the_requested_shrink_on_a_plain_resize() {
    const SCREEN: Vec2 = vec2(400.0, 300.0);
    const START_SIZE: Vec2 = vec2(200.0, 100.0);
    // Small enough that the intra-frame move stays within the corner grip's
    // rect, so the press frame still starts the grip drag; large enough that
    // the stale re-pin's effect exceeds the assertion tolerance.
    const DELTA: f32 = 4.0;

    for (rtl, delta) in [(false, vec2(-DELTA, 0.0)), (true, vec2(DELTA, 0.0))] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        // One pass per frame: the discarded sizing pass is the visible output,
        // so the re-pin (or its absence) is observable on the release frame.
        ctx.options_mut(|o| o.max_passes = NonZeroUsize::MIN);

        let id = Id::unique("sizing_pass_final_shrink");

        let container = |events: Vec<Event>, time: f64| {
            let output = run(&ctx, input_at(SCREEN, time, events), |ui| {
                CentralPanel::default().show(ui, |ui| {
                    egui::Resize::default()
                        .id(id)
                        .default_size(START_SIZE)
                        .show(ui, width_filling_content);
                });
            });
            container_rect_from_shapes(&output)
        };

        // Settle at the start size, then grab the grip:
        let mut time = 0.0;
        let mut prev = container(vec![], time);
        for _ in 0..8 {
            time += 0.1;
            let next = container(vec![], time);
            if same_rect(next, prev) {
                break;
            }
            prev = next;
        }
        assert!(
            prev.is_positive(),
            "rtl = {rtl}: the container should be laid out (got {prev:?})"
        );

        // The grip sits at the reading-end bottom corner (bottom-right in LTR,
        // bottom-left in RTL); read its real rect from the corner response.
        let grip_center = ctx
            .read_response(id.with("__resize_corner"))
            .map(|corner| corner.rect.center())
            .filter(|center| prev.contains(*center))
            .unwrap_or_else(|| {
                if rtl {
                    pos2(prev.left() + 8.0, prev.bottom() - 8.0)
                } else {
                    pos2(prev.right() - 8.0, prev.bottom() - 8.0)
                }
            });
        time += 1.0;
        container(vec![Event::PointerMoved(grip_center)], time);
        // Press and move within the same frame: the corner response is kept
        // lag-less for the widget being interacted with, so this frame's
        // `begin` already computes the shrunk request from the moved pointer
        // and runs the sizing pass on it — with `max_passes = 1` that sizing
        // pass is the visible pass, so the re-pin (or its absence) is
        // observable right here:
        time += 0.1;
        let moved = grip_center + delta;
        let dragging = container(
            vec![pointer_press(grip_center, true), Event::PointerMoved(moved)],
            time,
        );
        let expected_width = START_SIZE.x - DELTA;
        assert!(
            (dragging.width() - expected_width).abs() <= 1.0,
            "rtl = {rtl}: the sizing-pass frame must report the requested shrink \
             (expected ~{expected_width}, got {dragging:?})"
        );
        if rtl {
            assert!(
                (dragging.right() - prev.right()).abs() <= 1.0,
                "rtl = {rtl}: the reading-start (right) edge must stay fixed on the \
                 sizing-pass frame (settled {prev:?}, dragging {dragging:?})"
            );
        } else {
            assert!(
                (dragging.left() - prev.left()).abs() <= 1.0,
                "rtl = {rtl}: the reading-start (left) edge must stay fixed on the \
                 sizing-pass frame (settled {prev:?}, dragging {dragging:?})"
            );
        }

        // Release: the corner response keeps carrying the pointer through the
        // release frame, which re-applies the request and re-measures, so the
        // shrunk size must stick across idle frames.
        for i in 0..3 {
            time += 0.1;
            let idle = container(vec![], time);
            assert!(
                (idle.width() - expected_width).abs() <= 1.0,
                "rtl = {rtl}: the shrunk size must survive the release \
                 (dragging {dragging:?}, idle frame {i} {idle:?})"
            );
        }
    }
}
