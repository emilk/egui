//! Regression tests for `ScrollArea` under the reading direction
//! ([`egui::Style::right_to_left`]): horizontal content anchors at the
//! reading-start (right) edge, pointer drag / wheel / `scroll_with_delta` /
//! `scroll_to_rect` keep their content-movement semantics, and the vertical
//! scroll bar sits on the reading-end (left) edge.
//!
//! Every mirrored case also asserts its LTR counterpart (flag off), so this file
//! doubles as the LTR-unchanged regression. No snapshot assertions; geometry only.

use egui::{
    Align, CentralPanel, Context, Event, FullOutput, Id, Layout, Modifiers, MouseWheelSource,
    MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, ScrollArea, TouchPhase, Ui, Vec2,
    containers::scroll_area::{DragScroll, ScrollBarVisibility, ScrollSource},
    epaint::{Mesh, RectShape, Shape},
    pos2,
    style::ScrollStyle,
    vec2,
};

/// All tests run on this screen.
const SCREEN: Vec2 = vec2(800.0, 600.0);

/// The viewport width of the horizontal test areas.
const VIEWPORT_WIDTH: f32 = 400.0;

/// How many buttons the horizontal row tests lay out.
const N_ROW_BUTTONS: usize = 40;

/// How many buttons the vertical column tests lay out.
const N_COLUMN_BUTTONS: usize = 100;

/// Tolerance for pixel-rounded geometry.
const PIXEL_TOLERANCE: f32 = 1.0;

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

fn button_event(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// A context whose direction flag matches `rtl`, for tests driven purely by the flag.
fn context(rtl: bool) -> Context {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    ctx
}

// ---------------------------------------------------------------------------

/// Geometry captured during one pass over a row of buttons in a
/// `ScrollArea::horizontal` (or a column in a `ScrollArea::vertical`).
#[derive(Clone)]
struct Capture {
    id: Id,
    /// The rect of each button, in reading order (index i = i-th added button).
    buttons: Vec<Rect>,
    /// The content clip rect (`ui.clip_rect()` inside the scroll closure).
    clip: Rect,
    /// The begin-time viewport rect (`ScrollAreaOutput::inner_rect`).
    inner_rect: Rect,
    content_size: Vec2,
    clamped_offset: Vec2,
}

impl Default for Capture {
    fn default() -> Self {
        Self {
            id: Id::NULL,
            buttons: Vec::new(),
            clip: Rect::NOTHING,
            inner_rect: Rect::NOTHING,
            content_size: Vec2::ZERO,
            clamped_offset: Vec2::ZERO,
        }
    }
}

/// One pass over a horizontal scroll area with buttons laid out in reading order.
///
/// `offset` is the builder's `horizontal_scroll_offset` (applied at `begin` of this
/// pass only); `style` replaces `spacing_mut().scroll` on the panel; `tweak`
/// customizes the area; `body` runs inside the content ui after the buttons.
#[expect(clippy::too_many_arguments)]
fn row_pass(
    ctx: &Context,
    rtl: bool,
    time: f64,
    events: Vec<Event>,
    n_buttons: usize,
    offset: Option<f32>,
    salt: &str,
    style: ScrollStyle,
    tweak: &mut dyn FnMut(ScrollArea) -> ScrollArea,
    body: &mut dyn FnMut(&mut Ui, &[Rect]),
    capture: &mut Capture,
) -> FullOutput {
    let layout = if rtl {
        Layout::right_to_left(Align::Min)
    } else {
        Layout::left_to_right(Align::Min)
    };
    let mut buttons = Vec::new();
    let mut id = Id::NULL;
    let mut clip = Rect::NOTHING;
    let mut inner_rect = Rect::NOTHING;
    let mut content_size = Vec2::ZERO;
    let mut clamped_offset = Vec2::ZERO;

    let output = run(ctx, input_at(SCREEN, time, events), |ui| {
        CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                ui.spacing_mut().scroll = style;
                ui.with_layout(layout, |ui| {
                    let mut area = ScrollArea::horizontal().id_salt(salt);
                    if let Some(offset) = offset {
                        area = area.horizontal_scroll_offset(offset);
                    }
                    let out = tweak(area).show(ui, |ui| {
                        for i in 0..n_buttons {
                            buttons.push(ui.button(format!("button {i}")).rect);
                        }
                        clip = ui.clip_rect();
                        body(ui, &buttons);
                    });
                    id = out.id;
                    inner_rect = out.inner_rect;
                    content_size = out.content_size;
                    clamped_offset = out.state.clamped_offset();
                });
            });
    });

    capture.id = id;
    capture.buttons = buttons;
    capture.clip = clip;
    capture.inner_rect = inner_rect;
    capture.content_size = content_size;
    capture.clamped_offset = clamped_offset;
    output
}

/// One pass over a vertical scroll area with a column of buttons.
#[expect(clippy::too_many_arguments)]
fn column_pass(
    ctx: &Context,
    _rtl: bool,
    time: f64,
    events: Vec<Event>,
    n_buttons: usize,
    salt: &str,
    style: ScrollStyle,
    tweak: &mut dyn FnMut(ScrollArea) -> ScrollArea,
    capture: &mut Capture,
) -> FullOutput {
    let mut buttons = Vec::new();
    let mut id = Id::NULL;
    let mut clip = Rect::NOTHING;
    let mut inner_rect = Rect::NOTHING;
    let mut content_size = Vec2::ZERO;
    let mut clamped_offset = Vec2::ZERO;

    let output = run(ctx, input_at(SCREEN, time, events), |ui| {
        CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                ui.spacing_mut().scroll = style;
                let out = tweak(ScrollArea::vertical().id_salt(salt)).show(ui, |ui| {
                    for i in 0..n_buttons {
                        buttons.push(ui.button(format!("button {i}")).rect);
                    }
                    clip = ui.clip_rect();
                });
                id = out.id;
                inner_rect = out.inner_rect;
                content_size = out.content_size;
                clamped_offset = out.state.clamped_offset();
            });
    });

    capture.id = id;
    capture.buttons = buttons;
    capture.clip = clip;
    capture.inner_rect = inner_rect;
    capture.content_size = content_size;
    capture.clamped_offset = clamped_offset;
    output
}

