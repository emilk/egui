//! Regression tests for slider and progress-bar right-to-left mirroring: the horizontal
//! slider rail, the slider/range-slider fill and the progress-bar fill grow from the
//! reading-start (right) edge under a right-to-left reading direction, while the pointer
//! keeps its physical mapping.
//!
//! Both mirroring mechanisms are exercised: [`egui::Context::set_right_to_left`] (the
//! interface-direction flag) and [`egui::Layout::right_to_left`] with the flag off. The
//! flag is set before the first frame, so it reaches the first mirrored pass. Every
//! mirrored case also asserts its left-to-right counterpart, so this file doubles as the
//! LTR-unchanged regression. No snapshot assertions; geometry and painted shapes only.
//!
//! The sliders key their chrome on either mechanism. The progress bar is widget chrome
//! that follows the interface reading direction only (the flag), so under an explicit
//! right-to-left layout with the flag off it keeps its left-to-right paint.

use egui::epaint::{RectShape, Shape};
use egui::{
    Align, Color32, Context, CursorIcon, DragValue, Event, FullOutput, Layout, Modifiers,
    PointerButton, Pos2, ProgressBar, RangeSlider, RawInput, Rect, Slider, Ui, Vec2, pos2, vec2,
};

/// Which mirroring mechanism a test case runs under.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Dir {
    /// No flag, default layout: today's behaviour, byte-identical.
    Ltr,

    /// The interface-direction flag, set before the first frame, in a plain body.
    FlagRtl,

    /// An explicit right-to-left layout with the flag off.
    LayoutRtl,
}

fn make_ctx(dir: Dir) -> Context {
    let ctx = Context::default();
    if dir == Dir::FlagRtl {
        // Set before the first frame, so the flag reaches the first mirrored pass.
        ctx.set_right_to_left(true);
    }
    ctx
}

/// Run `add_contents`, wrapped in an explicit right-to-left layout for [`Dir::LayoutRtl`].
fn rtl_layout<R>(ui: &mut Ui, dir: Dir, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    if dir == Dir::LayoutRtl {
        ui.with_layout(Layout::right_to_left(Align::TOP), add_contents)
            .inner
    } else {
        add_contents(ui)
    }
}

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

/// All rects painted in `color` that sit inside `widget`.
fn rects_in(output: &FullOutput, widget: Rect, color: Color32) -> Vec<Rect> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(RectShape { rect, fill, .. })
                if *fill == color && widget.contains_rect(*rect) =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .collect()
}

/// The painted rail: the `widgets.inactive.bg_fill` rect that spans the widget's width.
///
/// The rail is as wide as the allocation; the handle rects in the same colour are narrow.
fn rail_of(output: &FullOutput, widget: Rect, rail_color: Color32) -> Rect {
    let rails: Vec<Rect> = rects_in(output, widget, rail_color)
        .into_iter()
        .filter(|rect| (rect.width() - widget.width()).abs() < 0.5)
        .collect();
    assert_eq!(rails.len(), 1, "expected exactly one rail, got {rails:?}");
    rails[0]
}

/// Under either mirroring mechanism, a slider's trailing fill hugs the rail's
/// reading-start (right) edge; left-to-right keeps hugging the left edge.
#[test]
fn slider_fill_hugs_the_reading_start_edge() {
    fn painted_slider(dir: Dir) -> (Rect, Rect, Rect) {
        let ctx = make_ctx(dir);
        let screen = vec2(400.0, 120.0);
        let mut value = 0.25_f64;
        let mut widget = Rect::NOTHING;
        let mut colors = (Color32::BLACK, Color32::BLACK);
        let output = run(&ctx, input(screen), |ui| {
            colors = (
                ui.visuals().selection.bg_fill,
                ui.visuals().widgets.inactive.bg_fill,
            );
            widget = rtl_layout(ui, dir, |ui| {
                ui.add(
                    Slider::new(&mut value, 0.0..=1.0)
                        .trailing_fill(true)
                        .show_value(false),
                )
                .rect
            });
        });
        let rail = rail_of(&output, widget, colors.1);
        let fills = rects_in(&output, widget, colors.0);
        assert_eq!(
            fills.len(),
            1,
            "expected exactly one fill (dir = {dir:?}), got {fills:?}"
        );
        (widget, rail, fills[0])
    }

    for dir in [Dir::Ltr, Dir::FlagRtl, Dir::LayoutRtl] {
        let (widget, rail, fill) = painted_slider(dir);
        assert!(
            rail.contains_rect(fill),
            "{dir:?}: the fill should sit on the rail (fill {fill:?}, rail {rail:?})"
        );
        assert!(
            fill.width() < rail.width(),
            "{dir:?}: 25 % should not fill the whole rail (fill {fill:?}, rail {rail:?})"
        );
        match dir {
            Dir::Ltr => {
                assert_eq!(
                    fill.left(),
                    rail.left(),
                    "LTR: the fill should hug the rail's left edge (fill {fill:?}, rail {rail:?})"
                );
                assert!(
                    fill.right() < rail.center().x,
                    "LTR: 25 % should stay on the left half (fill {fill:?}, rail {rail:?})"
                );
            }
            Dir::FlagRtl | Dir::LayoutRtl => {
                assert_eq!(
                    fill.right(),
                    rail.right(),
                    "RTL: the fill should hug the rail's right edge (fill {fill:?}, rail {rail:?})"
                );
                assert!(
                    fill.left() > rail.center().x,
                    "RTL: 25 % should sit on the right half (fill {fill:?}, rail {rail:?})"
                );
            }
        }
        let _ = widget;
    }
}

