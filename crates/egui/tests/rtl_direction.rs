//! Regression tests for the whole-UI direction flag:
//! [`egui::Style::right_to_left`], [`egui::Context::set_right_to_left`],
//! [`egui::Context::is_rtl`] and [`egui::Ui::is_rtl`].
//!
//! Every mirrored case also asserts its LTR counterpart (flag off), so this file
//! doubles as the LTR-unchanged regression. No snapshot assertions; geometry and
//! painted shapes only.

use egui::epaint::{CircleShape, PathShape, RectShape};
use egui::{
    Align, Context, Event, FullOutput, Layout, Modifiers, PointerButton, Pos2, RawInput, Rect,
    Theme, Ui, UiBuilder, Vec2, collapsing_header::CollapsingState, epaint::Shape, pos2, vec2,
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

fn click(ctx: &Context, size: Vec2, pos: Pos2, mut show_ui: impl FnMut(&mut Ui)) {
    run(
        ctx,
        input_at(size, 1.0, vec![Event::PointerMoved(pos)]),
        |ui| show_ui(ui),
    );
    run(
        ctx,
        input_at(size, 1.1, vec![pointer_press(pos, true)]),
        |ui| show_ui(ui),
    );
    run(
        ctx,
        input_at(size, 1.2, vec![pointer_press(pos, false)]),
        |ui| show_ui(ui),
    );
}

/// Show one collapsing header (icon-button variant), capturing the toggle response,
/// the header row, the optional body, and the header's id.
struct HeaderUi {
    id: egui::Id,
    toggle: Rect,
    header: Rect,
    body: Option<Rect>,
}

/// Draw one collapsing header (icon-button variant) into the given `Ui`,
/// capturing the toggle response, the header row, the optional body, and the id.
fn draw_header(ui: &mut Ui, result: &mut HeaderUi) {
    let id = ui.make_persistent_id("rtl-section");
    let (toggle, header, body) = CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.label("Header");
        })
        .body(|ui| {
            ui.label("Body");
        });
    result.id = id;
    result.toggle = toggle.rect;
    result.header = header.response.rect;
    result.body = body.map(|body| body.response.rect);
}

fn show_header(ctx: &Context, size: Vec2, result: &mut HeaderUi) {
    run(ctx, input(size), |ui| draw_header(ui, result));
}

/// The direction the painted disclosure triangle points, as the vector from the
/// midpoint of its base to its apex — or `None` if not exactly one filled 3-gon was
/// painted. A UI must contain only the collapsing header, so that 3-gon is the icon.
///
/// The apex is the vertex opposite the SHORTEST edge: `paint_default_icon` receives
/// the small icon square (`Spacing::icon_width_inner` = 8.0) scaled by 0.75 to a
/// 6.0 x 6.0 rect, so the triangle's base (6.0) is strictly shorter than its two
/// sides (≈ 6.7) at every rotation, and rotation preserves edge lengths.
fn triangle_direction(shapes: &[egui::epaint::ClippedShape]) -> Option<Vec2> {
    let mut triangles: Vec<Vec<Pos2>> = Vec::new();
    for clipped in shapes {
        if let Shape::Path(PathShape { points, .. }) = &clipped.shape
            && points.len() == 3
        {
            triangles.push(points.clone());
        }
    }
    if triangles.len() != 1 {
        return None;
    }

    let points = &triangles[0];
    let (p0, p1, p2) = (points[0], points[1], points[2]);
    let (a, b, apex) = {
        let d01 = p0.distance(p1);
        let d12 = p1.distance(p2);
        let d20 = p2.distance(p0);
        if d01 <= d12 && d01 <= d20 {
            (p0, p1, p2)
        } else if d12 <= d20 {
            (p1, p2, p0)
        } else {
            (p2, p0, p1)
        }
    };
    Some(apex.to_vec2() - (a.to_vec2() + b.to_vec2()) * 0.5)
}

/// A collapsed header, with its painted triangle direction (from the icon apex to the
/// midpoint of its base) if a triangle was painted.
struct TriangleUi {
    direction: Option<Vec2>,
}