/// The painted handle of a horizontal scroll bar, found among the pass's shapes as
/// the narrowest filled rect inside the bar's strip: the bar background spans the
/// full strip, the handle is the narrower rect in it.
fn horizontal_bar_handle(output: &FullOutput, bar: Rect) -> Option<Rect> {
    let mut handle: Option<Rect> = None;
    for clipped in &output.shapes {
        if let Shape::Rect(RectShape { rect, .. }) = &clipped.shape
            && bar.min.y - PIXEL_TOLERANCE <= rect.min.y
            && rect.max.y <= bar.max.y + PIXEL_TOLERANCE
            && bar.min.x - PIXEL_TOLERANCE <= rect.min.x
            && rect.max.x <= bar.max.x + PIXEL_TOLERANCE
            && rect.width() < bar.width() - PIXEL_TOLERANCE
            && handle.is_none_or(|h| rect.width() < h.width())
        {
            handle = Some(*rect);
        }
    }
    handle
}

/// The horizontal fade gradient rects painted by a scroll area: four-vertex
/// gradient meshes spanning the full height of the (end-time) viewport.
fn horizontal_fades(output: &FullOutput, inner_rect: Rect, viewport_height: f32) -> Vec<Rect> {
    let mut fades = Vec::new();
    for clipped in &output.shapes {
        if let Shape::Mesh(mesh) = &clipped.shape
            && let Mesh { vertices, .. } = &**mesh
            && vertices.len() == 4
        {
            let mut bounds = Rect::from_min_max(vertices[0].pos, vertices[0].pos);
            for vertex in &vertices[1..] {
                bounds.extend_with(vertex.pos);
            }
            if (bounds.height() - viewport_height).abs() <= PIXEL_TOLERANCE
                && (bounds.min.y - inner_rect.min.y).abs() <= 2.0 * PIXEL_TOLERANCE
                && bounds.width() <= 40.0
            {
                fades.push(bounds);
            }
        }
    }
    fades
}

/// `assert_near!` helper: two scalars within a tolerance.
#[track_caller]
fn assert_near(actual: f32, expected: f32, tolerance: f32, message: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{message}: {actual} vs {expected} (tolerance {tolerance})"
    );
}

// ---------------------------------------------------------------------------

/// A1: under RTL a horizontal `ScrollArea` anchors its content at the viewport's
/// right edge at offset 0, and after scrolling to `max_offset` the trailing
/// (leftmost in screen space, last in reading order) button is reachable —
/// unreachable before the fix. LTR counterpart in the same test.
#[test]
fn horizontal_content_reaches_its_trailing_end() {
    fn case(rtl: bool) {
        let ctx = context(rtl);
        let mut capture = Capture::default();

        row_pass(
            &ctx,
            rtl,
            0.0,
            vec![],
            N_ROW_BUTTONS,
            None,
            "trailing_end",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
            &mut |_, _| {},
            &mut capture,
        );
        let max_offset = capture.content_size.x - VIEWPORT_WIDTH;
        assert!(max_offset > 0.0, "the row should overflow the viewport");

        row_pass(
            &ctx,
            rtl,
            0.1,
            vec![],
            N_ROW_BUTTONS,
            Some(max_offset),
            "trailing_end",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
            &mut |_, _| {},
            &mut capture,
        );

        let last = &capture.buttons[N_ROW_BUTTONS - 1];
        assert!(
            last.intersects(capture.clip),
            "the trailing button should intersect the clip rect at max offset \
             (rtl: {rtl}, button {last:?}, clip {:?})",
            capture.clip
        );
        assert!(
            capture.clamped_offset.x >= -PIXEL_TOLERANCE
                && capture.clamped_offset.x <= max_offset + PIXEL_TOLERANCE,
            "the offset must stay within 0..=max_offset (rtl: {rtl})"
        );
    }

    case(true);
    case(false);
}

/// A1: at offset 0 the first button hugs the reading-start edge — the right edge
/// of the viewport under RTL, the left edge under LTR.
#[test]
fn horizontal_starts_at_the_reading_start_edge() {
    fn case(rtl: bool) {
        let ctx = context(rtl);
        let mut capture = Capture::default();

        row_pass(
            &ctx,
            rtl,
            0.0,
            vec![],
            N_ROW_BUTTONS,
            None,
            "reading_start",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
            &mut |_, _| {},
            &mut capture,
        );

        let first = capture.buttons[0];
        if rtl {
            assert_near(
                first.right(),
                capture.inner_rect.right(),
                8.0,
                "the first button's right edge should hug the viewport's right edge under RTL",
            );
        } else {
            assert_near(
                first.left(),
                capture.inner_rect.left(),
                8.0,
                "the first button's left edge should hug the viewport's left edge under LTR",
            );
        }
    }

    case(true);
    case(false);
}

