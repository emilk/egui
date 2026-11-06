//! Regression tests for reading-direction side panels: [`egui::Panel::start`] and
//! [`egui::Panel::end`] resolve against [`egui::Style::right_to_left`] at show time,
//! while the physical [`egui::Panel::left`]/[`egui::Panel::right`] constructors never
//! follow the direction. The drag tests also pin the #7930 clamp: dragging a resize
//! separator past the panel's anchor edge stops at `min_size` and never reverses.
//!
//! Every mirrored case also asserts its LTR counterpart, so this file doubles as the
//! LTR-unchanged regression. No snapshot assertions; geometry and cursor icons only.

use egui::{
    Context, CursorIcon, Event, Modifiers, Panel, PointerButton, Pos2, RawInput, Rect, Ui,
    UiBuilder, Vec2, pos2, vec2,
};

const SCREEN: Vec2 = vec2(600.0, 400.0);

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
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> egui::FullOutput {
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

/// Show `ctor("logical-side")` for two frames (the second reads the persisted state)
/// and capture its outer rect. `Panel::start as fn(&'static str) -> Panel` and friends
/// coerce the generic constructors.
fn panel_rect(rtl: bool, ctor: fn(&'static str) -> Panel) -> Rect {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    let mut rect = Rect::NOTHING;
    for _ in 0..2 {
        run(&ctx, input(SCREEN), |ui| {
            rect = ctor("logical-side")
                .show(ui, |ui| {
                    ui.label("Panel");
                })
                .response
                .rect;
        });
    }
    rect
}

/// Show `ctor("anchor")` inside a nested `Ui` carved out of `area` (so the panel's
/// anchor edge is NOT the screen edge — the #7930 scenario), capturing its rect.
/// All frames run on the shared `ctx`.
fn show_in_area(ctx: &Context, ctor: fn(&'static str) -> Panel, area: Rect, rect: &mut Rect) {
    run(ctx, input(SCREEN), |ui| {
        let mut child = ui.new_child(UiBuilder::new().max_rect(area));
        *rect = ctor("anchor")
            .show(&mut child, |ui| {
                ui.label("Panel");
            })
            .response
            .rect;
    });
}

/// One drag of the separator, on the shared `ctx`: hover at `hover_x`, press, move
/// to `to_x`, release, then settle. `time` advances so events stay ordered. The
/// rect is captured on the final settle frame.
fn drag_separator(
    ctx: &Context,
    ctor: fn(&'static str) -> Panel,
    area: Rect,
    hover_x: f32,
    to_x: f32,
    time: &mut f64,
    rect: &mut Rect,
) {
    // Idle frame so the panel has a settled rect and a persisted size:
    show_in_area(ctx, ctor, area, rect);
    let y = rect.center().y;

    let steps = [
        vec![Event::PointerMoved(pos2(hover_x, y))],
        vec![pointer_press(pos2(hover_x, y), true)],
        vec![Event::PointerMoved(pos2(to_x, y))],
        vec![pointer_press(pos2(to_x, y), false)],
    ];
    for events in steps {
        *time += 0.1;
        run(ctx, input_at(SCREEN, *time, events), |ui| {
            let mut child = ui.new_child(UiBuilder::new().max_rect(area));
            ctor("anchor").show(&mut child, |ui| {
                ui.label("Panel");
            });
        });
    }

    // Settle past any discarded sizing pass, and capture:
    *time += 0.1;
    show_in_area(ctx, ctor, area, rect);
    *time += 0.1;
    show_in_area(ctx, ctor, area, rect);
}

/// A1: `Panel::start` lands at the reading start (left in LTR, right in RTL).
#[test]
fn start_panel_lands_at_the_reading_start() {
    let ltr = panel_rect(false, Panel::start);
    assert!(
        ltr.center().x < SCREEN.x / 2.0,
        "LTR: a start panel should sit on the left half of the screen, got {ltr:?}"
    );

    let rtl = panel_rect(true, Panel::start);
    assert!(
        rtl.center().x > SCREEN.x / 2.0,
        "RTL: a start panel should sit on the right half of the screen, got {rtl:?}"
    );
}

/// A1: `Panel::end` is the exact mirror of `Panel::start` in both directions.
#[test]
fn end_panel_mirrors_start() {
    // LTR: `end` sits on the right half, mirroring `start` about the screen centre:
    let ltr_start = panel_rect(false, Panel::start);
    let ltr_end = panel_rect(false, Panel::end);
    assert!(
        ltr_end.center().x > SCREEN.x / 2.0,
        "LTR: an end panel should sit on the right half of the screen, got {ltr_end:?}"
    );
    assert!(
        (ltr_end.center().x - (SCREEN.x - ltr_start.center().x)).abs() <= 1.0,
        "LTR: end should mirror start about the screen centre \
         (start {ltr_start:?}, end {ltr_end:?})"
    );
    assert!(
        (ltr_end.width() - ltr_start.width()).abs() <= 1.0,
        "LTR: mirrored panels should have equal widths \
         (start {ltr_start:?}, end {ltr_end:?})"
    );

    // RTL: `end` sits on the left half, mirroring `start` about the screen centre:
    let rtl_start = panel_rect(true, Panel::start);
    let rtl_end = panel_rect(true, Panel::end);
    assert!(
        rtl_end.center().x < SCREEN.x / 2.0,
        "RTL: an end panel should sit on the left half of the screen, got {rtl_end:?}"
    );
    assert!(
        (rtl_end.center().x - (SCREEN.x - rtl_start.center().x)).abs() <= 1.0,
        "RTL: end should mirror start about the screen centre \
         (start {rtl_start:?}, end {rtl_end:?})"
    );
    assert!(
        (rtl_end.width() - rtl_start.width()).abs() <= 1.0,
        "RTL: mirrored panels should have equal widths \
         (start {rtl_start:?}, end {rtl_end:?})"
    );
}

/// A2: the physical constructors never follow the reading direction.
#[test]
fn physical_constructors_do_not_follow_direction() {
    let left = panel_rect(true, Panel::left);
    assert!(
        left.center().x < SCREEN.x / 2.0,
        "RTL: a physical left panel must stay on the left, got {left:?}"
    );

    let right = panel_rect(true, Panel::right);
    assert!(
        right.center().x > SCREEN.x / 2.0,
        "RTL: a physical right panel must stay on the right, got {right:?}"
    );
}

/// A3: a `Panel::start` keeps its persisted width when the reading direction flips
/// between frames — only the anchor edge moves. Fails if the panel id were
/// direction-keyed.
#[test]
fn size_survives_a_direction_flip() {
    let ctx = Context::default();
    let mut rect = Rect::NOTHING;
    let show = |rect: &mut Rect| {
        run(&ctx, input(SCREEN), |ui| {
            *rect = Panel::start("flip")
                .resizable(true)
                .show(ui, |ui| {
                    ui.label("Panel");
                })
                .response
                .rect;
        });
    };

    // Two LTR frames: the panel settles, hugging the left screen edge:
    show(&mut rect);
    let ltr = rect;
    show(&mut rect);
    assert!(
        ltr.left() <= 1.0,
        "LTR: a start panel should hug the left screen edge, got {ltr:?}"
    );

    // Flip to RTL (takes effect from the next frame):
    ctx.set_right_to_left(true);
    show(&mut rect);
    let rtl = rect;
    show(&mut rect);
    assert!(
        SCREEN.x - rtl.right() <= 1.0,
        "RTL: a start panel should hug the right screen edge, got {rtl:?}"
    );
    assert!(
        (rtl.width() - ltr.width()).abs() <= 1.0,
        "the persisted size must survive the direction flip (LTR {ltr:?}, RTL {rtl:?})"
    );

    // Flip back:
    ctx.set_right_to_left(false);
    show(&mut rect);
    let back = rect;
    show(&mut rect);
    assert!(
        back.left() <= 1.0,
        "flipping back should re-anchor the panel on the left, got {back:?}"
    );
    assert!(
        (back.width() - ltr.width()).abs() <= 1.0,
        "the persisted size must survive the flip back (LTR {ltr:?}, back {back:?})"
    );
}

/// A4 (#7930, left side): with a `Panel::left` whose anchor edge sits away from the
/// screen edge (nested `Ui`), dragging the separator leftward past the anchor edge
/// clamps at `min_size` (96.0), and a continued deeper drag never grows the panel
/// again — the drag direction does not reverse.
#[test]
fn drag_past_the_anchor_edge_clamps_left() {
    let ctx = Context::default();
    // Panel area [300, 600]: anchor edge at x = 300, far from the screen edge.
    let area = Rect::from_min_size(pos2(300.0, 50.0), vec2(300.0, 200.0));
    let mut time = 0.0;
    let mut rect = Rect::NOTHING;

    show_in_area(&ctx, Panel::left, area, &mut rect);
    let hover_x = rect.right() - 1.0; // inside the 3.0-point grab strip

    // First drag, past the anchor edge: the old `.abs()` math would grow the
    // panel to 100.0; the signed math clamps to min_size:
    drag_separator(
        &ctx,
        Panel::left,
        area,
        hover_x,
        200.0,
        &mut time,
        &mut rect,
    );
    assert!(
        (rect.width() - 96.0).abs() <= 1.0,
        "dragging left past the anchor edge must clamp at min_size, got {rect:?}"
    );

    // Second drag, deeper (separator now at ≈ 396): still clamped. A reversed
    // drag would now measure `|50 - 300| = 250` and GROW the panel past its
    // default 200.0:
    let hover_x = rect.right() - 1.0;
    drag_separator(&ctx, Panel::left, area, hover_x, 50.0, &mut time, &mut rect);
    assert!(
        (rect.width() - 96.0).abs() <= 1.0,
        "a continued drag past the anchor edge must stay clamped at min_size, got {rect:?}"
    );
    assert!(
        rect.width() <= 200.0,
        "the drag must never grow the panel again while the pointer is past the \
         anchor edge, got {rect:?}"
    );
}

/// A5 (#7930, right side): symmetric case for `Panel::right` — dragging the
/// separator rightward past its right-edge anchor clamps at `min_size` and stays
/// there. All pointer x stay inside the 600-wide screen.
#[test]
fn drag_past_the_anchor_edge_clamps_right() {
    let ctx = Context::default();
    // Panel area [200, 400]: anchor edge at x = 400, far from the screen edge.
    let area = Rect::from_min_size(pos2(200.0, 50.0), vec2(200.0, 200.0));
    let mut time = 0.0;
    let mut rect = Rect::NOTHING;

    show_in_area(&ctx, Panel::right, area, &mut rect);
    let hover_x = rect.left() + 1.0; // inside the 3.0-point grab strip

    // First drag, past the anchor edge: the old `.abs()` math would grow the
    // panel to 150.0; the signed math clamps to min_size:
    drag_separator(
        &ctx,
        Panel::right,
        area,
        hover_x,
        550.0,
        &mut time,
        &mut rect,
    );
    assert!(
        (rect.width() - 96.0).abs() <= 1.0,
        "dragging right past the anchor edge must clamp at min_size, got {rect:?}"
    );

    // Second drag, deeper: still clamped. A reversed drag would measure
    // `|590 - 400| = 190` and grow the panel:
    let hover_x = rect.left() + 1.0;
    drag_separator(
        &ctx,
        Panel::right,
        area,
        hover_x,
        590.0,
        &mut time,
        &mut rect,
    );
    assert!(
        (rect.width() - 96.0).abs() <= 1.0,
        "a continued drag past the anchor edge must stay clamped at min_size, got {rect:?}"
    );
    assert!(
        rect.width() <= 200.0,
        "the drag must never grow the panel again while the pointer is past the \
         anchor edge, got {rect:?}"
    );
}

/// The cursor icon of an at-min-size panel's separator, with the pointer hovering
/// it. Two consecutive hover frames run and the second one's output is read (the
/// size update reads the previous frame's interact response).
fn separator_cursor(rtl: bool, ctor: fn(&'static str) -> Panel) -> CursorIcon {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }

    // Two frames: the second reads the persisted state, so the rect is settled.
    // `exact_size(96.0)` puts the panel at its minimum, so the cursor takes the
    // side-specific "can only grow" branch:
    let mut rect = Rect::NOTHING;
    for _ in 0..2 {
        run(&ctx, input(SCREEN), |ui| {
            rect = ctor("cursor-side")
                .exact_size(96.0)
                .show(ui, |ui| {
                    ui.label("Panel");
                })
                .response
                .rect;
        });
    }

    // A top-level panel hugs the screen edge it resolved to, and its separator sits
    // on the inner edge: the panel's right edge for a resolved-Left panel, its left
    // edge for a resolved-Right one. Deriving the hover position from the geometry
    // keeps the cursor assertion discriminating: if the side resolution is wrong
    // the hover lands on the real separator of the wrong side and the expected icon
    // does not match. (±1.0 lands inside the 3.0-point grab strip.)
    let resolved_right = (SCREEN.x - rect.right()).abs() <= 1.0;
    let y = rect.center().y;
    let hover_x = if resolved_right {
        rect.left() + 1.0
    } else {
        rect.right() - 1.0
    };

    // Hover frame, then a second hover frame whose output is the one read:
    run(
        &ctx,
        input_at(SCREEN, 0.0, vec![Event::PointerMoved(pos2(hover_x, y))]),
        |ui| {
            ctor("cursor-side").exact_size(96.0).show(ui, |ui| {
                ui.label("Panel");
            });
        },
    );
    let output = run(
        &ctx,
        input_at(SCREEN, 0.1, vec![Event::PointerMoved(pos2(hover_x, y))]),
        |ui| {
            ctor("cursor-side").exact_size(96.0).show(ui, |ui| {
                ui.label("Panel");
            });
        },
    );
    output.platform_output.cursor_icon
}

/// A6: the separator cursor follows the resolved side.
#[test]
fn cursor_icons_follow_the_resolved_side() {
    assert_eq!(
        separator_cursor(false, Panel::start),
        CursorIcon::ResizeEast,
        "LTR start panel: separator on the right edge, resizable eastward"
    );
    assert_eq!(
        separator_cursor(true, Panel::start),
        CursorIcon::ResizeWest,
        "RTL start panel: separator on the left edge, like an LTR end panel"
    );
    assert_eq!(
        separator_cursor(true, Panel::end),
        CursorIcon::ResizeEast,
        "RTL end panel: separator on the right edge, like an LTR start panel"
    );
    assert_eq!(
        separator_cursor(true, Panel::left),
        CursorIcon::ResizeEast,
        "the physical left constructor does not follow the direction"
    );
}