fn show_header_triangle(ctx: &Context, size: Vec2, time: f64) -> TriangleUi {
    let output = run(ctx, input_at(size, time, vec![]), |ui| {
        let id = ui.make_persistent_id("rtl-section");
        CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.label("Header");
            })
            .body(|ui| {
                ui.label("Body");
            });
    });

    TriangleUi {
        direction: triangle_direction(&output.shapes),
    }
}

// ---------------------------------------------------------------------------

/// A1: the flag is settable and readable, on both theme branches, survives a
/// `set_theme` round-trip, and defaults to `false`.
#[test]
fn flag_is_settable_on_both_theme_branches() {
    let ctx = Context::default();
    assert!(!ctx.is_rtl(), "default is left-to-right");

    ctx.set_right_to_left(true);
    assert!(ctx.is_rtl());
    for theme in [Theme::Light, Theme::Dark] {
        assert!(
            ctx.style_of(theme).right_to_left,
            "{theme:?} branch should mirror"
        );
    }

    // An appearance switch must not revert the direction:
    ctx.set_theme(Theme::Light);
    assert!(ctx.is_rtl(), "survives set_theme(Light)");
    ctx.set_theme(Theme::Dark);
    assert!(ctx.is_rtl(), "survives set_theme(Dark)");

    // And `Ui::is_rtl` agrees with the context:
    let mut ui_is_rtl: Option<bool> = None;
    run(&ctx, input(vec2(400.0, 300.0)), |ui| {
        ui_is_rtl = Some(ui.is_rtl());
    });
    assert_eq!(ui_is_rtl, Some(true));

    // A per-`Ui` override works, independently of the global flag:
    ctx.set_right_to_left(false);
    run(&ctx, input(vec2(400.0, 300.0)), |ui| {
        assert!(!ui.is_rtl());
        ui.style_mut().right_to_left = true;
        assert!(ui.is_rtl());
    });
    assert!(
        !ctx.is_rtl(),
        "a per-Ui override must not leak into the context"
    );

    // Unset again clears both theme branches:
    ctx.set_right_to_left(false);
    for theme in [Theme::Light, Theme::Dark] {
        assert!(
            !ctx.style_of(theme).right_to_left,
            "{theme:?} branch should be LTR again"
        );
    }
}

/// A2: a nested `ui.horizontal` inside a vertical `Align::Min` body mirrors under RTL
/// (its first widget is placed at the row's right edge), and stays left-anchored under LTR.
#[test]
fn nested_rows_mirror_in_vertical_bodies() {
    fn run_rows(rtl: bool) -> (Rect, Rect, Rect) {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let mut first = Rect::NOTHING;
        let mut row = Rect::NOTHING;
        let mut body = Rect::NOTHING;
        run(&ctx, input(vec2(400.0, 300.0)), |ui| {
            let body_response = ui.vertical(|ui| {
                let row_response = ui.horizontal(|ui| {
                    first = ui.label("First").rect;
                    ui.label("Second");
                });
                row = row_response.response.rect;
            });
            body = body_response.response.rect;
        });
        (first, row, body)
    }

    let (rtl_first, _rtl_row, rtl_body) = run_rows(true);
    assert!(
        (rtl_first.right() - rtl_body.right()).abs() <= 1.0,
        "RTL: the row's first widget should hug the body's right edge \
         (first {rtl_first:?}, body {rtl_body:?})"
    );

    let (ltr_first, _ltr_row, ltr_body) = run_rows(false);
    assert!(
        (ltr_first.left() - ltr_body.left()).abs() <= 1.0,
        "LTR: the row's first widget should hug the body's left edge \
         (first {ltr_first:?}, body {ltr_body:?})"
    );
}