/// A2: `Ui::scroll_to_rect` reveals a target widget under both directions, and
/// `Align::LEFT` aligns with the reading direction: right edges under RTL, left
/// edges under LTR. (The minimal-reveal variant targets the trailing button; the
/// align variant targets a middle button, since aligning the trailing button's
/// reading-start edge would clamp at `max_offset` and mask the align semantics.)
#[test]
fn scroll_to_rect_brings_the_trailing_widget_into_view() {
    fn case(rtl: bool, align: Option<Align>) {
        let ctx = context(rtl);
        let mut capture = Capture::default();
        let target_index = if align.is_some() {
            N_ROW_BUTTONS / 2
        } else {
            N_ROW_BUTTONS - 1
        };

        row_pass(
            &ctx,
            rtl,
            0.0,
            vec![],
            N_ROW_BUTTONS,
            None,
            "scroll_to_rect",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
            &mut |ui, buttons| {
                ui.scroll_to_rect(buttons[target_index], align);
            },
            &mut capture,
        );
        row_pass(
            &ctx,
            rtl,
            0.1,
            vec![],
            N_ROW_BUTTONS,
            None,
            "scroll_to_rect",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
            &mut |_, _| {},
            &mut capture,
        );

        let target = capture.buttons[target_index];
        assert!(
            target.intersects(capture.clip),
            "the target button should be in view after scroll_to_rect \
             (rtl: {rtl}, align: {align:?}, button {target:?}, clip {:?})",
            capture.clip
        );
        if align == Some(Align::LEFT) {
            let item_spacing = ctx.global_style().spacing.item_spacing.x;
            if rtl {
                assert_near(
                    target.right(),
                    capture.clip.right(),
                    item_spacing + 2.0,
                    "Align::LEFT should align the target's reading-start (right) edge with the clip's right edge under RTL",
                );
            } else {
                assert_near(
                    target.left(),
                    capture.clip.left(),
                    item_spacing + 2.0,
                    "Align::LEFT should align the target's left edge with the clip's left edge under LTR",
                );
            }
        }
    }

    for align in [None, Some(Align::LEFT)] {
        case(true, align);
        case(false, align);
    }
}

/// A2: a pointer drag moves the content with the finger in both directions:
/// a rightward gesture moves the first button right by the drag amount.
#[test]
fn drag_follows_the_finger() {
    fn case(rtl: bool) {
        const DRAG: f32 = 40.0;
        let ctx = context(rtl);
        let mut capture = Capture::default();
        let scroll_source = ScrollSource {
            drag: DragScroll::Always,
            ..Default::default()
        };

        // Pass 0: warm-up so `state.interact_rect` is sensed and we learn the geometry.
        row_pass(
            &ctx,
            rtl,
            0.0,
            vec![],
            N_ROW_BUTTONS,
            None,
            "drag",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).scroll_source(scroll_source),
            &mut |_, _| {},
            &mut capture,
        );
        let mid = (capture.content_size.x - VIEWPORT_WIDTH) / 2.0;
        // The press must land on the area's interact rect (the row itself), so pick a
        // visible button: pressing a click-sensed button still starts the drag on the
        // scroll area's drag-sensed background. Press near the button's top edge, since
        // the floating scroll bar overlaps the bottom of the (short) row.
        let press = capture
            .buttons
            .iter()
            .find(|rect| rect.intersects(capture.clip))
            .map(|rect| pos2(rect.center().x, rect.min.y + 2.0))
            .unwrap_or_else(|| panic!("a button should be visible at the pre-scroll offset"));

        // Pass 1: pre-scroll to an in-bounds offset and move the pointer onto the area.
        row_pass(
            &ctx,
            rtl,
            0.1,
            vec![Event::PointerMoved(press)],
            N_ROW_BUTTONS,
            Some(mid),
            "drag",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).scroll_source(scroll_source),
            &mut |_, _| {},
            &mut capture,
        );
        let before = capture.buttons[0];

        // Pass 2: press (zero pointer delta — the pointer is already there).
        row_pass(
            &ctx,
            rtl,
            0.2,
            vec![button_event(press, PointerButton::Primary, true)],
            N_ROW_BUTTONS,
            None,
            "drag",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).scroll_source(scroll_source),
            &mut |_, _| {},
            &mut capture,
        );

        // Pass 3: drag right by `DRAG`.
        row_pass(
            &ctx,
            rtl,
            0.3,
            vec![Event::PointerMoved(press + vec2(DRAG, 0.0))],
            N_ROW_BUTTONS,
            None,
            "drag",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).scroll_source(scroll_source),
            &mut |_, _| {},
            &mut capture,
        );

        // Pass 4: idle — the drag is applied in `begin` after the content was laid
        // out, so the moved content is only visible on the next pass.
        row_pass(
            &ctx,
            rtl,
            0.4,
            vec![],
            N_ROW_BUTTONS,
            None,
            "drag",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).scroll_source(scroll_source),
            &mut |_, _| {},
            &mut capture,
        );
        let after = capture.buttons[0];

        // Pass 5: release (not asserted; release starts kinetic scrolling).
        row_pass(
            &ctx,
            rtl,
            0.5,
            vec![button_event(
                press + vec2(DRAG, 0.0),
                PointerButton::Primary,
                false,
            )],
            N_ROW_BUTTONS,
            None,
            "drag",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).scroll_source(scroll_source),
            &mut |_, _| {},
            &mut capture,
        );

        assert_near(
            after.center().x - before.center().x,
            DRAG,
            2.0,
            "a rightward drag should move the content right by the drag amount in both directions",
        );
    }

    case(true);
    case(false);
}