/// The same physical press x picks ~75 % of the range left-to-right and ~25 %
/// right-to-left: the value↔x map mirrors, the pointer is not remapped.
#[test]
fn slider_pointer_maps_physical_x_through_the_mirrored_range() {
    fn press_at(rtl: bool) -> f64 {
        let ctx = make_ctx(if rtl { Dir::FlagRtl } else { Dir::Ltr });
        let screen = vec2(400.0, 120.0);
        let mut value = 0.0_f64;
        let mut widget = Rect::NOTHING;
        // Frame 1 lays the widget out and captures where its rail sits:
        run(&ctx, input(screen), |ui| {
            widget = ui
                .add(
                    Slider::new(&mut value, 0.0..=100.0)
                        .smart_aim(false)
                        .show_value(false),
                )
                .rect;
        });
        // Press at 75 % across the widget: the same physical x in both directions.
        let press = pos2(widget.left() + 0.75 * widget.width(), widget.center().y);
        click(&ctx, screen, press, |ui| {
            ui.add(
                Slider::new(&mut value, 0.0..=100.0)
                    .smart_aim(false)
                    .show_value(false),
            );
        });
        value
    }

    let ltr = press_at(false);
    assert!(
        70.0 < ltr && ltr < 80.0,
        "LTR: a press 75 % across the rail should pick ~75 % of the range, got {ltr}"
    );

    let rtl = press_at(true);
    assert!(
        20.0 < rtl && rtl < 30.0,
        "RTL: the same physical press should pick ~25 % of the range, got {rtl}"
    );
}

/// The progress fill hugs the reading-start edge under the interface flag (its chrome
/// follows the flag, not the local layout), spans progress × bar width, fills the whole bar
/// at 1.0, and at 0.0 only the corner-radius cap remains, at the reading-start edge.
#[test]
fn progress_fill_hugs_the_reading_start_edge_and_clamps() {
    fn painted_progress(dir: Dir, progress: f32) -> (Rect, Rect) {
        let ctx = make_ctx(dir);
        let screen = vec2(400.0, 120.0);
        let mut widget = Rect::NOTHING;
        let mut fill_color = Color32::BLACK;
        let output = run(&ctx, input(screen), |ui| {
            fill_color = ui.visuals().selection.bg_fill;
            widget = rtl_layout(ui, dir, |ui| ui.add(ProgressBar::new(progress)).rect);
        });
        let fills = rects_in(&output, widget, fill_color);
        assert_eq!(
            fills.len(),
            1,
            "expected exactly one fill (dir = {dir:?}, progress = {progress}), got {fills:?}"
        );
        (widget, fills[0])
    }

    for dir in [Dir::Ltr, Dir::FlagRtl, Dir::LayoutRtl] {
        // At 25 % the fill is a quarter of the bar, anchored at the reading start:
        let (outer, fill) = painted_progress(dir, 0.25);
        assert!(
            (fill.width() - 0.25 * outer.width()).abs() < 0.5,
            "{dir:?}: 25 % progress should fill 25 % of the bar (fill {fill:?}, bar {outer:?})"
        );
        // The progress bar's chrome follows the interface flag only, so an explicit
        // right-to-left layout with the flag off paints like left-to-right:
        match dir {
            Dir::Ltr | Dir::LayoutRtl => assert_eq!(
                fill.left(),
                outer.left(),
                "flag off: the fill should hug the bar's left edge (fill {fill:?}, bar {outer:?})"
            ),
            Dir::FlagRtl => assert_eq!(
                fill.right(),
                outer.right(),
                "RTL: the fill should hug the bar's right edge (fill {fill:?}, bar {outer:?})"
            ),
        }

        // At 100 % the fill spans the whole bar in every direction:
        let (outer, fill) = painted_progress(dir, 1.0);
        assert_eq!(
            fill.left(),
            outer.left(),
            "{dir:?}: full progress fills from the start"
        );
        assert_eq!(
            fill.right(),
            outer.right(),
            "{dir:?}: full progress fills to the end"
        );

        // At 0 % only the min-width cap remains, at the reading-start edge:
        let (outer, fill) = painted_progress(dir, 0.0);
        match dir {
            Dir::Ltr | Dir::LayoutRtl => assert_eq!(
                fill.left(),
                outer.left(),
                "flag off: the cap should hug the bar's left edge (fill {fill:?}, bar {outer:?})"
            ),
            Dir::FlagRtl => assert_eq!(
                fill.right(),
                outer.right(),
                "RTL: the cap should hug the bar's right edge (fill {fill:?}, bar {outer:?})"
            ),
        }
        assert!(
            0.0 < fill.width() && fill.width() <= outer.height(),
            "{dir:?}: only the corner-radius cap should remain (fill {fill:?}, bar {outer:?})"
        );
    }
}