/// A4: the checkbox and radio icons swap to the reading-start (right) side under RTL,
/// verified through the painted shapes.
#[test]
fn checkbox_and_radio_icons_swap_sides() {
    /// Paint a single checkbox with empty text and return its rect and the rect of the
    /// painted icon box.
    fn painted_checkbox_icon(rtl: bool) -> (Rect, Rect) {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let mut widget = Rect::NOTHING;
        let output = run(&ctx, input(vec2(200.0, 100.0)), |ui| {
            let mut checked = false;
            let response = ui.add(egui::Checkbox::new(&mut checked, ""));
            widget = response.rect;
        });
        let icons: Vec<Rect> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Rect(RectShape { rect, .. }) => Some(*rect),
                _ => None,
            })
            .filter(|rect| {
                rect.is_positive()
                    && widget.contains_rect(*rect)
                    && rect.width() < widget.width() - 1.0
            })
            .collect();
        assert_eq!(
            icons.len(),
            1,
            "expected exactly one painted checkbox box (rtl = {rtl}), got {icons:?}"
        );
        (widget, icons[0])
    }

    let (ltr_widget, ltr_icon) = painted_checkbox_icon(false);
    assert!(
        ltr_icon.center().x < ltr_widget.center().x,
        "LTR: the checkbox icon should be painted on the left of the widget \
         (icon {ltr_icon:?}, widget {ltr_widget:?})"
    );

    let (rtl_widget, rtl_icon) = painted_checkbox_icon(true);
    assert!(
        rtl_icon.center().x > rtl_widget.center().x,
        "RTL: the checkbox icon should be painted on the right of the widget \
         (icon {rtl_icon:?}, widget {rtl_widget:?})"
    );

    /// Same for the radio button, whose icon is a circle.
    fn painted_radio_icon(rtl: bool) -> (Rect, Pos2) {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let mut widget = Rect::NOTHING;
        let output = run(&ctx, input(vec2(200.0, 100.0)), |ui| {
            let response = ui.add(egui::RadioButton::new(false, ""));
            widget = response.rect;
        });
        let icons: Vec<Pos2> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Circle(CircleShape { center, .. }) => Some(*center),
                _ => None,
            })
            .filter(|center| widget.contains(*center))
            .collect();
        assert_eq!(
            icons.len(),
            1,
            "expected exactly one painted radio circle (rtl = {rtl}), got {icons:?}"
        );
        (widget, icons[0])
    }

    let (ltr_widget, ltr_icon) = painted_radio_icon(false);
    assert!(
        ltr_icon.x < ltr_widget.center().x,
        "LTR: the radio icon should be painted on the left of the widget"
    );

    let (rtl_widget, rtl_icon) = painted_radio_icon(true);
    assert!(
        rtl_icon.x > rtl_widget.center().x,
        "RTL: the radio icon should be painted on the right of the widget"
    );
}

/// A3: the collapsing-header disclosure icon sits at the reading-start edge of the
/// header row under RTL (left edge under LTR), and toggling still opens the body.
#[test]
fn collapsing_header_icon_sits_at_reading_start_edge() {
    // Closed state on the first pass:
    let mut rtl = HeaderUi {
        id: egui::Id::NULL,
        toggle: Rect::NOTHING,
        header: Rect::NOTHING,
        body: None,
    };
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    show_header(&ctx, vec2(400.0, 300.0), &mut rtl);
    assert!(
        (rtl.toggle.right() - rtl.header.right()).abs() <= 1.0,
        "RTL: the disclosure icon should hug the header's reading-start (right) edge \
         (toggle {:?}, header {:?})",
        rtl.toggle,
        rtl.header
    );
    assert!(rtl.body.is_none(), "the header starts closed");

    let mut ltr = HeaderUi {
        id: egui::Id::NULL,
        toggle: Rect::NOTHING,
        header: Rect::NOTHING,
        body: None,
    };
    let ltr_ctx = Context::default();
    show_header(&ltr_ctx, vec2(400.0, 300.0), &mut ltr);
    assert!(
        (ltr.toggle.left() - ltr.header.left()).abs() <= 1.0,
        "LTR: the disclosure icon should hug the header's left edge \
         (toggle {:?}, header {:?})",
        ltr.toggle,
        ltr.header
    );

    // Clicking the icon still opens the body under RTL:
    show_header(&ctx, vec2(400.0, 300.0), &mut rtl);
    let toggle_center = rtl.toggle.center();
    click(&ctx, vec2(400.0, 300.0), toggle_center, |ui| {
        draw_header(ui, &mut rtl);
    });

    let mut body_open = false;
    for frame in 0..20 {
        show_header(&ctx, vec2(400.0, 300.0), &mut rtl);
        if rtl.body.is_some() {
            body_open = true;
        }
        let openness = CollapsingState::load(&ctx, rtl.id).map_or(0.0, |s| s.openness(&ctx));
        if openness >= 1.0 {
            break;
        }
        let _ = frame;
    }
    assert!(
        body_open,
        "clicking the disclosure icon under RTL should open the body"
    );
}