/// A2: `MouseWheel` and `Ui::scroll_with_delta` follow the content-movement
/// convention in both directions: a content-right delta of 30 moves the first
/// button right by 30.
#[test]
fn wheel_and_scroll_with_delta_follow_the_content_movement_convention() {
    fn case(rtl: bool, variant: Variant) {
        const DELTA: f32 = 30.0;
        let ctx = context(rtl);
        let mut capture = Capture::default();

        // Pass 0: warm-up so we learn the geometry.
        row_pass(
            &ctx,
            rtl,
            0.0,
            vec![],
            N_ROW_BUTTONS,
            None,
            "wheel",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
            &mut |_, _| {},
            &mut capture,
        );
        let mid = (capture.content_size.x - VIEWPORT_WIDTH) / 2.0;
        // The pointer must sit inside the area's outer rect for the wheel block to run,
        // so put it on a visible button.
        let pointer = capture
            .buttons
            .iter()
            .find(|rect| rect.intersects(capture.clip))
            .map(|rect| rect.center())
            .unwrap_or_else(|| panic!("a button should be visible at offset 0"));

        // Pass 1: pre-scroll to an in-bounds offset and move the pointer over the area
        // (the wheel block is gated on the pointer being inside the outer rect).
        row_pass(
            &ctx,
            rtl,
            0.1,
            vec![Event::PointerMoved(pointer)],
            N_ROW_BUTTONS,
            Some(mid),
            "wheel",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
            &mut |_, _| {},
            &mut capture,
        );
        let before = capture.buttons[0];

        // Pass 2: the scroll gesture itself.
        match variant {
            Variant::Wheel => {
                row_pass(
                    &ctx,
                    rtl,
                    0.2,
                    vec![Event::MouseWheel {
                        unit: MouseWheelUnit::Point,
                        delta: vec2(DELTA, 0.0),
                        phase: TouchPhase::Move,
                        source: MouseWheelSource::Wheel,
                        modifiers: Modifiers::NONE,
                    }],
                    N_ROW_BUTTONS,
                    None,
                    "wheel",
                    ScrollStyle::default(),
                    &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
                    &mut |_, _| {},
                    &mut capture,
                );
            }
            Variant::ScrollWithDelta => {
                row_pass(
                    &ctx,
                    rtl,
                    0.2,
                    vec![],
                    N_ROW_BUTTONS,
                    None,
                    "wheel",
                    ScrollStyle::default(),
                    &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
                    &mut |ui, _| {
                        ui.scroll_with_delta(vec2(DELTA, 0.0));
                    },
                    &mut capture,
                );
            }
        }

        // Pass 3: the delta is consumed in `end`, so read the geometry on the next pass.
        row_pass(
            &ctx,
            rtl,
            0.3,
            vec![],
            N_ROW_BUTTONS,
            None,
            "wheel",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
            &mut |_, _| {},
            &mut capture,
        );
        let after = capture.buttons[0];

        // The wheel variant goes through the input pipeline's smooth-wheel integration,
        // which scales the injected delta slightly; `scroll_with_delta` is exact.
        let tolerance = match variant {
            Variant::Wheel => 6.0,
            Variant::ScrollWithDelta => 2.0,
        };
        let message = format!(
            "a content-right delta should move the content right in both directions \
             (rtl: {rtl}, variant: {variant:?})"
        );
        assert_near(
            after.center().x - before.center().x,
            DELTA,
            tolerance,
            &message,
        );
    }

    for variant in [Variant::Wheel, Variant::ScrollWithDelta] {
        case(true, variant);
        case(false, variant);
    }
}

#[derive(Clone, Copy, Debug)]
enum Variant {
    Wheel,
    ScrollWithDelta,
}

/// A3: with a space-reserving (solid) style the vertical scroll bar and its
/// reserved space sit on the reading-end edge — left under RTL, right under
/// LTR — and the content clip rect matches the viewport on x in both
/// directions (the bar strip is cropped from the correct side).
#[test]
fn vertical_scroll_bar_is_on_the_reading_end_edge() {
    fn case(rtl: bool, floating: bool) {
        let ctx = context(rtl);
        let mut capture = Capture::default();
        let mut style = ScrollStyle::solid();
        if floating {
            style.floating = true;
        }

        // Run until the bars' show factor settles at 1.0.
        for pass in 0..12 {
            column_pass(
                &ctx,
                rtl,
                pass as f64 * 0.1,
                vec![],
                N_COLUMN_BUTTONS,
                "bar_side",
                style,
                &mut |area| {
                    area.auto_shrink([false, true])
                        .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                },
                &mut capture,
            );
        }

        let bar = ctx.read_response(capture.id.with(1)).unwrap_or_else(|| {
            panic!("the vertical scroll bar should be interactable (rtl: {rtl})")
        });

        if floating {
            // The bar overlaps the content edge instead of reserving space; assert the side only.
            if rtl {
                assert!(
                    bar.rect.center().x < capture.inner_rect.center().x,
                    "the floating bar should hug the reading-end (left) edge under RTL \
                     (bar {:?}, inner {:?})",
                    bar.rect,
                    capture.inner_rect
                );
            } else {
                assert!(
                    bar.rect.center().x > capture.inner_rect.center().x,
                    "the floating bar should hug the reading-end (right) edge under LTR \
                     (bar {:?}, inner {:?})",
                    bar.rect,
                    capture.inner_rect
                );
            }
        } else {
            if rtl {
                assert!(
                    bar.rect.max.x <= capture.inner_rect.min.x + PIXEL_TOLERANCE,
                    "the bar should sit on the reading-end (left) edge under RTL \
                     (bar {:?}, inner {:?})",
                    bar.rect,
                    capture.inner_rect
                );
                assert!(
                    capture.inner_rect.min.x > 0.5,
                    "the bar's space should be reserved on the left under RTL \
                     (inner {:?})",
                    capture.inner_rect
                );
            } else {
                assert!(
                    bar.rect.min.x >= capture.inner_rect.max.x - PIXEL_TOLERANCE,
                    "the bar should sit on the reading-end (right) edge under LTR \
                     (bar {:?}, inner {:?})",
                    bar.rect,
                    capture.inner_rect
                );
            }
            // The clip fix: the bar strip is cropped from the reading-end side, so the
            // clip rect's x-range matches the viewport's x-range in both directions.
            assert_near(
                capture.clip.min.x,
                capture.inner_rect.min.x,
                PIXEL_TOLERANCE,
                "the clip's left edge should match the viewport's left edge",
            );
            assert_near(
                capture.clip.max.x,
                capture.inner_rect.max.x,
                PIXEL_TOLERANCE,
                "the clip's right edge should match the viewport's right edge",
            );
        }
    }

    case(true, false);
    case(false, false);
    case(true, true);
    case(false, true);
}