/// The range slider's fill spans the two handles, so the shared mirrored geometry
/// carries it: low = 0, high = 25 paints the reading-start quarter in both directions.
#[test]
fn range_slider_fill_mirrors_with_the_shared_geometry() {
    fn painted_range_slider(rtl: bool) -> (Rect, Rect, Rect) {
        let ctx = make_ctx(if rtl { Dir::FlagRtl } else { Dir::Ltr });
        let screen = vec2(400.0, 120.0);
        let mut low = 0.0_f64;
        let mut high = 25.0_f64;
        let mut widget = Rect::NOTHING;
        let mut colors = (Color32::BLACK, Color32::BLACK);
        let output = run(&ctx, input(screen), |ui| {
            colors = (
                ui.visuals().selection.bg_fill,
                ui.visuals().widgets.inactive.bg_fill,
            );
            widget = ui
                .add(RangeSlider::new(&mut low, &mut high, 0.0..=100.0).show_value(false))
                .rect;
        });
        let rail = rail_of(&output, widget, colors.1);
        let fills = rects_in(&output, widget, colors.0);
        assert_eq!(
            fills.len(),
            1,
            "expected exactly one fill (rtl = {rtl}), got {fills:?}"
        );
        (widget, rail, fills[0])
    }

    let (ltr_widget, ltr_rail, ltr_fill) = painted_range_slider(false);
    assert_eq!(
        ltr_fill.left(),
        ltr_rail.left(),
        "LTR: the fill should hug the rail's left edge (fill {ltr_fill:?}, rail {ltr_rail:?})"
    );
    assert!(
        ltr_fill.right() < ltr_rail.center().x,
        "LTR: the low-to-high span should stay on the left half (fill {ltr_fill:?}, rail {ltr_rail:?})"
    );
    let _ = ltr_widget;

    let (rtl_widget, rtl_rail, rtl_fill) = painted_range_slider(true);
    assert_eq!(
        rtl_fill.right(),
        rtl_rail.right(),
        "RTL: the fill should hug the rail's right edge (fill {rtl_fill:?}, rail {rtl_rail:?})"
    );
    assert!(
        rtl_fill.left() > rtl_rail.center().x,
        "RTL: the low-to-high span should sit on the right half (fill {rtl_fill:?}, rail {rtl_rail:?})"
    );
    let _ = rtl_widget;
}