/// A3: the closed disclosure triangle points at the reading start (left under RTL,
/// right under LTR) and the fully-open triangle points down in both directions.
/// This pins the mirrored `closed_rotation` sweep of `paint_default_icon`.
#[test]
fn triangle_apex_points_reading_start_when_closed() {
    // Closed state (the default, openness 0):
    let rtl_ctx = Context::default();
    rtl_ctx.set_right_to_left(true);
    let rtl = show_header_triangle(&rtl_ctx, vec2(400.0, 300.0), 0.0);
    let rtl_dir = rtl
        .direction
        .expect("a closed header always paints its triangle");
    assert!(
        rtl_dir.x < 0.0 && rtl_dir.y.abs() <= 0.2 * rtl_dir.x.abs(),
        "RTL: the closed triangle should point left (horizontally), got {rtl_dir:?}"
    );

    let ltr_ctx = Context::default();
    let ltr = show_header_triangle(&ltr_ctx, vec2(400.0, 300.0), 0.0);
    let ltr_dir = ltr
        .direction
        .expect("a closed header always paints its triangle");
    assert!(
        ltr_dir.x > 0.0 && ltr_dir.y.abs() <= 0.2 * ltr_dir.x.abs(),
        "LTR: the closed triangle should point right (horizontally), got {ltr_dir:?}"
    );

    // Open state: the triangle must point straight down in BOTH directions, guarding
    // the sweep's open endpoint (openness 1.0 = rotation 0.0).
    for (rtl, ctx) in [(true, &rtl_ctx), (false, &ltr_ctx)] {
        let size = vec2(400.0, 300.0);
        let mut state = HeaderUi {
            id: egui::Id::NULL,
            toggle: Rect::NOTHING,
            header: Rect::NOTHING,
            body: None,
        };
        show_header(ctx, size, &mut state);
        let toggle_center = state.toggle.center();
        click(ctx, size, toggle_center, |ui| {
            draw_header(ui, &mut state);
        });

        let mut dir = None;
        for frame in 0..20 {
            let openness = CollapsingState::load(ctx, state.id).map_or(0.0, |s| s.openness(ctx));
            if openness >= 1.0 {
                dir = show_header_triangle(ctx, size, 2.0 + frame as f64 * 0.1).direction;
                break;
            }
            show_header(ctx, size, &mut state);
        }

        let dir = dir.expect("the fully open header should paint its triangle");
        assert!(
            dir.y > 0.0 && dir.x.abs() <= 0.2 * dir.y.abs(),
            "the open triangle should point down in both directions (rtl = {rtl}), got {dir:?}"
        );
    }
}

/// A7: the sizing pass measures a right-to-left cross-`Align::Max` body anchored on the
/// reading-start edge: `min_rect().left()` equals the offered rect's left edge (the same
/// measurement an explicit `Align::Min` body gets) and is offered-width-independent.
#[test]
fn sizing_pass_measures_from_reading_start_edge() {
    fn measure(rtl: bool, cross_align: Align, offered_width: f32) -> Rect {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let offered = Rect::from_min_size(pos2(50.0, 50.0), vec2(offered_width, 300.0));
        let mut min_rect = Rect::NOTHING;
        run(&ctx, input(vec2(1000.0, 600.0)), |ui| {
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(offered)
                    .layout(Layout::top_down_justified(cross_align))
                    .sizing_pass(),
            );
            child.label("A label of some width");
            min_rect = child.min_rect();
        });
        min_rect
    }

    // RTL: the sizing measurement is anchored on the reading start and does not depend
    // on the offered width:
    let rtl_narrow = measure(true, Align::Max, 400.0);
    let rtl_wide = measure(true, Align::Max, 800.0);
    assert!(
        (rtl_narrow.left() - 50.0).abs() <= 0.5,
        "RTL: the cross-Max sizing pass should anchor on the reading-start (left) edge, \
         got min_rect {rtl_narrow:?}"
    );
    assert!(
        (rtl_narrow.left() - rtl_wide.left()).abs() <= 0.5
            && (rtl_narrow.right() - rtl_wide.right()).abs() <= 0.5
            && (rtl_narrow.top() - rtl_wide.top()).abs() <= 0.5
            && (rtl_narrow.bottom() - rtl_wide.bottom()).abs() <= 0.5,
        "RTL: the sizing measurement must be offered-width-independent \
         (narrow {rtl_narrow:?}, wide {rtl_wide:?})"
    );

    // ...and equals the measurement of an explicit Align::Min body:
    let rtl_min = measure(true, Align::Min, 400.0);
    assert!(
        (rtl_narrow.left() - rtl_min.left()).abs() <= 0.5,
        "RTL: cross-Max sizing should measure like cross-Min \
         (Max {rtl_narrow:?}, Min {rtl_min:?})"
    );

    // LTR counterpart (flag off): the cross-Max sizing measurement stays right-anchored
    // in the offered rect, exactly as before:
    let ltr = measure(false, Align::Max, 400.0);
    let offered_right = 50.0 + 400.0;
    assert!(
        (ltr.left() - (offered_right - ltr.width())).abs() <= 0.5,
        "LTR: the cross-Max sizing measurement should stay reading-end anchored, got {ltr:?}"
    );
}