/// A3: `ScrollArea::scroll_bar_rect` overrides stay absolute along the scrolling
/// axis under RTL, and a horizontal bar's handle travel stays within the override.
#[test]
fn scroll_bar_rect_override_still_wins() {
    let custom = Rect::from_min_max(pos2(100.0, 50.0), pos2(700.0, 550.0));

    // Vertical bar: the override wins along the scrolling (y) axis under RTL.
    {
        let ctx = context(true);
        let mut capture = Capture::default();
        for pass in 0..12 {
            column_pass(
                &ctx,
                true,
                pass as f64 * 0.1,
                vec![],
                N_COLUMN_BUTTONS,
                "bar_override",
                ScrollStyle::solid(),
                &mut |area| {
                    area.scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                        .scroll_bar_rect(custom)
                },
                &mut capture,
            );
        }
        let bar = ctx
            .read_response(capture.id.with(1))
            .unwrap_or_else(|| panic!("the vertical scroll bar should be interactable"));
        assert_near(
            bar.rect.top(),
            custom.top(),
            PIXEL_TOLERANCE,
            "the vertical bar's override rect should win along y under RTL",
        );
        assert_near(
            bar.rect.bottom(),
            custom.bottom(),
            PIXEL_TOLERANCE,
            "the vertical bar's override rect should win along y under RTL",
        );
    }

    // Horizontal bar: the handle stays within the override's x-range under RTL.
    {
        let ctx = context(true);
        let mut capture = Capture::default();
        let mut output = None;
        for pass in 0..12 {
            output = Some(row_pass(
                &ctx,
                true,
                pass as f64 * 0.1,
                vec![],
                N_ROW_BUTTONS,
                None,
                "bar_override_h",
                ScrollStyle::solid(),
                &mut |area| {
                    area.max_width(VIEWPORT_WIDTH)
                        .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                        .scroll_bar_rect(custom)
                },
                &mut |_, _| {},
                &mut capture,
            ));
        }
        let bar = ctx
            .read_response(capture.id.with(0))
            .unwrap_or_else(|| panic!("the horizontal scroll bar should be interactable"));
        let handle = horizontal_bar_handle(&output.unwrap(), bar.rect)
            .unwrap_or_else(|| panic!("the horizontal handle should be painted"));
        assert!(
            custom.x_range().contains(handle.min.x) && custom.x_range().contains(handle.max.x),
            "the handle travel should stay within the override's x-range (handle {handle:?})"
        );
        assert_near(
            handle.right(),
            custom.right(),
            PIXEL_TOLERANCE,
            "at offset 0 the handle should sit at the reading-start (right) end of the override",
        );
    }
}

/// A5: under RTL the horizontal handle at offset 0 sits at the bar's right end,
/// and dragging it toward the reading end (left) increases the offset; mirrored
/// under LTR.
#[test]
fn handle_drag_and_position_mirror() {
    fn case(rtl: bool) {
        let ctx = context(rtl);
        let mut capture = Capture::default();
        let mut output = None;

        // Settle the bars (solid style, always visible).
        for pass in 0..12 {
            output = Some(row_pass(
                &ctx,
                rtl,
                pass as f64 * 0.1,
                vec![],
                N_ROW_BUTTONS,
                None,
                "handle",
                ScrollStyle::solid(),
                &mut |area| {
                    area.max_width(VIEWPORT_WIDTH)
                        .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                },
                &mut |_, _| {},
                &mut capture,
            ));
        }
        let bar = ctx
            .read_response(capture.id.with(0))
            .unwrap_or_else(|| panic!("the horizontal scroll bar should be interactable"));
        let handle = horizontal_bar_handle(&output.unwrap(), bar.rect)
            .unwrap_or_else(|| panic!("the horizontal handle should be painted"));

        if rtl {
            assert_near(
                handle.right(),
                bar.rect.right(),
                PIXEL_TOLERANCE,
                "at offset 0 the handle should sit at the bar's right end under RTL",
            );
        } else {
            assert_near(
                handle.left(),
                bar.rect.left(),
                PIXEL_TOLERANCE,
                "at offset 0 the handle should sit at the bar's left end under LTR",
            );
        }

        let before = capture.buttons[0];
        let before_offset = capture.clamped_offset.x;
        let drag = if rtl { -50.0 } else { 50.0 };
        let press = handle.center();
        let moved = press + vec2(drag, 0.0);

        row_pass(
            &ctx,
            rtl,
            1.3,
            vec![button_event(press, PointerButton::Primary, true)],
            N_ROW_BUTTONS,
            None,
            "handle",
            ScrollStyle::solid(),
            &mut |area| {
                area.max_width(VIEWPORT_WIDTH)
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            },
            &mut |_, _| {},
            &mut capture,
        );
        row_pass(
            &ctx,
            rtl,
            1.4,
            vec![Event::PointerMoved(moved)],
            N_ROW_BUTTONS,
            None,
            "handle",
            ScrollStyle::solid(),
            &mut |area| {
                area.max_width(VIEWPORT_WIDTH)
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            },
            &mut |_, _| {},
            &mut capture,
        );
        row_pass(
            &ctx,
            rtl,
            1.5,
            vec![button_event(moved, PointerButton::Primary, false)],
            N_ROW_BUTTONS,
            None,
            "handle",
            ScrollStyle::solid(),
            &mut |area| {
                area.max_width(VIEWPORT_WIDTH)
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            },
            &mut |_, _| {},
            &mut capture,
        );

        assert!(
            capture.clamped_offset.x > before_offset,
            "dragging the handle toward the reading end should increase the offset \
             (rtl: {rtl}, {} -> {})",
            before_offset,
            capture.clamped_offset.x
        );
        let content_move = capture.buttons[0].center().x - before.center().x;
        if rtl {
            assert!(
                content_move > 0.0,
                "dragging the handle left under RTL should move the content right (moved {content_move})"
            );
        } else {
            assert!(
                content_move < 0.0,
                "dragging the handle right under LTR should move the content left (moved {content_move})"
            );
        }
    }

    case(true);
    case(false);
}

