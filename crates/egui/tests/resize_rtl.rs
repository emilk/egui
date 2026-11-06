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

/// Under RTL the corner grip sits in the bottom-left corner *of the container*
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

/// A drag along the reading direction grows the container while the
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

/// The same pointer movement *relative to the grip* (mirrored in x) yields
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

/// The grip cursor matches the grip diagonal: `ResizeNeSw` under RTL
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

/// Over-shrink drags clamp to exactly the configured min size
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

/// In a horizontal RTL parent, the widget placed after the `Resize` container
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

/// Persisted state survives a direction flip: drag the container to a
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

/// A resizable `Window` keeps its reading-start edge fixed while its size
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

    // Content shrink: a window never auto-shrinks with its content — on idle
    // frames the inner `Resize` keeps its sticky `desired_size` (grow-only via
    // the max against the fresh measurement) and the width-filling title-bar
    // chrome keeps the fresh measurement at the offered width, so the persisted
    // width cannot drop. The reading-start seed is what allows clamp and drag
    // shrinks; plain content narrowing is neither, so the window must stay
    // exactly where it was — the reading-start (right) edge fixed, no drift of
    // any kind:
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

/// A resizable [`Window`] paints its grip in the bottom-right corner under
/// LTR and the bottom-left corner under RTL — the grip mirrors with the
/// reading direction, like the `Resize` container's grip (the window's corner
/// selection always prefers the bottom-right, and a movable window enables
/// both left and right resize edges, so the choice has to be mirrored). The
/// grip is identified by the diagonal line segments the window paints near
/// its corner (the only diagonal lines a plain window paints).
#[test]
fn window_grip_corner_mirrors_under_rtl() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);

    /// The painted grip rect, from the window's diagonal line segments:
    fn grip_rect(output: &FullOutput) -> Rect {
        let mut grip = Rect::NOTHING;
        for clipped in &output.shapes {
            if let Shape::LineSegment { points: [a, b], .. } = &clipped.shape
                && (a.x - b.x).abs() > 0.5
                && (a.y - b.y).abs() > 0.5
            {
                grip = grip.union(Rect::from_two_pos(*a, *b));
            }
        }
        grip
    }

    fn frame(ctx: &Context) -> (Rect, Rect) {
        let mut window = Rect::NOTHING;
        let output = run(ctx, input(SCREEN), |_ui| {
            if let Some(response) = Window::new("grip window")
                .resizable(true)
                .default_pos(pos2(100.0, 50.0))
                .default_size(vec2(300.0, 200.0))
                .show(ctx, |ui| {
                    ui.label("Content");
                })
            {
                window = response.response.rect;
            }
        });
        (window, grip_rect(&output))
    }

    fn settle(ctx: &Context) -> (Rect, Rect) {
        let mut prev = frame(ctx);
        for _ in 0..8 {
            let next = frame(ctx);
            if same_rect(next.0, prev.0) {
                return next;
            }
            prev = next;
        }
        prev
    }

    // LTR: the grip hugs the window's bottom-right corner:
    let ctx = Context::default();
    let (window, grip) = settle(&ctx);
    assert!(
        window.is_positive() && grip.is_positive(),
        "the frame should lay out the window and paint the grip \
         (window {window:?}, grip {grip:?})"
    );
    assert!(
        (grip.right() - window.right()).abs() <= 8.0,
        "LTR: the grip must sit on the window's right edge \
         (window {window:?}, grip {grip:?})"
    );
    assert!(
        (grip.bottom() - window.bottom()).abs() <= 8.0,
        "LTR: the grip must sit on the window's bottom edge \
         (window {window:?}, grip {grip:?})"
    );
    assert!(
        grip.center().x > window.center().x && grip.center().y > window.center().y,
        "LTR: the grip must sit in the window's bottom-right quadrant \
         (window {window:?}, grip {grip:?})"
    );

    // RTL: the grip hugs the window's bottom-left (reading-start) corner:
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let (window, grip) = settle(&ctx);
    assert!(
        window.is_positive() && grip.is_positive(),
        "the frame should lay out the window and paint the grip \
         (window {window:?}, grip {grip:?})"
    );
    assert!(
        (grip.left() - window.left()).abs() <= 8.0,
        "RTL: the grip must sit on the window's reading-start (left) edge \
         (window {window:?}, grip {grip:?})"
    );
    assert!(
        (grip.bottom() - window.bottom()).abs() <= 8.0,
        "RTL: the grip must sit on the window's bottom edge \
         (window {window:?}, grip {grip:?})"
    );
    assert!(
        grip.center().x < window.center().x && grip.center().y > window.center().y,
        "RTL: the grip must sit in the window's bottom-left quadrant \
         (window {window:?}, grip {grip:?})"
    );
}

