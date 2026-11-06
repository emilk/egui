//! Regression tests for [`egui::Sides`] under the whole-UI direction flag
//! ([`egui::Style::right_to_left`]): the two slots are reading-order roles, so
//! under RTL the reading-start side (`add_left`) hugs the RIGHT edge of the
//! container and the reading-end side (`add_right`) hugs the LEFT edge — the
//! point reflection of the LTR layout — the same convention [`egui::Grid`]
//! uses for its cells. Under LTR nothing changes.
//!
//! Every mirrored case also asserts its LTR counterpart, so this file doubles
//! as the LTR-unchanged regression. The content is direction-independent
//! (fixed-size widgets, identical in both runs), so the mirrored layouts are
//! exact point reflections of each other. No snapshot assertions; geometry
//! only. An explicit `left_to_right` layout still wins over the global flag,
//! consistent with [`egui::Grid`] and the `horizontal` helpers.

use egui::{
    Align, CentralPanel, Context, Layout, Pos2, RawInput, Rect, Sense, Sides, Ui, Vec2, vec2,
};

const SCREEN: Vec2 = vec2(600.0, 400.0);
const TOL: f32 = 0.5;

/// The content of the reading-start (`add_left`) side: fixed-size so the
/// layout is identical under both directions.
const LEFT_SIZE: Vec2 = vec2(120.0, 10.0);

/// The content of the reading-end (`add_right`) side.
const RIGHT_SIZE: Vec2 = vec2(150.0, 10.0);

/// Oversized reading-start content for the shrink-priority test: together
/// with [`RIGHT_SIZE`] and the item spacing it cannot fit the screen, so the
/// shrinking side's max rect must be capped to the remainder.
const OVERSIZED_LEFT_SIZE: Vec2 = vec2(450.0, 10.0);

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
}

/// The geometry of one `Sides` slot: the child's min rect (what it painted)
/// and its max rect as handed to the child (the constraint `Sides` chose —
/// recorded before any content, since an overflowing widget expands it).
type SideRects = (Rect, Rect);

/// The item spacing a fresh context uses (what [`Sides`] defaults to).
fn default_item_spacing_x() -> f32 {
    let ctx = Context::default();
    ctx.style_of(ctx.theme()).spacing.item_spacing.x
}

/// One frame: a central panel whose content is one `Sides` call with one
/// fixed-size widget per side. Returns the container rect the `Sides` read
/// (its available rect before wrap) and the two sides' geometry.
fn sides_layout(
    rtl: bool,
    sides: Sides,
    left_size: Vec2,
    right_size: Vec2,
) -> (Rect, SideRects, SideRects) {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    let mut container = Rect::NOTHING;
    let mut left_rects: SideRects = (Rect::NOTHING, Rect::NOTHING);
    let mut right_rects: SideRects = (Rect::NOTHING, Rect::NOTHING);
    run(&ctx, input(SCREEN), |ui| {
        CentralPanel::default().show(ui, |ui| {
            container = ui.available_rect_before_wrap();
            let (left, right) = sides.show(
                ui,
                |ui| {
                    let max_rect = ui.max_rect();
                    ui.allocate_exact_size(left_size, Sense::hover());
                    (ui.min_rect(), max_rect)
                },
                |ui| {
                    let max_rect = ui.max_rect();
                    ui.allocate_exact_size(right_size, Sense::hover());
                    (ui.min_rect(), max_rect)
                },
            );
            left_rects = left;
            right_rects = right;
        });
    });
    (container, left_rects, right_rects)
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOL
}