/// The speed a slider hands its `DragValue` is signed: positive left-to-right, negative
/// under either mirroring mechanism, same magnitude. The sign is intentional — it makes
/// dragging and stepping the number beside the rail move the value the rail's way, exactly
/// like an inverted range — so a future "fix" that makes it positive must fail here loudly
/// (see `SliderGeometry::gradient_at`). Observed through the published accesskit
/// `numeric_value_step`, which carries the same signed speed the drag and keyboard paths
/// apply.
#[test]
fn slider_drag_value_speed_stays_signed_under_rtl() {
    fn numeric_value_step(dir: Dir) -> f64 {
        let ctx = make_ctx(dir);
        ctx.enable_accesskit();
        let screen = vec2(400.0, 120.0);
        let mut value = 0.5_f64;
        let output = run(&ctx, input(screen), |ui| {
            rtl_layout(ui, dir, |ui| {
                ui.add(Slider::new(&mut value, 0.0..=1.0));
            });
        });
        let update = output
            .platform_output
            .accesskit_update
            .expect("the slider should publish an accesskit update");
        let steps: Vec<f64> = update
            .nodes
            .iter()
            .filter_map(|(_, node)| node.numeric_value_step())
            .collect();
        assert_eq!(
            steps.len(),
            1,
            "expected exactly one node with a numeric_value_step (the slider's DragValue), \
             got {steps:?}"
        );
        steps[0]
    }

    let ltr = numeric_value_step(Dir::Ltr);
    let flag_rtl = numeric_value_step(Dir::FlagRtl);
    let layout_rtl = numeric_value_step(Dir::LayoutRtl);

    assert!(
        ltr > 0.0,
        "LTR: the DragValue speed must be positive, got {ltr}"
    );
    assert!(
        flag_rtl < 0.0,
        "RTL (flag): the DragValue speed must stay negative, got {flag_rtl}"
    );
    assert!(
        layout_rtl < 0.0,
        "RTL (layout): the DragValue speed must stay negative, got {layout_rtl}"
    );
    assert!(
        (ltr.abs() - flag_rtl.abs()).abs() < 1e-6 && (ltr.abs() - layout_rtl.abs()).abs() < 1e-6,
        "mirroring must flip only the sign, never the magnitude: \
         ltr {ltr}, flag {flag_rtl}, layout {layout_rtl}"
    );
}

/// The center of the only painted text galley of the pass.
///
/// A bare `DragValue` (or a slider with its number and no label) paints exactly one
/// text galley: its value. Used to find where to press for a drag.
fn only_text_center(output: &FullOutput) -> Pos2 {
    fn collect(shape: &Shape, out: &mut Vec<Rect>) {
        match shape {
            Shape::Text(text) => out.push(text.galley.mesh_bounds.translate(text.pos.to_vec2())),
            Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut rects = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut rects);
    }
    assert_eq!(
        rects.len(),
        1,
        "expected exactly one painted text (the value), got {rects:?}"
    );
    rects[0].center()
}

/// Hover `pos`, press, and drag by `delta`, one frame each.
fn drag_by(
    ctx: &Context,
    size: Vec2,
    pos: Pos2,
    delta: Vec2,
    mut show_ui: impl FnMut(&mut Ui),
) -> FullOutput {
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
        input_at(size, 1.2, vec![Event::PointerMoved(pos + delta)]),
        |ui| show_ui(ui),
    )
}

/// A standalone `DragValue` follows the interface reading direction: dragging
/// toward the reading end (physically left under the flag) increases the value, while
/// the same leftward drag decreases it in LTR. Layout-only right-to-left (flag off)
/// keeps the LTR-ward drag, like every other flag-driven widget chrome in this file:
/// only the interface flag mirrors, never the local layout.
#[test]
fn standalone_drag_value_drags_toward_the_reading_direction() {
    fn dragged_value(dir: Dir) -> f64 {
        let ctx = make_ctx(dir);
        let screen = vec2(400.0, 120.0);
        let mut value = 0.5_f64;
        let show = |ui: &mut Ui, value: &mut f64| {
            rtl_layout(ui, dir, |ui| {
                ui.add(DragValue::new(value).range(0.0..=1.0));
            });
        };
        // Frame 1 lays the widget out; its value text is the only painted text:
        let layout = run(&ctx, input(screen), |ui| show(ui, &mut value));
        let pos = only_text_center(&layout);
        drag_by(&ctx, screen, pos, vec2(-10.0, 0.0), |ui| {
            show(ui, &mut value);
        });
        value
    }

    assert_eq!(
        dragged_value(Dir::Ltr),
        0.0,
        "LTR: dragging left should decrease the value toward the range start"
    );
    assert_eq!(
        dragged_value(Dir::FlagRtl),
        1.0,
        "RTL (flag): dragging toward the reading end (left) should increase the value"
    );
    assert_eq!(
        dragged_value(Dir::LayoutRtl),
        0.0,
        "layout-only RTL: the drag keeps its LTR-ward mapping (flag off)"
    );
}