// ---------------------------------------------------------------------------
// The window edge strips. A window's sides are drag strips
// (`resize_grab_radius_side` = 3.0 wide, corners excluded by
// `resize_grab_radius_corner` = 10.0); `PossibleInteractions` gates which
// sides exist, and under RTL the pivot is the reading-start corner.

/// A reusable window harness for the edge-strip tests: one context, one window
/// id, configurable movability, position and body, and frame/settle/edge-drag
/// helpers. The edge drag presses the strip, moves in three steps, releases,
/// settles and asserts no drift on two idle frames — the no-drift pin for the
/// `set_rect_pivot_pos` pivot re-derivation and the `edge_drag_in_flight`
/// compensation-skip boundary.
struct WindowHarness {
    ctx: Context,
    screen: Vec2,
    id: Id,
    movable: bool,
    pos: Pos2,
    body: Body,
}

/// The window body: a plain label, or a vertically-filling `ScrollArea` (the
/// window then renders at its requested height, so a vertical edge drag
/// resizes instead of translating — the content-height quirk pinned by
/// `window_top_edge_on_content_height_window_translates`).
enum Body {
    Label,
    Filling,
}

impl WindowHarness {
    fn new(rtl: bool, movable: bool, pos: Pos2, body: Body) -> Self {
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        Self {
            ctx,
            screen: vec2(700.0, 500.0),
            id: Id::unique("window_edge_strip_rtl"),
            movable,
            pos,
            body,
        }
    }

    fn frame(&self, events: Vec<Event>, time: f64) -> Rect {
        let mut rect = Rect::NOTHING;
        let id = self.id;
        let movable = self.movable;
        let pos = self.pos;
        let body = &self.body;
        let ctx = &self.ctx;
        let mut output = run(ctx, input_at(self.screen, time, events), |_ui| {
            let title = if movable { "edge win" } else { "pinned win" };
            if let Some(response) = Window::new(title)
                .id(id)
                .resizable(true)
                .movable(movable)
                .default_size(vec2(300.0, 200.0))
                .default_pos(pos)
                .show(ctx, |ui| match body {
                    Body::Filling => {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                ui.set_min_height(ui.available_height());
                                ui.label("Content");
                            });
                    }
                    Body::Label => {
                        ui.label("Content");
                    }
                })
            {
                rect = response.response.rect;
            }
        });
        output.textures_delta.clear();
        rect
    }

    /// Run idle frames until the window stops changing (settling past any
    /// discarded sizing pass).
    fn settle(&self, mut time: f64) -> Rect {
        let mut prev = self.frame(vec![], time);
        for _ in 0..10 {
            time += 0.1;
            let next = self.frame(vec![], time);
            if same_rect(next, prev) {
                return next;
            }
            prev = next;
        }
        prev
    }

    /// Press the strip at `pos`, drag by `delta` in three steps, release,
    /// settle, and assert the settled rect does not drift on two idle frames.
    /// Returns the settled rect.
    fn edge_drag(&self, pos: Pos2, delta: Vec2, mut time: f64) -> Rect {
        self.frame(vec![Event::PointerMoved(pos)], time);
        time += 0.1;
        self.frame(vec![pointer_press(pos, true)], time);
        let mut current = pos;
        for _ in 0..3 {
            time += 0.1;
            current += delta / 3.0;
            self.frame(vec![Event::PointerMoved(current)], time);
        }
        time += 0.1;
        self.frame(vec![pointer_press(current, false)], time);
        let settled = self.settle(time + 0.2);
        let idle1 = self.frame(vec![], time + 1.0);
        let idle2 = self.frame(vec![], time + 1.1);
        assert!(
            same_rect(idle1, settled) && same_rect(idle2, settled),
            "the window must not drift after the edge drag settles \
             (settled {settled:?}, idle1 {idle1:?}, idle2 {idle2:?})"
        );
        settled
    }
}