/// A4: flipping the direction flag between frames preserves the reading position:
/// the same builder offset reveals the same middle button under both directions,
/// and the offset stays within `0..=max_offset`.
#[test]
fn direction_flip_preserves_the_reading_position() {
    let ctx = Context::default();
    let mut capture = Capture::default();

    row_pass(
        &ctx,
        false,
        0.0,
        vec![],
        N_ROW_BUTTONS,
        None,
        "flip",
        ScrollStyle::default(),
        &mut |area| area.max_width(VIEWPORT_WIDTH),
        &mut |_, _| {},
        &mut capture,
    );
    let mid = (capture.content_size.x - VIEWPORT_WIDTH) / 2.0;

    row_pass(
        &ctx,
        false,
        0.1,
        vec![],
        N_ROW_BUTTONS,
        Some(mid),
        "flip",
        ScrollStyle::default(),
        &mut |area| area.max_width(VIEWPORT_WIDTH),
        &mut |_, _| {},
        &mut capture,
    );
    let ltr = capture.clone();
    assert!(
        ltr.clamped_offset.x >= -PIXEL_TOLERANCE && ltr.clamped_offset.x <= mid + PIXEL_TOLERANCE,
        "the LTR offset must stay within 0..=max_offset"
    );

    ctx.set_right_to_left(true);
    row_pass(
        &ctx,
        true,
        0.2,
        vec![],
        N_ROW_BUTTONS,
        Some(mid),
        "flip",
        ScrollStyle::default(),
        &mut |area| area.max_width(VIEWPORT_WIDTH),
        &mut |_, _| {},
        &mut capture,
    );
    assert!(
        capture.clamped_offset.x >= -PIXEL_TOLERANCE
            && capture.clamped_offset.x <= mid + PIXEL_TOLERANCE,
        "the RTL offset must stay within 0..=max_offset"
    );

    // The button nearest the LTR viewport center must still be visible under RTL.
    let ltr_center = ltr.clip.center().x;
    let (index, _) = ltr
        .buttons
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (a.center().x - ltr_center)
                .abs()
                .total_cmp(&(b.center().x - ltr_center).abs())
        })
        .unwrap_or_else(|| panic!("the LTR pass should have captured buttons"));
    assert!(
        capture.buttons[index].intersects(capture.clip),
        "the same middle button should be visible after the direction flip \
         (button {:?}, clip {:?})",
        capture.buttons[index],
        capture.clip
    );
}

/// A6: the horizontal fades appear on the side that actually hides content:
/// under RTL the left fade shows from offset 0 and the right fade once the
/// reading start has been scrolled past; mirrored under LTR.
#[test]
fn horizontal_fades_mirror() {
    fn case(rtl: bool) {
        let ctx = context(rtl);
        let mut capture = Capture::default();

        // Pass 1: offset 0 — only the far-end fade shows.
        let output = row_pass(
            &ctx,
            rtl,
            0.0,
            vec![],
            N_ROW_BUTTONS,
            None,
            "fades",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
            &mut |_, _| {},
            &mut capture,
        );
        let inner = capture.inner_rect;
        let fades = horizontal_fades(&output, inner, capture.content_size.y);
        assert_eq!(
            fades.len(),
            1,
            "exactly one horizontal fade should show at offset 0 (rtl: {rtl}): {fades:?}"
        );
        if rtl {
            assert_near(
                fades[0].min.x,
                inner.left(),
                2.0,
                "under RTL the left fade should show at offset 0",
            );
        } else {
            assert_near(
                fades[0].max.x,
                inner.right(),
                2.0,
                "under LTR the right fade should show at offset 0",
            );
        }

        // Pass 2: offset max — only the reading-start fade shows.
        let max_offset = capture.content_size.x - VIEWPORT_WIDTH;
        let output = row_pass(
            &ctx,
            rtl,
            0.1,
            vec![],
            N_ROW_BUTTONS,
            Some(max_offset),
            "fades",
            ScrollStyle::default(),
            &mut |area| area.max_width(VIEWPORT_WIDTH).animated(false),
            &mut |_, _| {},
            &mut capture,
        );
        let inner = capture.inner_rect;
        let fades = horizontal_fades(&output, inner, capture.content_size.y);
        assert_eq!(
            fades.len(),
            1,
            "exactly one horizontal fade should show at max offset (rtl: {rtl}): {fades:?}"
        );
        if rtl {
            assert_near(
                fades[0].max.x,
                inner.right(),
                2.0,
                "under RTL the right fade should show at max offset",
            );
        } else {
            assert_near(
                fades[0].min.x,
                inner.left(),
                2.0,
                "under LTR the left fade should show at max offset",
            );
        }
    }

    case(true);
    case(false);
}