/// The `DragValue` beside a slider rail keeps the rail's signed drag under every
/// direction: its speed is the signed gradient (negative under either mirroring
/// mechanism, pinned by `slider_drag_value_speed_stays_signed_under_rtl`), so dragging
/// the NUMBER toward the reading end (physically left) increases the value without any
/// delta mirroring — mirroring it a second time would double-flip the gesture.
#[test]
fn slider_embedded_drag_value_keeps_the_rail_signed_drag() {
    fn dragged_value(dir: Dir) -> f64 {
        let ctx = make_ctx(dir);
        let screen = vec2(400.0, 120.0);
        let mut value = 0.5_f64;
        let show = |ui: &mut Ui, value: &mut f64| {
            rtl_layout(ui, dir, |ui| {
                ui.add(Slider::new(value, 0.0..=1.0));
            });
        };
        // Frame 1 lays the widget out; its number text is the only painted text:
        let layout = run(&ctx, input(screen), |ui| show(ui, &mut value));
        let pos = only_text_center(&layout);
        drag_by(&ctx, screen, pos, vec2(-10.0, 0.0), |ui| {
            show(ui, &mut value);
        });
        value
    }

    assert!(
        dragged_value(Dir::Ltr) < 0.5,
        "LTR: dragging the number left should decrease the value along the rail"
    );
    assert!(
        dragged_value(Dir::FlagRtl) > 0.5,
        "RTL (flag): dragging the number left should increase the value along the rail \
         (the signed speed and the physical delta cancel)"
    );
    assert!(
        dragged_value(Dir::LayoutRtl) > 0.5,
        "RTL (layout): dragging the number left should increase the value along the rail"
    );
}

/// The end-of-range cursor hints follow the interface reading direction: at the
/// minimum the only productive drag is toward the reading end (physically left under
/// the flag), so the cursor points West there under the flag, and East at the maximum.
/// The physical `Resize*` variants are kept — the widget picks the variant per reading
/// direction. Layout-only right-to-left (flag off) keeps the physical mapping, like
/// every other flag-driven widget chrome in this file.
#[test]
fn drag_value_end_of_range_cursors_follow_the_reading_direction() {
    fn cursor_at(dir: Dir, value: f64) -> CursorIcon {
        let ctx = make_ctx(dir);
        let screen = vec2(400.0, 120.0);
        let mut value = value;
        let show = |ui: &mut Ui, value: &mut f64| {
            rtl_layout(ui, dir, |ui| {
                ui.add(DragValue::new(value).range(0.0..=1.0));
            });
        };
        // Frame 1 lays the widget out; its value text is the only painted text:
        let layout = run(&ctx, input(screen), |ui| show(ui, &mut value));
        let pos = only_text_center(&layout);
        // Frame 2 hovers the widget, so it publishes its hover cursor:
        let output = run(
            &ctx,
            input_at(screen, 1.0, vec![Event::PointerMoved(pos)]),
            |ui| show(ui, &mut value),
        );
        output.platform_output.cursor_icon
    }

    assert_eq!(
        cursor_at(Dir::Ltr, 0.0),
        CursorIcon::ResizeEast,
        "LTR: at the minimum the cursor should invite the rightward drag"
    );
    assert_eq!(
        cursor_at(Dir::Ltr, 1.0),
        CursorIcon::ResizeWest,
        "LTR: at the maximum the cursor should invite the leftward drag"
    );
    assert_eq!(
        cursor_at(Dir::FlagRtl, 0.0),
        CursorIcon::ResizeWest,
        "RTL (flag): at the minimum the only productive drag is leftward, \
         so the cursor should point West"
    );
    assert_eq!(
        cursor_at(Dir::FlagRtl, 0.5),
        CursorIcon::ResizeHorizontal,
        "RTL (flag): mid-range keeps the horizontal resize cursor"
    );
    assert_eq!(
        cursor_at(Dir::FlagRtl, 1.0),
        CursorIcon::ResizeEast,
        "RTL (flag): at the maximum the only productive drag is rightward, \
         so the cursor should point East"
    );
    assert_eq!(
        cursor_at(Dir::LayoutRtl, 0.0),
        CursorIcon::ResizeEast,
        "layout-only RTL: the cursor keeps its physical mapping (flag off)"
    );
}