/// A8: the `Resize` container anchors on the reading-start edge under RTL
/// (grip in the bottom-left corner, drags along the reading direction grow the rect,
/// the fixed reading-start edge stays put), and the `min_content_width` floor is
/// direction-independent.
#[test]
fn resize_container_anchors_on_reading_start_edge() {
    const SCREEN: Vec2 = vec2(400.0, 300.0);
    const DEFAULT_SIZE: Vec2 = vec2(200.0, 100.0);

    fn resize_id() -> egui::Id {
        egui::Id::unique("rtl_resize_test")
    }

    fn corner_id() -> egui::Id {
        resize_id().with("__resize_corner")
    }

    struct Snapshot {
        container: Rect,
        grip: Rect,
    }

    /// Run one frame, capturing the container rect (from the painted stroke, which is
    /// the container rect expanded by 2.0) and the grip rect.
    fn frame(ctx: &Context, events: Vec<Event>, time: f64) -> Snapshot {
        let mut snapshot = Snapshot {
            container: Rect::NOTHING,
            grip: Rect::NOTHING,
        };
        let output = run(ctx, input_at(SCREEN, time, events), |ui| {
            egui::Resize::default()
                .id(resize_id())
                .default_size(DEFAULT_SIZE)
                .show(ui, |ui| {
                    ui.label("Content");
                });
            if let Some(corner) = ctx.read_response(corner_id()) {
                snapshot.grip = corner.rect;
            }
        });
        for clipped in &output.shapes {
            if let Shape::Rect(RectShape {
                rect, fill, stroke, ..
            }) = &clipped.shape
                && *fill == egui::Color32::TRANSPARENT
                && !stroke.is_empty()
            {
                // The `Resize` stroke is the container rect expanded by 2.0 per side:
                snapshot.container = rect.shrink(2.0);
            }
        }
        snapshot
    }

    /// Drag the grip by `delta` (while held), release, settle, and return the final snapshot.
    fn drag(rtl: bool, delta: Vec2) -> Snapshot {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let idle = frame(&ctx, vec![], 0.0);
        assert!(
            idle.grip.is_positive(),
            "no corner grip response captured (rtl = {rtl})"
        );
        let grip_center = idle.grip.center();

        frame(&ctx, vec![Event::PointerMoved(grip_center)], 1.0);
        frame(&ctx, vec![pointer_press(grip_center, true)], 1.1);
        let moved = grip_center + delta;
        frame(&ctx, vec![Event::PointerMoved(moved)], 1.2);
        frame(&ctx, vec![pointer_press(moved, false)], 1.3);
        // Settle past any discarded sizing pass:
        let mut final_snapshot = frame(&ctx, vec![], 1.4);
        if !final_snapshot.container.is_positive() {
            final_snapshot = frame(&ctx, vec![], 1.5);
        }
        final_snapshot
    }

    // The grip must be inside the container in both directions:
    for rtl in [false, true] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let idle = frame(&ctx, vec![], 0.0);
        assert!(
            idle.container.is_positive() && idle.grip.is_positive(),
            "the idle frame should paint the container and the grip (rtl = {rtl})"
        );
        assert!(
            idle.container.contains_rect(idle.grip),
            "the corner grip must stay inside the container (rtl = {rtl}, \
             container {:?}, grip {:?})",
            idle.container,
            idle.grip
        );
    }

    // LTR: anchored at the reading start (left edge), grows rightward:
    let ltr_idle = {
        let ctx = Context::default();
        frame(&ctx, vec![], 0.0)
    };
    let ltr_dragged = drag(false, vec2(50.0, 0.0));
    assert!(
        (ltr_dragged.container.left() - ltr_idle.container.left()).abs() <= 1.0,
        "LTR: the reading-start (left) edge must stay fixed \
         (idle {:?}, dragged {:?})",
        ltr_idle.container,
        ltr_dragged.container
    );
    assert!(
        ltr_dragged.container.right() > ltr_idle.container.right() + 25.0,
        "LTR: dragging the grip along the reading direction must grow the rect \
         (idle {:?}, dragged {:?})",
        ltr_idle.container,
        ltr_dragged.container
    );

    // RTL: anchored at the reading start (right edge), grows leftward:
    let rtl_idle = {
        let ctx = Context::default();
        ctx.set_right_to_left(true);
        frame(&ctx, vec![], 0.0)
    };
    assert!(
        (rtl_idle.container.right() - SCREEN.x).abs() <= 1.0,
        "RTL: the reading-start (right) edge should sit at the offered right edge, got {:?}",
        rtl_idle.container
    );
    let rtl_dragged = drag(true, vec2(-50.0, 0.0));
    assert!(
        (rtl_dragged.container.right() - rtl_idle.container.right()).abs() <= 1.0,
        "RTL: the reading-start (right) edge must stay fixed \
         (idle {:?}, dragged {:?})",
        rtl_idle.container,
        rtl_dragged.container
    );
    assert!(
        rtl_dragged.container.left() < rtl_idle.container.left() - 25.0,
        "RTL: dragging the grip along the reading direction must grow the rect \
         (idle {:?}, dragged {:?})",
        rtl_idle.container,
        rtl_dragged.container
    );

    // A7 guard: over-shrinking clamps to the same content floor in both directions.
    // LTR: drag far left; RTL: drag far right (the shrinking directions).
    let ltr_min = drag(false, vec2(-500.0, 0.0));
    let rtl_min = drag(true, vec2(500.0, 0.0));
    assert!(
        (ltr_min.container.width() - rtl_min.container.width()).abs() <= 2.0,
        "the min-content floor must be direction-independent \
         (LTR {:?}, RTL {:?})",
        ltr_min.container,
        rtl_min.container
    );
    assert!(
        ltr_min.container.width() < DEFAULT_SIZE.x - 50.0,
        "the drag should actually have shrunk the container below its default size \
         (LTR {:?}, RTL {:?})",
        ltr_min.container,
        rtl_min.container
    );
}

