//! `Resize` and resizable [`egui::Window`] mirroring under the whole-UI direction
//! flag ([`egui::Style::right_to_left`]): the grip sits at the reading-start corner
//! *of the container*, the end-of-frame geometry (cursor advance, stroke outline,
//! grip, debug rects) is anchored on the container rather than on its content,
//! drag deltas are direction-symmetric, the min-size floor is
//! direction-independent, persisted sizes survive direction flips, and resizable
//! windows keep their reading-start edge fixed while their size changes.
//!
//! Every mirrored case also asserts its LTR counterpart (flag off), so this file
//! doubles as the LTR-unchanged regression. No snapshot assertions; geometry,
//! painted shapes and cursor icons only.

use egui::epaint::RectShape;
use egui::{
    Align, CentralPanel, Context, CursorIcon, Event, FullOutput, Id, Layout, Modifiers,
    PointerButton, Pos2, RawInput, Rect, Ui, Vec2, Window, epaint::Shape, pos2, vec2,
};

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

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

/// The painted `Resize` container rect: the stroke outline is the container rect
/// expanded by 2.0 per side (LTR and RTL alike), so shrink it back.
fn container_rect_from_shapes(output: &FullOutput) -> Rect {
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

fn same_rect(a: Rect, b: Rect) -> bool {
    (a.left() - b.left()).abs() <= 0.5
        && (a.right() - b.right()).abs() <= 0.5
        && (a.top() - b.top()).abs() <= 0.5
        && (a.bottom() - b.bottom()).abs() <= 0.5
}

/// What one frame showed: the container rect (from the painted stroke) and the
/// corner-grip rect.
#[derive(Clone, Copy, Debug)]
struct Snapshot {
    container: Rect,
    grip: Rect,
}

/// A reusable single-`Resize` harness: one context with a fixed screen, one
/// `Resize` id and config, and frame/settle/drag helpers.
struct Harness {
    ctx: Context,
    screen: Vec2,
    id: Id,
    default_size: Vec2,
    min_size: Vec2,
}

impl Harness {
    fn new(rtl: bool, screen: Vec2, default_size: Vec2, min_size: Vec2) -> Self {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        Self {
            ctx,
            screen,
            id: Id::unique("resize_rtl_harness"),
            default_size,
            min_size,
        }
    }

    fn frame_output(&self, events: Vec<Event>, time: f64) -> (Snapshot, FullOutput) {
        let mut snapshot = Snapshot {
            container: Rect::NOTHING,
            grip: Rect::NOTHING,
        };
        let output = run(&self.ctx, input_at(self.screen, time, events), |ui| {
            egui::Resize::default()
                .id(self.id)
                .default_size(self.default_size)
                .min_size(self.min_size)
                .show(ui, |ui| {
                    ui.label("Content");
                });
            if let Some(corner) = self.ctx.read_response(self.id.with("__resize_corner")) {
                snapshot.grip = corner.rect;
            }
        });
        snapshot.container = container_rect_from_shapes(&output);
        (snapshot, output)
    }

    fn frame(&self, events: Vec<Event>, time: f64) -> Snapshot {
        self.frame_output(events, time).0
    }

    /// Run idle frames until the container stops changing (settling past any
    /// discarded sizing pass).
    fn settle(&self, mut time: f64) -> Snapshot {
        let mut prev = self.frame(vec![], time);
        for _ in 0..8 {
            time += 0.1;
            let next = self.frame(vec![], time);
            if same_rect(next.container, prev.container) {
                return next;
            }
            prev = next;
        }
        prev
    }

    /// Drag the grip by `delta`, release, settle, and return the final snapshot.
    fn drag(&self, delta: Vec2, time: f64) -> Snapshot {
        let idle = self.settle(time);
        assert!(idle.grip.is_positive(), "no corner grip response captured");
        let grip_center = idle.grip.center();
        self.frame(vec![Event::PointerMoved(grip_center)], time + 1.0);
        self.frame(vec![pointer_press(grip_center, true)], time + 1.1);
        let moved = grip_center + delta;
        self.frame(vec![Event::PointerMoved(moved)], time + 1.2);
        self.frame(vec![pointer_press(moved, false)], time + 1.3);
        self.settle(time + 1.4)
    }
}

// ---------------------------------------------------------------------------

/// A1: under RTL the corner grip sits in the bottom-left corner *of the container*
/// (its left edge on the container's left edge), under LTR in the bottom-right
/// corner, and the grip stays inside the container in both directions. The RTL
/// case runs the repro that pins the end-of-frame fix: a horizontal
/// right-to-left parent where the container is much wider than its content (the
/// old min_rect-anchored grip landed mid-container there).
#[test]
fn grip_is_bottom_left_under_rtl_bottom_right_under_ltr() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);

    fn frame(rtl: bool) -> (Rect, Rect) {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let id = Id::unique("grip_corner_container");
        let mut grip = Rect::NOTHING;
        let output = run(&ctx, input(SCREEN), |ui| {
            CentralPanel::default().show(ui, |ui| {
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    egui::Resize::default()
                        .id(id)
                        .show(ui, |ui| ui.label("Content"));
                });
            });
            if let Some(corner) = ctx.read_response(id.with("__resize_corner")) {
                grip = corner.rect;
            }
        });
        (container_rect_from_shapes(&output), grip)
    }

    // RTL: the grip hugs the container's reading-start (bottom-left) corner:
    let (container, grip) = frame(true);
    assert!(
        container.is_positive() && grip.is_positive(),
        "the frame should paint the container and the grip (container {container:?}, grip {grip:?})"
    );
    assert!(
        (grip.left() - container.left()).abs() <= 1.0,
        "RTL: the grip must sit on the container's reading-start (left) edge \
         (container {container:?}, grip {grip:?})"
    );
    assert!(
        (grip.bottom() - container.bottom()).abs() <= 1.0,
        "RTL: the grip must sit on the container's bottom edge \
         (container {container:?}, grip {grip:?})"
    );
    assert!(
        container.contains_rect(grip),
        "RTL: the grip must stay inside the container (container {container:?}, grip {grip:?})"
    );

    // LTR counterpart: bottom-right corner:
    let (container, grip) = frame(false);
    assert!(container.is_positive() && grip.is_positive());
    assert!(
        (grip.right() - container.right()).abs() <= 1.0,
        "LTR: the grip must sit on the container's right edge \
         (container {container:?}, grip {grip:?})"
    );
    assert!(
        (grip.bottom() - container.bottom()).abs() <= 1.0,
        "LTR: the grip must sit on the container's bottom edge \
         (container {container:?}, grip {grip:?})"
    );
    assert!(
        container.contains_rect(grip),
        "LTR: the grip must stay inside the container (container {container:?}, grip {grip:?})"
    );
}

