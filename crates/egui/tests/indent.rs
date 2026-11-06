//! Regression tests for the reading-direction behaviour of [`egui::Ui::indent`]:
//! the indented child rect and the indent guide vline (plus the opt-in end stub)
//! follow the reading edge — an RTL-preferring vertical layout or
//! [`egui::Style::right_to_left`] mirrors today's left-to-right geometry.
//!
//! Every mirrored case also asserts its LTR counterpart (flag off), so this file
//! doubles as the LTR-unchanged regression. No snapshot assertions; geometry and
//! painted shapes only, all relative to rects captured inside the ui closures
//! (the default `CentralPanel` frame carries an 8 pt inner margin, so absolute
//! pixel expectations would be wrong by construction).

use egui::epaint::ClippedShape;
use egui::{
    Align, CentralPanel, Context, FullOutput, Layout, Pos2, RawInput, Rect, Ui, Vec2,
    collapsing_header::CollapsingHeader, epaint::Shape, vec2,
};

/// The screen size of every test frame.
const SCREEN: Vec2 = vec2(400.0, 300.0);

/// The default [`egui::Spacing::indent`].
const INDENT: f32 = 18.0;

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
    output
}

/// Every painted straight-line segment (the indent vline and its opt-in end stub
/// are the only ones in these minimal UIs; text and frames are meshes and rect shapes).
fn line_segments(shapes: &[ClippedShape]) -> Vec<[Pos2; 2]> {
    let mut segments = Vec::new();
    for clipped in shapes {
        if let Shape::LineSegment { points, .. } = &clipped.shape {
            segments.push(*points);
        }
    }
    segments
}

/// The x of every painted segment tall enough to be the indent vline
/// (the opt-in end stub is ~0 tall).
fn vertical_line_xs(shapes: &[ClippedShape]) -> Vec<f32> {
    line_segments(shapes)
        .into_iter()
        .filter(|points| (points[0].y - points[1].y).abs() > 8.0)
        .map(|points| points[0].x)
        .collect()
}

/// Show a single `ui.indent` in a `CentralPanel` under `layout`, capturing the
/// surrounding ui's and the indented child's `max_rect` plus the painted shapes.
fn indent_once(layout: Layout) -> (Rect, Rect, Vec<ClippedShape>) {
    let ctx = Context::default();
    let mut parent_max = Rect::NOTHING;
    let mut child_max = Rect::NOTHING;
    let output = run(&ctx, input(SCREEN), |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.with_layout(layout, |ui| {
                parent_max = ui.max_rect();
                ui.indent("indent", |ui| {
                    child_max = ui.max_rect();
                    ui.label("nested");
                });
            });
        });
    });
    (parent_max, child_max, output.shapes)
}

/// A1 + A2, layout-predicate path: under `top_down(Align::Max)` (which prefers
/// right-to-left) the indented child rect is inset from the parent's reading (right)
/// edge and the guide vline sits at the reading-edge icon column; under
/// `top_down(Align::Min)` both keep today's left-side geometry.
#[test]
fn indent_child_rect_and_vline_follow_reading_direction() {
    // RTL arm: the layout predicate alone mirrors, flag off.
    let (parent_max, child_max, shapes) = indent_once(Layout::top_down(Align::Max));
    assert!(
        (child_max.right() - (parent_max.right() - INDENT)).abs() < 1.0,
        "RTL: the child must be inset from the reading edge, got child {child_max:?} in {parent_max:?}"
    );
    let vline_xs = vertical_line_xs(&shapes);
    assert_eq!(vline_xs.len(), 1, "exactly one guide vline is painted");
    assert!(
        (vline_xs[0] - (parent_max.right() - INDENT / 2.0)).abs() < 1.0,
        "RTL: the vline must run through the reading-edge icon column, got x {} in {parent_max:?}",
        vline_xs[0]
    );

    // LTR counterpart: today's geometry, unchanged.
    let (parent_max, child_max, shapes) = indent_once(Layout::top_down(Align::Min));
    assert!(
        (child_max.left() - (parent_max.left() + INDENT)).abs() < 1.0,
        "LTR: the child must be inset from the left, got child {child_max:?} in {parent_max:?}"
    );
    let vline_xs = vertical_line_xs(&shapes);
    assert_eq!(vline_xs.len(), 1, "exactly one guide vline is painted");
    assert!(
        (vline_xs[0] - (parent_max.left() + INDENT / 2.0)).abs() < 1.0,
        "LTR: the vline must run through the icon column at the left, got x {} in {parent_max:?}",
        vline_xs[0]
    );
}