/// A movable window's edge strips move exactly the dragged side: the
/// right-edge strip drags only the right edge, the left-edge strip only the
/// left edge, and (with a vertically-filling body, so the window renders at
/// its requested height) the bottom strip only the bottom edge and the top
/// strip — pressed 2 points *above* the top edge, inside the 3-point side grab
/// band and above the title bar — only the top edge. The right strip pressed
/// at title-bar height still resizes (the strip sits above the title bar in
/// hit-test order). After every drag the settled window does not drift.
#[test]
fn window_edge_strip_drag_moves_exactly_that_side() {
    // RTL: the window is rooted on its reading-start (right) edge, so the drag
    // sides have room to move into:
    {
        let h = WindowHarness::new(true, true, pos2(650.0, 50.0), Body::Label);
        let before = h.settle(0.0);
        assert!(
            before.is_positive(),
            "the window should be laid out (got {before:?})"
        );

        // Right edge (the reading-start edge under RTL), +30 x: exactly that
        // side moves.
        let after = h.edge_drag(
            pos2(before.right(), before.center().y),
            vec2(30.0, 0.0),
            2.0,
        );
        assert!(
            (after.right() - before.right() - 30.0).abs() <= 1.0,
            "RTL: the right-edge strip must move exactly the right edge \
             (before {before:?}, after {after:?})"
        );
        assert!(
            (after.left() - before.left()).abs() <= 1.0,
            "RTL: the right-edge strip must not move the left edge \
             (before {before:?}, after {after:?})"
        );

        // Left edge (the reading-end edge under RTL), -20 x: exactly that side
        // moves.
        let after_left = h.edge_drag(pos2(after.left(), after.center().y), vec2(-20.0, 0.0), 8.0);
        assert!(
            (after_left.left() - after.left() + 20.0).abs() <= 1.0,
            "RTL: the left-edge strip must move exactly the left edge \
             (before {after:?}, after {after_left:?})"
        );
        assert!(
            (after_left.right() - after.right()).abs() <= 1.0,
            "RTL: the left-edge strip must not move the right edge \
             (before {after:?}, after {after_left:?})"
        );

        // The right strip pressed at title-bar height still resizes (the strip
        // is hit-tested above the title bar). Shrinking, so the dragged side
        // has room: the window sits 20 points from the screen edge after the
        // first drag, and a clamp would shift the whole rect instead.
        let after_title = h.edge_drag(
            pos2(after_left.right(), after_left.top() + 12.0),
            vec2(-20.0, 0.0),
            14.0,
        );
        assert!(
            (after_title.right() - after_left.right() + 20.0).abs() <= 1.0,
            "RTL: the right strip pressed at title-bar height must still resize \
             (before {after_left:?}, after {after_title:?})"
        );
        assert!(
            (after_title.left() - after_left.left()).abs() <= 1.0,
            "RTL: the title-bar-height right strip must not move the left edge \
             (before {after_left:?}, after {after_title:?})"
        );
    }

    // RTL, vertically-filling body: the window renders at its requested height,
    // so the vertical strips resize:
    {
        let h = WindowHarness::new(true, true, pos2(650.0, 50.0), Body::Filling);
        let before = h.settle(0.0);
        assert!(
            before.is_positive(),
            "the window should be laid out (got {before:?})"
        );

        // Bottom edge, +30 y: exactly that side moves.
        let after = h.edge_drag(
            pos2(before.center().x, before.bottom()),
            vec2(0.0, 30.0),
            2.0,
        );
        assert!(
            (after.bottom() - before.bottom() - 30.0).abs() <= 1.0,
            "RTL: the bottom-edge strip must move exactly the bottom edge \
             (before {before:?}, after {after:?})"
        );
        assert!(
            (after.top() - before.top()).abs() <= 1.0,
            "RTL: the bottom-edge strip must not move the top edge \
             (before {before:?}, after {after:?})"
        );

        // Top edge, pressed 2 points above the top edge, -20 y: exactly that
        // side moves.
        let after_top = h.edge_drag(
            pos2(after.center().x, after.top() - 2.0),
            vec2(0.0, -20.0),
            8.0,
        );
        assert!(
            (after_top.top() - after.top() + 20.0).abs() <= 1.0,
            "RTL: the top-edge strip (pressed above the window) must move exactly \
             the top edge (before {after:?}, after {after_top:?})"
        );
        assert!(
            (after_top.bottom() - after.bottom()).abs() <= 1.0,
            "RTL: the top-edge strip must not move the bottom edge \
             (before {after:?}, after {after_top:?})"
        );
    }

    // LTR twins with mirrored deltas: the left edge is the reading-start edge:
    {
        let h = WindowHarness::new(false, true, pos2(100.0, 50.0), Body::Label);
        let before = h.settle(0.0);
        assert!(
            before.is_positive(),
            "the window should be laid out (got {before:?})"
        );

        // Left edge (the reading-start edge under LTR), -20 x: exactly that
        // side moves.
        let after = h.edge_drag(
            pos2(before.left(), before.center().y),
            vec2(-20.0, 0.0),
            2.0,
        );
        assert!(
            (after.left() - before.left() + 20.0).abs() <= 1.0,
            "LTR: the left-edge strip must move exactly the left edge \
             (before {before:?}, after {after:?})"
        );
        assert!(
            (after.right() - before.right()).abs() <= 1.0,
            "LTR: the left-edge strip must not move the right edge \
             (before {before:?}, after {after:?})"
        );

        // Right edge (the reading-end edge under LTR), +30 x: exactly that
        // side moves.
        let after_right = h.edge_drag(pos2(after.right(), after.center().y), vec2(30.0, 0.0), 8.0);
        assert!(
            (after_right.right() - after.right() - 30.0).abs() <= 1.0,
            "LTR: the right-edge strip must move exactly the right edge \
             (before {after:?}, after {after_right:?})"
        );
        assert!(
            (after_right.left() - after.left()).abs() <= 1.0,
            "LTR: the right-edge strip must not move the left edge \
             (before {after:?}, after {after_right:?})"
        );
    }

    // LTR, vertically-filling body: the vertical strips are direction-neutral:
    {
        let h = WindowHarness::new(false, true, pos2(100.0, 50.0), Body::Filling);
        let before = h.settle(0.0);
        assert!(
            before.is_positive(),
            "the window should be laid out (got {before:?})"
        );

        let after = h.edge_drag(
            pos2(before.center().x, before.bottom()),
            vec2(0.0, 30.0),
            2.0,
        );
        assert!(
            (after.bottom() - before.bottom() - 30.0).abs() <= 1.0
                && (after.top() - before.top()).abs() <= 1.0,
            "LTR: the bottom-edge strip must move exactly the bottom edge \
             (before {before:?}, after {after:?})"
        );

        let after_top = h.edge_drag(
            pos2(after.center().x, after.top() - 2.0),
            vec2(0.0, -20.0),
            8.0,
        );
        assert!(
            (after_top.top() - after.top() + 20.0).abs() <= 1.0
                && (after_top.bottom() - after.bottom()).abs() <= 1.0,
            "LTR: the top-edge strip (pressed above the window) must move exactly \
             the top edge (before {after:?}, after {after_top:?})"
        );
    }
}