/// A7: flipping the direction flag between frames keeps the persisted offset
/// reading-relative: a NON-center offset stored under one direction reads back
/// unchanged after a flip with no builder offset (so `State::load` serves the
/// persisted offset instead of it being clobbered), and the same reading-relative
/// span of buttons stays visible. A load-time remap (`x -> max_offset.x - x`)
/// would teleport the viewport to the opposite end; this pins that none exists.
///
/// The assertions describe today's behavior BY DESIGN: there is no fail-before,
/// this pin guards the refuted deep finding that claimed the persisted offset
/// flips meaning at a runtime direction toggle. The ltr twin is the flip back
/// to ltr, asserted in the same test.
#[test]
fn direction_flip_keeps_a_non_center_offset_reading_relative() {
    fn nearest_button_index(buttons: &[Rect], x: f32) -> usize {
        buttons
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (a.center().x - x)
                    .abs()
                    .total_cmp(&(b.center().x - x).abs())
            })
            .map(|(index, _)| index)
            .expect("the pass should have captured buttons")
    }

    let ctx = Context::default();
    let mut capture = Capture::default();

    // Seed the persisted state (offset 0).
    row_pass(
        &ctx,
        false,
        0.0,
        vec![],
        N_ROW_BUTTONS,
        None,
        "flip-offset",
        ScrollStyle::default(),
        &mut |area| area.max_width(VIEWPORT_WIDTH),
        &mut |_, _| {},
        &mut capture,
    );
    let x = (capture.content_size.x - VIEWPORT_WIDTH) / 4.0;
    assert!(
        x > PIXEL_TOLERANCE
            && ((capture.content_size.x - VIEWPORT_WIDTH) / 2.0 - x).abs() > PIXEL_TOLERANCE,
        "the pin offset must be non-center for this pin to mean anything (x: {x})"
    );

    // LTR with a builder offset x: the window shows the reading-relative span
    // `x..x+viewport_width`.
    row_pass(
        &ctx,
        false,
        0.1,
        vec![],
        N_ROW_BUTTONS,
        Some(x),
        "flip-offset",
        ScrollStyle::default(),
        &mut |area| area.max_width(VIEWPORT_WIDTH),
        &mut |_, _| {},
        &mut capture,
    );
    let ltr = capture.clone();

    // Flip to RTL WITHOUT a builder offset, so the persisted offset is loaded.
    ctx.set_right_to_left(true);
    row_pass(
        &ctx,
        true,
        0.2,
        vec![],
        N_ROW_BUTTONS,
        None,
        "flip-offset",
        ScrollStyle::default(),
        &mut |area| area.max_width(VIEWPORT_WIDTH),
        &mut |_, _| {},
        &mut capture,
    );
    assert_near(
        capture.clamped_offset.x,
        x,
        PIXEL_TOLERANCE,
        "RTL: the persisted non-center offset must read back unchanged after the flip \
         (a remap would read max_offset - x)",
    );
    // The button at the reading-start edge of the ltr window is still visible.
    let index = nearest_button_index(&ltr.buttons, ltr.clip.min.x);
    assert!(
        capture.buttons[index].intersects(capture.clip),
        "RTL: the button at the ltr window's reading-start edge should still be visible \
         (button {:?}, clip {:?})",
        capture.buttons[index],
        capture.clip
    );

    // Flip back to LTR: store x under rtl, then load it without a builder offset.
    row_pass(
        &ctx,
        true,
        0.3,
        vec![],
        N_ROW_BUTTONS,
        Some(x),
        "flip-offset",
        ScrollStyle::default(),
        &mut |area| area.max_width(VIEWPORT_WIDTH),
        &mut |_, _| {},
        &mut capture,
    );
    let rtl = capture.clone();
    ctx.set_right_to_left(false);
    row_pass(
        &ctx,
        false,
        0.4,
        vec![],
        N_ROW_BUTTONS,
        None,
        "flip-offset",
        ScrollStyle::default(),
        &mut |area| area.max_width(VIEWPORT_WIDTH),
        &mut |_, _| {},
        &mut capture,
    );
    assert_near(
        capture.clamped_offset.x,
        x,
        PIXEL_TOLERANCE,
        "LTR: the persisted non-center offset must read back unchanged after the flip back \
         (a remap would read max_offset - x)",
    );
    let index = nearest_button_index(&rtl.buttons, rtl.clip.max.x);
    assert!(
        capture.buttons[index].intersects(capture.clip),
        "LTR: the button at the rtl window's reading-start edge should still be visible \
         (button {:?}, clip {:?})",
        capture.buttons[index],
        capture.clip
    );
}