/// The shared per-direction expectations, called once per direction with the
/// kind under test: the reading-start side hugs the reading-start edge, the
/// reading-end side hugs the reading-end edge, and the spacing gap between
/// them is respected.
fn assert_sides_layout(container: Rect, left: SideRects, right: SideRects, rtl: bool) {
    let (left_rect, _) = left;
    let (right_rect, _) = right;
    let spacing = default_item_spacing_x();

    // Both sides are as tall as the container's row and non-degenerate:
    assert!(
        left_rect.width() > 0.0 && right_rect.width() > 0.0,
        "both sides must be non-degenerate, got left {left_rect:?} right {right_rect:?}"
    );

    if rtl {
        // The reading-start side hugs the RIGHT edge (= reading start):
        assert!(
            near(left_rect.right(), container.right()),
            "rtl: the reading-start (left) side hugs the right edge, got {left_rect:?} in {container:?}"
        );
        // The reading-end side hugs the LEFT edge (= reading end):
        assert!(
            near(right_rect.left(), container.left()),
            "rtl: the reading-end (right) side hugs the left edge, got {right_rect:?} in {container:?}"
        );
        // The spacing gap between them is respected:
        assert!(
            left_rect.left() - right_rect.right() >= spacing - TOL,
            "rtl: the spacing gap must be respected, got left {left_rect:?} right {right_rect:?} (spacing {spacing})"
        );
    } else {
        // LTR is unchanged: the reading-start (left) side hugs the LEFT edge:
        assert!(
            near(left_rect.left(), container.left()),
            "ltr: the reading-start (left) side hugs the left edge, got {left_rect:?} in {container:?}"
        );
        assert!(
            near(right_rect.right(), container.right()),
            "ltr: the reading-end (right) side hugs the right edge, got {right_rect:?} in {container:?}"
        );
        assert!(
            right_rect.left() - left_rect.right() >= spacing - TOL,
            "ltr: the spacing gap must be respected, got left {left_rect:?} right {right_rect:?} (spacing {spacing})"
        );
    }
}

/// The reading-start side hugs the reading-start edge and the reading-end
/// side the reading-end edge, for every kind, under both directions.
#[test]
fn sides_follow_reading_direction() {
    for (name, sides) in [
        ("extend", Sides::new()),
        ("shrink_left", Sides::new().shrink_left()),
        ("shrink_right", Sides::new().shrink_right()),
    ] {
        let (container_ltr, left_ltr, right_ltr) =
            sides_layout(false, sides, LEFT_SIZE, RIGHT_SIZE);
        assert_sides_layout(container_ltr, left_ltr, right_ltr, false);

        let (container_rtl, left_rtl, right_rtl) = sides_layout(true, sides, LEFT_SIZE, RIGHT_SIZE);
        assert_sides_layout(container_rtl, left_rtl, right_rtl, true);

        // The names in the failure messages above are not enough to tell
        // which kind failed; re-assert with the kind in the message:
        assert!(
            near(container_ltr.min.x, container_rtl.min.x)
                && near(container_ltr.max.x, container_rtl.max.x),
            "{name}: the container must be identical under both directions, \
             got {container_ltr:?} vs {container_rtl:?}"
        );
    }
}

/// The point-reflection property: mirroring the RTL rects about the container
/// (x ↦ min.x + max.x − x, swapping min/max) reproduces the LTR rects exactly,
/// per kind. The content is direction-independent, so this must hold to the
/// pixel.
#[test]
fn sides_order_is_mirrored_between_directions() {
    for (name, sides) in [
        ("extend", Sides::new()),
        ("shrink_left", Sides::new().shrink_left()),
        ("shrink_right", Sides::new().shrink_right()),
    ] {
        let (container, left_ltr, right_ltr) = sides_layout(false, sides, LEFT_SIZE, RIGHT_SIZE);
        let (_, left_rtl, right_rtl) = sides_layout(true, sides, LEFT_SIZE, RIGHT_SIZE);

        for (role, ltr, rtl) in [
            ("left", left_ltr, left_rtl),
            ("right", right_ltr, right_rtl),
        ] {
            let mirrored_min_x = container.min.x + container.max.x - rtl.0.max.x;
            let mirrored_max_x = container.min.x + container.max.x - rtl.0.min.x;
            assert!(
                near(ltr.0.min.x, mirrored_min_x)
                    && near(ltr.0.max.x, mirrored_max_x)
                    && near(ltr.0.min.y, rtl.0.min.y)
                    && near(ltr.0.max.y, rtl.0.max.y),
                "{name}: the mirrored rtl {role} rect must equal the ltr rect, \
                 got ltr {:?} vs mirrored [{mirrored_min_x}, {mirrored_max_x}] × y {:?}",
                ltr.0,
                (rtl.0.min.y, rtl.0.max.y)
            );
        }
    }
}