/// Show a single `ui.indent` inside a plain `ui.vertical` (`top_down(Align::Min)`)
/// in a `CentralPanel`, with the whole-UI direction flag set when `rtl` — before the
/// first pass, per the established rule: never inside a pass.
fn indent_vertical(rtl: bool) -> (Rect, Rect, Vec<ClippedShape>) {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let mut parent_max = Rect::NOTHING;
    let mut child_max = Rect::NOTHING;
    let output = run(&ctx, input(SCREEN), |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.vertical(|ui| {
                parent_max = ui.max_rect();
                ui.indent("indent", |ui| {
                    child_max = ui.max_rect();
                    ui.label("nested");
                });
            });
        });
    });
    (parent_max, child_max, output.shapes)
}

/// A3, flag path: with [`egui::Style::right_to_left`] set, a plain `ui.vertical`
/// indents from the reading edge too — indent is a direction-aware consumer of the
/// flag. Content alignment is deliberately not asserted: cross alignment stays with
/// the layout.
#[test]
fn indent_follows_the_style_flag() {
    // Flag on: the flag term of the settled predicate mirrors.
    let (parent_max, child_max, shapes) = indent_vertical(true);
    assert!(
        (child_max.right() - (parent_max.right() - INDENT)).abs() < 1.0,
        "flag on: the child must be inset from the reading edge, got child {child_max:?} in {parent_max:?}"
    );
    let vline_xs = vertical_line_xs(&shapes);
    assert_eq!(vline_xs.len(), 1, "exactly one guide vline is painted");
    assert!(
        (vline_xs[0] - (parent_max.right() - INDENT / 2.0)).abs() < 1.0,
        "flag on: the vline must run through the reading-edge icon column, got x {} in {parent_max:?}",
        vline_xs[0]
    );

    // LTR counterpart (flag off): unchanged geometry.
    let (parent_max, child_max, shapes) = indent_vertical(false);
    assert!(
        (child_max.left() - (parent_max.left() + INDENT)).abs() < 1.0,
        "flag off: the child must be inset from the left, got child {child_max:?} in {parent_max:?}"
    );
    let vline_xs = vertical_line_xs(&shapes);
    assert_eq!(vline_xs.len(), 1, "exactly one guide vline is painted");
    assert!(
        (vline_xs[0] - (parent_max.left() + INDENT / 2.0)).abs() < 1.0,
        "flag off: the vline must run through the icon column at the left, got x {} in {parent_max:?}",
        vline_xs[0]
    );
}

/// Show one default-open `CollapsingHeader` and capture the containing ui's and the
/// body's `max_rect` in a single pass. `default_open(true)` on a fresh context makes
/// the body run fully on the first frame, so no openness poll is needed — and a
/// `CollapsingState::load` poll would use the wrong id (the state id is minted on
/// `show_dyn`'s inner vertical wrapper, `outer.with("child").with("Section")`) and
/// never succeed. When `layout` is `Some`, the header runs under `ui.with_layout`.
fn collapsing_body(layout: Option<Layout>, rtl: bool) -> (Rect, Rect) {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let mut parent_max = Rect::NOTHING;
    let mut body_max = Rect::NOTHING;
    run(&ctx, input(SCREEN), |ui| {
        CentralPanel::default().show(ui, |ui| {
            let show = |ui: &mut Ui| {
                parent_max = ui.max_rect();
                let response = CollapsingHeader::new("Section")
                    .default_open(true)
                    .show(ui, |ui| {
                        body_max = ui.max_rect();
                    });
                assert!(
                    response.openness >= 1.0,
                    "the body must be fully open on the first pass"
                );
            };
            match layout {
                Some(layout) => ui.with_layout(layout, show),
                None => ui.vertical(show),
            }
        });
    });
    (parent_max, body_max)
}