/// A2: a drag along the reading direction grows the container while the
/// reading-start edge stays fixed (right under RTL, left under LTR); the top
/// edge is fixed in both directions.
#[test]
fn resize_drag_keeps_reading_start_edge_fixed() {
    const SCREEN: Vec2 = vec2(400.0, 300.0);
    const DEFAULT_SIZE: Vec2 = vec2(200.0, 100.0);
    const MIN_SIZE: Vec2 = Vec2::splat(16.0);

    // LTR: the left (reading-start) edge stays fixed while a +x drag grows the rect:
    let ltr = Harness::new(false, SCREEN, DEFAULT_SIZE, MIN_SIZE);
    let idle = ltr.settle(0.0);
    let dragged = ltr.drag(vec2(50.0, 0.0), 0.0);
    assert!(
        (dragged.container.left() - idle.container.left()).abs() <= 1.0,
        "LTR: the reading-start (left) edge must stay fixed \
         (idle {:?}, dragged {:?})",
        idle.container,
        dragged.container
    );
    assert!(
        (dragged.container.right() - idle.container.right() - 50.0).abs() <= 1.0,
        "LTR: dragging the grip along the reading direction must grow the rect by ~50 \
         (idle {:?}, dragged {:?})",
        idle.container,
        dragged.container
    );
    assert!(
        (dragged.container.top() - idle.container.top()).abs() <= 1.0,
        "the top edge must stay fixed (idle {:?}, dragged {:?})",
        idle.container,
        dragged.container
    );

    // RTL: the mirrored drag (-x) grows the rect; the right (reading-start) edge
    // stays fixed:
    let rtl = Harness::new(true, SCREEN, DEFAULT_SIZE, MIN_SIZE);
    let idle = rtl.settle(0.0);
    let dragged = rtl.drag(vec2(-50.0, 0.0), 0.0);
    assert!(
        (dragged.container.right() - idle.container.right()).abs() <= 1.0,
        "RTL: the reading-start (right) edge must stay fixed \
         (idle {:?}, dragged {:?})",
        idle.container,
        dragged.container
    );
    assert!(
        (dragged.container.left() - idle.container.left() + 50.0).abs() <= 1.0,
        "RTL: dragging the grip along the reading direction must grow the rect by ~50 \
         (idle {:?}, dragged {:?})",
        idle.container,
        dragged.container
    );
    assert!(
        (dragged.container.top() - idle.container.top()).abs() <= 1.0,
        "the top edge must stay fixed (idle {:?}, dragged {:?})",
        idle.container,
        dragged.container
    );
}