/// A non-movable window keeps only its reading-end resize strips:
/// `PossibleInteractions` gates each side off when the pivot sits on it and
/// the window cannot be moved, so under RTL the right (reading-start) strip
/// does not exist — a drag there changes nothing — while the left (reading-end)
/// strip resizes. The LTR twin mirrors it.
#[test]
fn window_edge_strip_non_movable_side_gating() {
    // RTL: the pivot is the reading-start (right) corner, so the right strip is
    // gated off and the left strip resizes:
    {
        let h = WindowHarness::new(true, false, pos2(650.0, 50.0), Body::Label);
        let before = h.settle(0.0);
        assert!(
            before.is_positive(),
            "the window should be laid out (got {before:?})"
        );

        let after = h.edge_drag(
            pos2(before.right(), before.center().y),
            vec2(30.0, 0.0),
            2.0,
        );
        assert!(
            same_rect(after, before),
            "RTL: the gated-off right strip must not move or resize the window \
             (before {before:?}, after {after:?})"
        );

        // The reading-end (left) edge is resizable: dragging it rightward
        // shrinks the window from the left, the reading-start edge fixed.
        let after = h.edge_drag(pos2(before.left(), before.center().y), vec2(30.0, 0.0), 8.0);
        assert!(
            (after.left() - before.left() - 30.0).abs() <= 1.0,
            "RTL: the reading-end (left) strip must resize (before {before:?}, \
             after {after:?})"
        );
        assert!(
            (after.right() - before.right()).abs() <= 1.0,
            "RTL: the reading-start (right) edge must stay fixed while the \
             reading-end strip shrinks the window (before {before:?}, after {after:?})"
        );
    }

    // LTR twin: the left (reading-start) strip is gated off, the right
    // (reading-end) strip resizes:
    {
        let h = WindowHarness::new(false, false, pos2(100.0, 50.0), Body::Label);
        let before = h.settle(0.0);
        assert!(
            before.is_positive(),
            "the window should be laid out (got {before:?})"
        );

        let after = h.edge_drag(
            pos2(before.left(), before.center().y),
            vec2(-20.0, 0.0),
            2.0,
        );
        assert!(
            same_rect(after, before),
            "LTR: the gated-off left strip must not move or resize the window \
             (before {before:?}, after {after:?})"
        );

        let after = h.edge_drag(
            pos2(before.right(), before.center().y),
            vec2(30.0, 0.0),
            8.0,
        );
        assert!(
            (after.right() - before.right() - 30.0).abs() <= 1.0,
            "LTR: the reading-end (right) strip must resize (before {before:?}, \
             after {after:?})"
        );
        assert!(
            (after.left() - before.left()).abs() <= 1.0,
            "LTR: the reading-start (left) edge must stay fixed while the \
             reading-end strip grows the window (before {before:?}, after {after:?})"
        );
    }
}