/// The resize grip cursor mirrors with the grip side (A8).
#[test]
fn resize_grip_cursor_mirrors() {
    for (rtl, expected) in [
        (false, egui::CursorIcon::ResizeNwSe),
        (true, egui::CursorIcon::ResizeNeSw),
    ] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let size = vec2(400.0, 300.0);
        let id = egui::Id::unique("rtl_resize_cursor_test");

        // Lay out once so the grip exists, then hover it:
        let mut grip_center = Pos2::ZERO;
        run(&ctx, input(size), |ui| {
            egui::Resize::default()
                .id(id)
                .show(ui, |ui| ui.label("Content"));
            if let Some(corner) = ctx.read_response(id.with("__resize_corner")) {
                grip_center = corner.rect.center();
            }
        });
        run(
            &ctx,
            input_at(size, 0.0, vec![Event::PointerMoved(grip_center)]),
            |ui| {
                egui::Resize::default()
                    .id(id)
                    .show(ui, |ui| ui.label("Content"));
            },
        );

        let output = run(
            &ctx,
            input_at(size, 0.1, vec![Event::PointerMoved(grip_center)]),
            |ui| {
                egui::Resize::default()
                    .id(id)
                    .show(ui, |ui| ui.label("Content"));
            },
        );
        assert_eq!(
            output.platform_output.cursor_icon, expected,
            "rtl = {rtl}: the grip cursor should mirror with the grip side"
        );
    }
}