/// A3: the same pointer movement *relative to the grip* (mirrored in x) yields
/// the same resulting size in both directions — the correctness of the x
/// negation in the drag delta.
#[test]
fn drag_delta_is_direction_symmetric() {
    const SCREEN: Vec2 = vec2(400.0, 300.0);
    const DEFAULT_SIZE: Vec2 = vec2(200.0, 100.0);
    const MIN_SIZE: Vec2 = Vec2::splat(16.0);

    let ltr = Harness::new(false, SCREEN, DEFAULT_SIZE, MIN_SIZE);
    let rtl = Harness::new(true, SCREEN, DEFAULT_SIZE, MIN_SIZE);
    // Mirrored deltas: +50 right of the LTR grip, 50 left of the RTL grip.
    let ltr = ltr.drag(vec2(50.0, 30.0), 0.0).container;
    let rtl = rtl.drag(vec2(-50.0, 30.0), 0.0).container;
    assert!(
        (ltr.width() - rtl.width()).abs() <= 1.0,
        "the same drag relative to the grip must give the same width \
         (LTR {ltr:?}, RTL {rtl:?})"
    );
    assert!(
        (ltr.height() - rtl.height()).abs() <= 1.0,
        "the same drag relative to the grip must give the same height \
         (LTR {ltr:?}, RTL {rtl:?})"
    );
}

/// A4: the grip cursor matches the grip diagonal: `ResizeNeSw` under RTL
/// (bottom-left grip), `ResizeNwSe` under LTR (bottom-right grip).
#[test]
fn cursor_icon_matches_grip_diagonal() {
    const SCREEN: Vec2 = vec2(400.0, 300.0);
    const DEFAULT_SIZE: Vec2 = vec2(200.0, 100.0);
    const MIN_SIZE: Vec2 = Vec2::splat(16.0);

    for (rtl, expected) in [
        (false, CursorIcon::ResizeNwSe),
        (true, CursorIcon::ResizeNeSw),
    ] {
        let harness = Harness::new(rtl, SCREEN, DEFAULT_SIZE, MIN_SIZE);
        let idle = harness.settle(0.0);
        let grip_center = idle.grip.center();
        harness.frame(vec![Event::PointerMoved(grip_center)], 1.0);
        let (_, output) = harness.frame_output(vec![Event::PointerMoved(grip_center)], 1.1);
        assert_eq!(
            output.platform_output.cursor_icon, expected,
            "rtl = {rtl}: the grip cursor should match the grip diagonal"
        );
    }
}

/// A5: over-shrink drags clamp to exactly the configured min size
/// (± 1 rounding) in both directions — the floor is direction-independent.
#[test]
fn min_size_floor_identical_in_both_directions() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);
    const MIN_SIZE: Vec2 = vec2(560.0, 400.0);

    for rtl in [false, true] {
        let harness = Harness::new(rtl, SCREEN, vec2(700.0, 500.0), MIN_SIZE);
        // Over-shrink: far toward the reading end (left+up in LTR, right+up in RTL).
        let delta = if rtl {
            vec2(500.0, -500.0)
        } else {
            vec2(-500.0, -500.0)
        };
        let shrunk = harness.drag(delta, 0.0).container;
        assert!(
            (shrunk.width() - MIN_SIZE.x).abs() <= 1.0,
            "rtl = {rtl}: the over-shrink drag must clamp to the configured min width \
             (got {shrunk:?}, min {MIN_SIZE:?})"
        );
        assert!(
            (shrunk.height() - MIN_SIZE.y).abs() <= 1.0,
            "rtl = {rtl}: the over-shrink drag must clamp to the configured min height \
             (got {shrunk:?}, min {MIN_SIZE:?})"
        );
    }
}