/// A4: under the flag path (`set_right_to_left` + plain `ui.vertical`, where every
/// nested level is a vertical `Align::Min` body) the collapsing body indents from the
/// reading edge; under LTR from the left, unchanged. A third case pins the
/// pre-existing non-propagation of the layout-predicate path into collapsing bodies:
/// `show_dyn` wraps everything in a forced `top_down(Align::Min)` vertical, so under
/// `top_down(Align::Max)` with the flag off the body is NOT reading-edge inset.
#[test]
fn collapsing_header_body_indents_from_reading_edge() {
    // Flag-path RTL.
    let (parent_max, body_max) = collapsing_body(None, true);
    assert!(
        parent_max.right() - body_max.right() >= INDENT - 1.0,
        "flag on: the body must be inset from the reading edge, got body {body_max:?} in {parent_max:?}"
    );

    // LTR counterpart (flag off).
    let (parent_max, body_max) = collapsing_body(None, false);
    assert!(
        body_max.left() - parent_max.left() >= INDENT - 1.0,
        "flag off: the body must be inset from the left, got body {body_max:?} in {parent_max:?}"
    );

    // Guard: the layout-predicate path does not propagate into collapsing bodies
    // (forced `top_down(Align::Min)` inside `show_dyn`), so the body's right edge
    // stays at the available maximum.
    let (parent_max, body_max) = collapsing_body(Some(Layout::top_down(Align::Max)), false);
    assert!(
        parent_max.right() - body_max.right() <= 1.0,
        "the Max-layout path must not reading-edge indent the body, got body {body_max:?} in {parent_max:?}"
    );
}

/// Show one `ui.indent` with the opt-in end stub enabled, capturing the child's
/// `min_rect` after its content (matching the paint-time measurement) and the shapes.
fn stub_case(layout: Layout) -> (Rect, Vec<ClippedShape>) {
    let ctx = Context::default();
    let mut child_min = Rect::NOTHING;
    let output = run(&ctx, input(SCREEN), |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.with_layout(layout, |ui| {
                ui.spacing_mut().indent_ends_with_horizontal_line = true;
                ui.indent("indent", |ui| {
                    ui.label("nested");
                    child_min = ui.min_rect();
                });
            });
        });
    });
    (child_min, output.shapes)
}

/// A5: with `Spacing::indent_ends_with_horizontal_line`, the stub grows from the
/// vline foot toward the reading end — left in RTL, right in LTR.
#[test]
fn indent_ends_with_horizontal_line_mirrors() {
    // RTL arm: the layout-predicate path (`top_down(Align::Max)`).
    let (child_min, shapes) = stub_case(Layout::top_down(Align::Max));
    let verticals: Vec<[Pos2; 2]> = line_segments(&shapes)
        .into_iter()
        .filter(|points| (points[0].y - points[1].y).abs() > 8.0)
        .collect();
    let horizontals: Vec<[Pos2; 2]> = line_segments(&shapes)
        .into_iter()
        .filter(|points| (points[0].x - points[1].x).abs() > 8.0)
        .collect();
    assert_eq!(verticals.len(), 1, "exactly one guide vline is painted");
    assert_eq!(horizontals.len(), 1, "exactly one end stub is painted");
    let vline = verticals[0];
    let stub = horizontals[0];
    assert!(
        (stub[0].y - vline[0].y.max(vline[1].y)).abs() < 1e-3,
        "the stub must start at the vline's foot"
    );
    assert!(
        (stub[1].x - (child_min.left() + 2.0)).abs() < 1.0,
        "RTL: the stub must end at the child's reading-start edge plus the fudge, got x {} for child {child_min:?}",
        stub[1].x
    );
    assert!(
        stub[1].x < vline[0].x,
        "RTL: the stub must grow toward the reading end, left of the vline"
    );

    // LTR counterpart: today's geometry, unchanged.
    let (child_min, shapes) = stub_case(Layout::top_down(Align::Min));
    let verticals: Vec<[Pos2; 2]> = line_segments(&shapes)
        .into_iter()
        .filter(|points| (points[0].y - points[1].y).abs() > 8.0)
        .collect();
    let horizontals: Vec<[Pos2; 2]> = line_segments(&shapes)
        .into_iter()
        .filter(|points| (points[0].x - points[1].x).abs() > 8.0)
        .collect();
    assert_eq!(verticals.len(), 1, "exactly one guide vline is painted");
    assert_eq!(horizontals.len(), 1, "exactly one end stub is painted");
    let vline = verticals[0];
    let stub = horizontals[0];
    assert!(
        (stub[0].y - vline[0].y.max(vline[1].y)).abs() < 1e-3,
        "the stub must start at the vline's foot"
    );
    assert!(
        (stub[1].x - (child_min.right() - 2.0)).abs() < 1.0,
        "LTR: the stub must end at the child's right edge minus the fudge, got x {} for child {child_min:?}",
        stub[1].x
    );
    assert!(
        stub[1].x > vline[0].x,
        "LTR: the stub must grow toward the reading end, right of the vline"
    );
}