/// This pins CURRENT behavior, not desired
/// behavior. A titled window whose body does not fill the offered height
/// renders at content height on y (the inner `Resize` is not resizable and
/// non-filling content never fills the offered height), so a top-edge drag
/// re-pins the pivot corner at the dragged rect while the height re-derives
/// from the content: the window *translates* by the drag delta instead of
/// resizing. The quirk is direction-neutral (no RTL branch in the y path) —
/// it is recorded here so a follow-up change inherits the reference
/// behavior; it must never be read as an RTL regression guard.
#[test]
fn window_top_edge_on_content_height_window_translates() {
    for (rtl, pos) in [(true, pos2(650.0, 50.0)), (false, pos2(100.0, 50.0))] {
        let dir = if rtl { "RTL" } else { "LTR" };
        let h = WindowHarness::new(rtl, true, pos, Body::Label);
        let before = h.settle(0.0);
        assert!(
            before.is_positive(),
            "{dir}: the window should be laid out (got {before:?})"
        );

        let after = h.edge_drag(pos2(before.center().x, before.top()), vec2(0.0, -20.0), 2.0);
        assert!(
            (after.top() - before.top() + 20.0).abs() <= 1.0,
            "{dir}: on a content-height window the top-edge drag translates the \
             window (before {before:?}, after {after:?})"
        );
        assert!(
            (after.bottom() - before.bottom() + 20.0).abs() <= 1.0,
            "{dir}: on a content-height window the top-edge drag moves the bottom \
             edge by the same delta (translate, not resize) \
             (before {before:?}, after {after:?})"
        );
        assert!(
            (after.width() - before.width()).abs() <= 1.0,
            "{dir}: the translate quirk must not change the width \
             (before {before:?}, after {after:?})"
        );
        if rtl {
            assert!(
                (after.right() - before.right()).abs() <= 1.0,
                "{dir}: the reading-start (right) edge must stay fixed \
                 (before {before:?}, after {after:?})"
            );
        } else {
            assert!(
                (after.left() - before.left()).abs() <= 1.0,
                "{dir}: the reading-start (left) edge must stay fixed \
                 (before {before:?}, after {after:?})"
            );
        }
    }
}