/// A6: in a horizontal RTL parent, the widget placed after the `Resize` container
/// does not intersect the container rect once the container is wider than its
/// content (the cursor advance must be anchored on the container, not on the
/// content's `min_rect`); under LTR the next widget sits right of the container.
#[test]
fn next_widget_does_not_overlap_after_resize_under_rtl() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);

    fn frame(rtl: bool) -> (Rect, Rect) {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let id = Id::unique("no_overlap_row");
        let layout = if rtl {
            Layout::right_to_left(Align::Min)
        } else {
            Layout::left_to_right(Align::Min)
        };
        let mut next = Rect::NOTHING;
        let output = run(&ctx, input(SCREEN), |ui| {
            CentralPanel::default().show(ui, |ui| {
                ui.with_layout(layout, |ui| {
                    egui::Resize::default()
                        .id(id)
                        .show(ui, |ui| ui.label("Content"));
                    next = ui.label("next").rect;
                });
            });
        });
        (container_rect_from_shapes(&output), next)
    }

    // RTL: the next widget sits entirely left of the container:
    let (container, next) = frame(true);
    assert!(
        container.is_positive() && next.is_positive(),
        "the frame should paint the container and the next widget \
         (container {container:?}, next {next:?})"
    );
    assert!(
        !container.intersects(next),
        "RTL: the next widget must not overlap the container \
         (container {container:?}, next {next:?})"
    );
    assert!(
        next.right() <= container.left() + 0.5,
        "RTL: the next widget must sit left of the container's reading-start edge \
         (container {container:?}, next {next:?})"
    );

    // LTR counterpart: the next widget sits entirely right of the container:
    let (container, next) = frame(false);
    assert!(
        !container.intersects(next),
        "LTR: the next widget must not overlap the container \
         (container {container:?}, next {next:?})"
    );
    assert!(
        next.left() >= container.right() - 0.5,
        "LTR: the next widget must sit right of the container \
         (container {container:?}, next {next:?})"
    );
}

/// A7: persisted state survives a direction flip: drag the container to a
/// non-default size under RTL, flip the flag between frames, and the size is
/// preserved while the anchor edge switches (and flips back symmetrically).
#[test]
fn resize_state_persists_across_direction_flip() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);
    const DEFAULT_SIZE: Vec2 = vec2(200.0, 100.0);
    const MIN_SIZE: Vec2 = Vec2::splat(16.0);

    let harness = Harness::new(true, SCREEN, DEFAULT_SIZE, MIN_SIZE);

    // Drag the container larger along the reading direction (RTL: -x):
    let before = harness.drag(vec2(-60.0, 0.0), 0.0);
    let (width, height) = (before.container.width(), before.container.height());
    assert!(
        width > DEFAULT_SIZE.x + 20.0,
        "the drag should have grown the container (got {before:?})"
    );

    // Flip the flag *between* frames (never mid-frame):
    harness.ctx.set_right_to_left(false);
    let ltr = harness.settle(10.0);
    assert!(
        (ltr.container.width() - width).abs() <= 1.0
            && (ltr.container.height() - height).abs() <= 1.0,
        "the size must survive the direction flip (before {before:?}, after {ltr:?})"
    );
    assert!(
        ltr.container.left() <= 1.0,
        "LTR: the container must anchor on its reading-start (left) edge after the flip \
         (got {ltr:?})"
    );

    // ...and flip back:
    harness.ctx.set_right_to_left(true);
    let rtl_again = harness.settle(20.0);
    assert!(
        (rtl_again.container.width() - width).abs() <= 1.0
            && (rtl_again.container.height() - height).abs() <= 1.0,
        "the size must survive flipping back (before {before:?}, after {rtl_again:?})"
    );
    assert!(
        (rtl_again.container.right() - SCREEN.x).abs() <= 1.0,
        "RTL: the reading-start (right) edge must be back at the offered right edge \
         (got {rtl_again:?})"
    );
}