/// One pass over a vertical-only scroll area with a narrow column of buttons,
/// capturing the area's geometry and the measured width of the parent `Ui`
/// after the area (the captures the other helpers don't provide). The area is
/// `auto_shrink([true, true])` (the default, spelled out), so the x axis
/// follows the content.
fn narrow_column_pass(
    ctx: &Context,
    time: f64,
    salt: &str,
    bar_visibility: ScrollBarVisibility,
    capture: &mut Capture,
    parent_rect: &mut Rect,
) -> FullOutput {
    let mut buttons = Vec::new();
    let mut clip = Rect::NOTHING;
    let mut inner_rect = Rect::NOTHING;
    let mut content_size = Vec2::ZERO;

    let output = run(ctx, input_at(SCREEN, time, vec![]), |ui| {
        CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                // A measuring parent: a nested vertical `Ui` whose `min_rect` hugs
                // what the scroll area reserves (a panel always fills its offered
                // space, so it cannot answer this).
                ui.vertical(|ui| {
                    let out = ScrollArea::vertical()
                        .id_salt(salt)
                        // Explicit rather than the (identical) default, so a future
                        // default change cannot silently un-hug these pins:
                        .auto_shrink([true, true])
                        .scroll_bar_visibility(bar_visibility)
                        .show(ui, |ui| {
                            for i in 0..N_COLUMN_BUTTONS {
                                buttons.push(ui.button(format!("button {i}")).rect);
                            }
                            clip = ui.clip_rect();
                        });
                    inner_rect = out.inner_rect;
                    content_size = out.content_size;
                    *parent_rect = ui.min_rect();
                });
            });
    });

    capture.buttons = buttons;
    capture.clip = clip;
    capture.inner_rect = inner_rect;
    capture.content_size = content_size;
    output
}

/// A8: the residuals the NOTE next to `rtl_x` used to document as unresolved,
/// settled by measurement (they are healed): with `auto_shrink` enabled, a
/// vertical-only area reserves a content-hugging span in a measuring parent,
/// anchored at the reading start under rtl exactly like under ltr, with and
/// without the vertical scroll bar (floating by default, so the bar variant
/// reserves the same strip-less span). The begin-time viewport stays full-width
/// by design (the anti-ballooning anchor in `begin`); the reserved span is what
/// the parent measures.
#[test]
fn vertical_auto_shrink_hugs_the_content_width_in_both_directions() {
    fn case(rtl: bool) {
        let ctx = context(rtl);
        let mut capture = Capture::default();
        let mut parent_rect = Rect::NOTHING;

        // Without a bar strip:
        narrow_column_pass(
            &ctx,
            0.0,
            "shrink-nobar",
            ScrollBarVisibility::VisibleWhenNeeded,
            &mut capture,
            &mut parent_rect,
        );
        assert_near(
            parent_rect.width(),
            capture.content_size.x,
            PIXEL_TOLERANCE,
            &format!("the reserved span should hug the content width (rtl: {rtl})"),
        );
        assert_near(
            capture.inner_rect.width(),
            SCREEN.x,
            PIXEL_TOLERANCE,
            &format!("the begin-time viewport should stay full-width by design (rtl: {rtl})"),
        );
        if rtl {
            assert_near(
                parent_rect.right(),
                SCREEN.x,
                PIXEL_TOLERANCE,
                "rtl: the reserved span should anchor at the reading-start (right) edge",
            );
        } else {
            assert_near(
                parent_rect.left(),
                0.0,
                PIXEL_TOLERANCE,
                "ltr: the reserved span should anchor at the reading-start (left) edge",
            );
        }

        // With the vertical scroll bar (AlwaysVisible; it floats over the content
        // by default, so no strip is reserved):
        narrow_column_pass(
            &ctx,
            0.1,
            "shrink-bar",
            ScrollBarVisibility::AlwaysVisible,
            &mut capture,
            &mut parent_rect,
        );
        assert_near(
            parent_rect.width(),
            capture.content_size.x,
            PIXEL_TOLERANCE,
            &format!("the reserved span should hug the content width with a bar too (rtl: {rtl})"),
        );
        if rtl {
            assert_near(
                parent_rect.right(),
                SCREEN.x,
                PIXEL_TOLERANCE,
                "rtl with a bar: the reserved span should anchor at the reading-start \
                 (right) edge",
            );
        } else {
            assert_near(
                parent_rect.left(),
                0.0,
                PIXEL_TOLERANCE,
                "ltr with a bar: the reserved span should anchor at the reading-start \
                 (left) edge",
            );
        }
    }

    case(true);
    case(false);
}

/// A9: no direction asymmetry in the auto-shrunk vertical window: the reserved
/// span's width is the same under rtl and ltr, with and without the bar.
#[test]
fn vertical_auto_shrink_window_width_is_direction_symmetric() {
    fn measure(rtl: bool, salt: &str, bar_visibility: ScrollBarVisibility) -> (f32, f32) {
        let ctx = context(rtl);
        let mut capture = Capture::default();
        let mut parent_rect = Rect::NOTHING;
        narrow_column_pass(
            &ctx,
            0.0,
            salt,
            bar_visibility,
            &mut capture,
            &mut parent_rect,
        );
        (parent_rect.width(), capture.content_size.x)
    }

    for (salt, bar_visibility) in [
        ("symmetric-nobar", ScrollBarVisibility::VisibleWhenNeeded),
        ("symmetric-bar", ScrollBarVisibility::AlwaysVisible),
    ] {
        let (rtl_width, rtl_content) = measure(true, salt, bar_visibility);
        let (ltr_width, ltr_content) = measure(false, salt, bar_visibility);
        assert_near(
            rtl_content,
            ltr_content,
            PIXEL_TOLERANCE,
            "the content should measure the same in both directions",
        );
        assert_near(
            rtl_width,
            ltr_width,
            PIXEL_TOLERANCE,
            &format!(
                "the reserved span's width should be direction-symmetric \
             (rtl {rtl_width}, ltr {ltr_width})"
            ),
        );
    }
}