/// Shrink priority is mirrored, not merely swapped: with an oversized
/// reading-start side (`shrink_left`), that side's max rect is capped to
/// `container.width − right_width − spacing` under BOTH directions (anchored
/// at the reading-start edge in each), and with `shrink_right` the roles
/// reverse.
#[test]
fn sides_shrink_priority_capped_in_both_directions() {
    let spacing = default_item_spacing_x();

    for (name, sides, oversized_left) in [
        ("shrink_left", Sides::new().shrink_left(), true),
        ("shrink_right", Sides::new().shrink_right(), false),
    ] {
        let (left_size, right_size) = if oversized_left {
            (OVERSIZED_LEFT_SIZE, RIGHT_SIZE)
        } else {
            (LEFT_SIZE, OVERSIZED_LEFT_SIZE)
        };
        let shrunk_is_left = oversized_left;

        for rtl in [false, true] {
            let (container, left, right) = sides_layout(rtl, sides, left_size, right_size);
            let (shrunk, other) = if shrunk_is_left {
                (left, right)
            } else {
                (right, left)
            };
            let cap = container.width() - other.0.width() - spacing;
            assert!(
                near(shrunk.1.width(), cap),
                "{name} (rtl={rtl}): the shrunk side's max rect must be capped to \
                 the remainder ({}), got {} in {container:?}",
                cap,
                shrunk.1.width()
            );
            // The shrunk constraint reaches the container edge its kind
            // names: `shrink_left` shrinks the reading-start side, so its
            // constraint touches the container's reading-start edge;
            // `shrink_right` the reading-end edge.
            let reading_start_edge = if rtl {
                container.max.x
            } else {
                container.min.x
            };
            let reading_end_edge = if rtl {
                container.min.x
            } else {
                container.max.x
            };
            let named_edge = if shrunk_is_left {
                reading_start_edge
            } else {
                reading_end_edge
            };
            assert!(
                named_edge >= shrunk.1.min.x - TOL && named_edge <= shrunk.1.max.x + TOL,
                "{name} (rtl={rtl}): the shrunk constraint must reach its kind's \
                 container edge ({named_edge}), got {:?} in {container:?}",
                shrunk.1
            );
        }
    }
}

/// An explicit `left_to_right` row under a global-RTL context wins: the
/// `Sides` inside it keeps its physical layout (explicit layout beats the
/// interface direction, consistent with [`egui::Grid`] and the `horizontal`
/// helpers).
#[test]
fn sides_inside_explicit_ltr_row_stays_physical() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let mut container = Rect::NOTHING;
    let mut left_rect = Rect::NOTHING;
    let mut right_rect = Rect::NOTHING;
    run(&ctx, input(SCREEN), |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                container = ui.available_rect_before_wrap();
                let (left, right) = Sides::new().show(
                    ui,
                    |ui| {
                        ui.allocate_exact_size(LEFT_SIZE, Sense::hover());
                        ui.min_rect()
                    },
                    |ui| {
                        ui.allocate_exact_size(RIGHT_SIZE, Sense::hover());
                        ui.min_rect()
                    },
                );
                left_rect = left;
                right_rect = right;
            });
        });
    });
    assert!(
        near(left_rect.left(), container.left()),
        "rtl: inside an explicit ltr row the reading-start side keeps its \
         physical left edge, got {left_rect:?} in {container:?}"
    );
    assert!(
        left_rect.right() < right_rect.left(),
        "rtl: inside an explicit ltr row the left side stays left of the \
         right side, got left {left_rect:?} right {right_rect:?}"
    );
}