/// A8: a resizable `Window` keeps its reading-start edge fixed while its size
/// changes: under RTL the right edge stays put and the left edge moves by the
/// width delta; under LTR the left edge stays put. Covers growth, the no-drift
/// case, content shrink (windows never auto-shrink with their content, so the
/// window must stay exactly put), and a genuine negative width delta via the
/// max-size clamp (screen narrower than the window). Also asserts there is no
/// drift once the size is unchanged across frames.
#[test]
fn resizable_window_keeps_reading_start_edge_fixed_under_rtl() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);
    const NARROW: &str = "hi";
    // ~500 points wide when not wrapped — much wider than the 200-point window.
    const WIDE: &str = "wwwwwwwwww wwwwwwwwww wwwwwwwwww wwwwwwwwww wwwwwwwwww";

    fn input_on(screen: Rect) -> RawInput {
        RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        }
    }

    /// The window is placed so the wide state fits on screen in both directions:
    /// under RTL the window grows leftward (placed near the right edge), under
    /// LTR rightward (placed near the left edge). `default_pos` roots the
    /// window on its reading-start edge — under RTL that is the right edge, so
    /// the position leaves ~650 points of room for the window to widen into.
    fn frame(ctx: &Context, rtl: bool, wide: bool, screen_w: f32) -> Rect {
        let pos = if rtl {
            pos2(650.0, 50.0)
        } else {
            pos2(100.0, 50.0)
        };
        let mut rect = Rect::NOTHING;
        run(
            ctx,
            input_on(Rect::from_min_size(Pos2::ZERO, vec2(screen_w, SCREEN.y))),
            |_ui| {
                let show = |ui: &mut Ui| {
                    if wide {
                        ui.add(egui::Label::new(WIDE).wrap_mode(egui::TextWrapMode::Extend));
                    } else {
                        ui.label(NARROW);
                    }
                };
                if let Some(response) = Window::new("rtl win")
                    .resizable(true)
                    .default_size(vec2(200.0, 100.0))
                    .default_pos(pos)
                    .show(ctx, show)
                {
                    rect = response.response.rect;
                }
            },
        );
        rect
    }

    fn settle(ctx: &Context, rtl: bool, wide: bool, screen_w: f32) -> Rect {
        let mut prev = frame(ctx, rtl, wide, screen_w);
        for _ in 0..8 {
            let next = frame(ctx, rtl, wide, screen_w);
            if same_rect(next, prev) {
                return next;
            }
            prev = next;
        }
        prev
    }

    // RTL: content-driven growth keeps the right (reading-start) edge fixed:
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let before = settle(&ctx, true, false, SCREEN.x);
    assert!(
        before.is_positive(),
        "the window should be laid out (got {before:?})"
    );
    let after = settle(&ctx, true, true, SCREEN.x);
    assert!(
        after.width() > before.width() + 20.0,
        "the window should grow with its content (before {before:?}, after {after:?})"
    );
    assert!(
        (after.right() - before.right()).abs() <= 1.0,
        "RTL: the window's reading-start (right) edge must stay fixed while it grows \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.left() - (before.left() - (after.width() - before.width()))).abs() <= 1.0,
        "RTL: the window's left edge must move by the width delta \
         (before {before:?}, after {after:?})"
    );

    // No drift once the size is unchanged across frames:
    let extra1 = frame(&ctx, true, true, SCREEN.x);
    let extra2 = frame(&ctx, true, true, SCREEN.x);
    assert!(
        same_rect(extra1, after) && same_rect(extra2, after),
        "RTL: an unchanged size must not drift the window \
         (settled {after:?}, extra1 {extra1:?}, extra2 {extra2:?})"
    );

    // Content shrink: a window never auto-shrinks with its content (upstream
    // `Resize` semantics: the desired size is sticky), and under RTL the content is
    // laid out against the reading-start edge while the parent's `min_rect` is
    // seeded on the old reading-end edge, so the persisted width can only grow.
    // The window must therefore stay exactly where it was — the reading-start
    // (right) edge fixed, no drift of any kind:
    let shrunk = settle(&ctx, true, false, SCREEN.x);
    assert!(
        same_rect(shrunk, after),
        "RTL: narrowing the content must not move or resize the window \
         (wide {after:?}, narrow {shrunk:?})"
    );

    // LTR counterpart: the left edge stays fixed and the right edge moves:
    let ctx = Context::default();
    let before = settle(&ctx, false, false, SCREEN.x);
    let after = settle(&ctx, false, true, SCREEN.x);
    assert!(
        after.width() > before.width() + 20.0,
        "the window should grow with its content (before {before:?}, after {after:?})"
    );
    assert!(
        (after.left() - before.left()).abs() <= 1.0,
        "LTR: the window's reading-start (left) edge must stay fixed while it grows \
         (before {before:?}, after {after:?})"
    );
    assert!(
        (after.right() - (before.right() + (after.width() - before.width()))).abs() <= 1.0,
        "LTR: the window's right edge must move by the width delta \
         (before {before:?}, after {after:?})"
    );

    // Content shrink counterpart: the desired size is sticky in both directions,
    // so the window must stay exactly where it was — the reading-start (left)
    // edge fixed, no drift of any kind:
    let shrunk = settle(&ctx, false, false, SCREEN.x);
    assert!(
        same_rect(shrunk, after),
        "LTR: narrowing the content must not move or resize the window \
         (wide {after:?}, narrow {shrunk:?})"
    );

    // Genuine negative width delta, via the max-size clamp: a screen narrower than
    // the window clamps the window's max width (with constraining off, so the
    // clamp is the only thing acting on the window). In both directions the window
    // shrinks with the reading-start edge fixed and the far edge moving by
    // exactly the width delta.
    for (rtl, pos) in [(true, pos2(650.0, 50.0)), (false, pos2(100.0, 50.0))] {
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);

        fn clamp_frame(ctx: &Context, screen_w: f32, pos: Pos2) -> Rect {
            let mut rect = Rect::NOTHING;
            run(
                ctx,
                input_on(Rect::from_min_size(Pos2::ZERO, vec2(screen_w, 500.0))),
                |_ui| {
                    if let Some(response) = Window::new("rtl win clamp")
                        .resizable(true)
                        .constrain(false)
                        .default_size(vec2(200.0, 100.0))
                        .default_pos(pos)
                        .show(ctx, |ui| {
                            ui.label(NARROW);
                        })
                    {
                        rect = response.response.rect;
                    }
                },
            );
            rect
        }

        fn clamp_settle(ctx: &Context, screen_w: f32, pos: Pos2) -> Rect {
            let mut prev = clamp_frame(ctx, screen_w, pos);
            for _ in 0..8 {
                let next = clamp_frame(ctx, screen_w, pos);
                if same_rect(next, prev) {
                    return next;
                }
                prev = next;
            }
            prev
        }

        let wide_screen = clamp_settle(&ctx, 700.0, pos);
        assert!(
            wide_screen.is_positive() && wide_screen.width() > 150.0,
            "the window should be laid out wider than the narrow screen \
             (rtl {rtl}, got {wide_screen:?})"
        );
        let clamped = clamp_settle(&ctx, 150.0, pos);
        if rtl {
            assert!(
                clamped.width() < wide_screen.width() - 20.0,
                "RTL: clamping the max size must shrink the window \
                 (wide screen {wide_screen:?}, narrow screen {clamped:?})"
            );
            assert!(
                (clamped.right() - wide_screen.right()).abs() <= 1.0,
                "RTL: the window's reading-start (right) edge must stay fixed while the \
                 max-size clamp shrinks it \
                 (wide screen {wide_screen:?}, narrow screen {clamped:?})"
            );
            let delta = wide_screen.width() - clamped.width();
            assert!(
                (clamped.left() - (wide_screen.left() + delta)).abs() <= 1.0,
                "RTL: the window's left edge must move right by the width delta while \
                 the max-size clamp shrinks it \
                 (wide screen {wide_screen:?}, narrow screen {clamped:?})"
            );
        } else {
            assert!(
                clamped.width() < wide_screen.width() - 20.0,
                "LTR: clamping the max size must shrink the window \
                 (wide screen {wide_screen:?}, narrow screen {clamped:?})"
            );
            assert!(
                (clamped.left() - wide_screen.left()).abs() <= 1.0,
                "LTR: the window's reading-start (left) edge must stay fixed while the \
                 max-size clamp shrinks it \
                 (wide screen {wide_screen:?}, narrow screen {clamped:?})"
            );
            let delta = wide_screen.width() - clamped.width();
            assert!(
                (clamped.right() - (wide_screen.right() - delta)).abs() <= 1.0,
                "LTR: the window's right edge must move left by the width delta while \
                 the max-size clamp shrinks it \
                 (wide screen {wide_screen:?}, narrow screen {clamped:?})"
            );
        }
    }
}
