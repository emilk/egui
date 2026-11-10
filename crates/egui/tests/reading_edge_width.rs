//! Regression tests for the anchoring semantics of the width setters
//! ([`egui::Ui::set_min_width`] / [`egui::Ui::set_max_width`] and everything
//! built on them: `set_width`, `set_width_range`, `take_available_width`,
//! `shrink_width_to_current`).
//!
//! The setters claim their span with the layout-derived anchor
//! ([`egui::Layout::horizontal_align`]): a vertical body claims from its
//! effective `cross_align` edge — mirrored to the reading-start edge for a
//! vertical `Align::Min` body under a right-to-left interface — so the
//! measured sizes (widths) stay direction-independent. These tests pin that
//! decision: a vertical `Align::Min` body measures the claim (never the
//! offered span) in both directions, and bodies whose layout is
//! reading-start-pinned under a right-to-left interface (the `Align::Max`
//! bodies popups and menus use) cap at the reading edge.
//!
//! Since the default `Area` pivot follows the reading direction (a fresh
//! `Area` roots on its reading-start edge: `LEFT_TOP` under LTR, `RIGHT_TOP`
//! under RTL), the fresh-Area measurements mirror in position: the
//! reading-start edge pins at the fixed pos in both directions, and the RTL
//! rect is the LTR rect mirrored about it. The width claims themselves stay
//! direction-independent.
//!
//! Every mirrored case carries its LTR control; several tests assert
//! LTR-rect == RTL-rect (direction independence) rather than absolute
//! geometry. No snapshot assertions.

use core::iter::zip;

use egui::{
    Align, Area, Context, Id, Layout, Pos2, RawInput, Rect, Ui, UiBuilder, Vec2, pos2, vec2,
};

/// The screen the fresh-Area tests run on. It must stay wider than
/// `FIXED_POS.x + 600` — see [`area_rects`].
const SCREEN: Vec2 = vec2(1300.0, 600.0);

/// Where the fresh-Area tests pin their area.
const FIXED_POS: Pos2 = pos2(600.0, 100.0);

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

/// Runs `body` inside a fresh `Area` at [`FIXED_POS`] on a 1300-wide screen
/// for two frames (`run_ui` twice) and returns the area's surface rect plus
/// the measured `min_rect` and `max_rect` after each frame.
///
/// The default pivot follows the reading direction: `LEFT_TOP` under LTR (the
/// left edge is pinned at 600), `RIGHT_TOP` under RTL (the right edge is
/// pinned at 600). In both directions the pinned edge survives ONLY because
/// the guessed 600-wide sizing-pass rect fits the 1300-wide constrain rect:
/// LTR spans 600..1200, RTL 0..600 — `Area::begin` runs
/// `Context::constrain_window_rect_to_area` every frame and would otherwise
/// move the pivot. Keep `FIXED_POS.x >= 600` and the screen wider than
/// `FIXED_POS.x + 600`.
///
/// Frame 1 is the Area's sizing pass over the guessed `default_area_size`
/// span, frame 2 the show pass over the persisted size (which `Prepared::end`
/// persists every frame).
fn area_rects(ctx: &Context, body: impl FnMut(&mut Ui) + Clone) -> [(Rect, Rect, Rect); 2] {
    let id = Id::unique("reading_edge_width_area");
    let mut rects = [(Rect::NOTHING, Rect::NOTHING, Rect::NOTHING); 2];
    for (area, min_rect, max_rect) in &mut rects {
        let mut body = body.clone();
        run(ctx, input(SCREEN), |ui| {
            let response = Area::new(id).fixed_pos(FIXED_POS).show(ui, |ui| {
                body(ui);
                *min_rect = ui.min_rect();
                *max_rect = ui.max_rect();
            });
            *area = response.response.rect;
        });
    }
    rects
}

/// In a vertical `Align::Min` body the width setters measure the claim, not
/// the offered span — identically under LTR and RTL.
#[test]
fn min_width_measures_the_claim_in_both_directions() {
    // (a) Fresh Area at a fixed pos: every frame measures the 220-wide claim,
    // and the area surface roots on its reading-start edge — the left edge
    // under LTR, the right edge under RTL (see `area_rects`).
    let ltr_ctx = Context::default();
    let ltr = area_rects(&ltr_ctx, |ui| ui.set_min_width(220.0));

    let rtl_ctx = Context::default();
    rtl_ctx.set_right_to_left(true);
    let rtl = area_rects(&rtl_ctx, |ui| ui.set_min_width(220.0));

    for (frame, ((ltr_area, ltr_min, _), (rtl_area, rtl_min, _))) in zip(ltr, rtl).enumerate() {
        assert!(
            (ltr_min.left() - FIXED_POS.x).abs() <= 0.5,
            "LTR frame {frame}: the measured left edge should stay at the fixed pos, \
             got {ltr_min:?}"
        );
        assert!(
            (rtl_area.right() - FIXED_POS.x).abs() <= 0.5,
            "RTL frame {frame}: the area should root on its reading-start (right) edge at the \
             fixed pos, got {rtl_area:?}"
        );
        for (name, min_rect) in [("LTR", ltr_min), ("RTL", rtl_min)] {
            assert!(
                (min_rect.width() - 220.0).abs() <= 0.5,
                "{name} frame {frame}: the measured width should be the 220-wide claim, \
                 not the offered span, got {min_rect:?}"
            );
        }
        assert!(
            (rtl_area.right() - ltr_area.left()).abs() <= 0.5
                && (rtl_area.width() - ltr_area.width()).abs() <= 0.5,
            "frame {frame}: the area surface should mirror about the reading-start edge \
             (LTR {ltr_area:?}, RTL {rtl_area:?})"
        );
    }

    // (b) Wide body: the near-edge seed and the near-edge claim coincide, so
    // the measurement stays the claim (220) instead of ballooning to the
    // offered span (400). This is the regression pin against reintroducing a
    // far-edge claim, which would union the near-edge seed with the far-edge
    // claim into the full offered span.
    let offered = Rect::from_min_size(pos2(200.0, 0.0), vec2(400.0, 300.0));
    for rtl in [false, true] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let mut min_rect = Rect::NOTHING;
        let mut max_rect = Rect::NOTHING;
        run(&ctx, input(SCREEN), |ui| {
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(offered)
                    .layout(Layout::top_down(Align::Min)),
            );
            child.set_min_width(220.0);
            min_rect = child.min_rect();
            max_rect = child.max_rect();
        });
        assert!(
            (min_rect.width() - 220.0).abs() <= 0.5,
            "rtl = {rtl}: the wide body should measure the 220-wide claim, \
             not the 400-wide offered span, got {min_rect:?}"
        );
        // The near edge follows the body's effective cross align: the left
        // edge under LTR, mirrored to the reading-start (right) edge under RTL.
        if rtl {
            assert_eq!(
                min_rect.right(),
                max_rect.right(),
                "rtl = {rtl}: the measurement should start on the body's content \
                 (near) edge, got min_rect {min_rect:?}, max_rect {max_rect:?}"
            );
        } else {
            assert_eq!(
                min_rect.left(),
                max_rect.left(),
                "rtl = {rtl}: the measurement should start on the body's content \
                 (near) edge, got min_rect {min_rect:?}, max_rect {max_rect:?}"
            );
        }
    }
}

/// `set_width_range` measures min and max as in LTR, in every frame.
#[test]
fn width_range_measures_min_and_max_in_both_directions() {
    let ltr_ctx = Context::default();
    let ltr = area_rects(&ltr_ctx, |ui| ui.set_width_range(120.0..=300.0));

    let rtl_ctx = Context::default();
    rtl_ctx.set_right_to_left(true);
    let rtl = area_rects(&rtl_ctx, |ui| ui.set_width_range(120.0..=300.0));

    for (frame, ((ltr_area, ltr_min, ltr_max), (rtl_area, rtl_min, rtl_max))) in
        zip(ltr, rtl).enumerate()
    {
        for (name, min_rect, max_rect) in [("LTR", ltr_min, ltr_max), ("RTL", rtl_min, rtl_max)] {
            assert!(
                (min_rect.width() - 120.0).abs() <= 0.5,
                "{name} frame {frame}: min_rect should measure the 120-wide minimum, \
                 got {min_rect:?}"
            );
            assert!(
                (max_rect.width() - 300.0).abs() <= 0.5,
                "{name} frame {frame}: max_rect should hold the 300-wide maximum, \
                 got {max_rect:?}"
            );
            assert!(
                max_rect.contains_rect(min_rect),
                "{name} frame {frame}: max_rect {max_rect:?} should contain min_rect {min_rect:?}"
            );
        }
        assert!(
            (ltr_area.left() - FIXED_POS.x).abs() <= 0.5,
            "LTR frame {frame}: the area should root on its left edge at the fixed pos, \
             got {ltr_area:?}"
        );
        assert!(
            (rtl_area.right() - FIXED_POS.x).abs() <= 0.5,
            "RTL frame {frame}: the area should root on its reading-start (right) edge at the \
             fixed pos, got {rtl_area:?}"
        );
        assert!(
            (rtl_area.right() - ltr_area.left()).abs() <= 0.5
                && (rtl_area.width() - ltr_area.width()).abs() <= 0.5,
            "frame {frame}: the area surface should mirror about the reading-start edge \
             (LTR {ltr_area:?}, RTL {rtl_area:?})"
        );
    }
}

/// `set_max_width` below the minimum is decided by the `max_rect |= min_rect`
/// guard, not by the anchor — identical in both directions.
#[test]
fn max_width_guard_keeps_the_minimum_in_both_directions() {
    let clamp = |ui: &mut Ui| {
        ui.set_min_width(200.0);
        ui.set_max_width(100.0);
    };

    let ltr_ctx = Context::default();
    let ltr = area_rects(&ltr_ctx, clamp);

    let rtl_ctx = Context::default();
    rtl_ctx.set_right_to_left(true);
    let rtl = area_rects(&rtl_ctx, clamp);

    for (frame, ((ltr_area, ltr_min, ltr_max), (rtl_area, rtl_min, rtl_max))) in
        zip(ltr, rtl).enumerate()
    {
        for (name, min_rect, max_rect) in [("LTR", ltr_min, ltr_max), ("RTL", rtl_min, rtl_max)] {
            assert!(
                (max_rect.width() - 200.0).abs() <= 0.5,
                "{name} frame {frame}: the 200-wide minimum must survive the \
                 100-wide cap, got max_rect {max_rect:?}"
            );
            assert!(
                max_rect.contains_rect(min_rect),
                "{name} frame {frame}: max_rect {max_rect:?} should contain min_rect {min_rect:?}"
            );
        }
        assert!(
            (ltr_area.left() - FIXED_POS.x).abs() <= 0.5,
            "LTR frame {frame}: the area should root on its left edge at the fixed pos, \
             got {ltr_area:?}"
        );
        assert!(
            (rtl_area.right() - FIXED_POS.x).abs() <= 0.5,
            "RTL frame {frame}: the area should root on its reading-start (right) edge at the \
             fixed pos, got {rtl_area:?}"
        );
        assert!(
            (rtl_area.right() - ltr_area.left()).abs() <= 0.5
                && (rtl_area.width() - ltr_area.width()).abs() <= 0.5,
            "frame {frame}: the area surface should mirror about the reading-start edge \
             (LTR {ltr_area:?}, RTL {rtl_area:?})"
        );
    }
}

/// Bodies whose layout is reading-start-pinned cap at the reading edge (the
/// `Align::Max` cross bodies popups and menus use under a right-to-left
/// interface — already true, pinned); everything else is flag-invariant.
#[test]
fn max_pinned_bodies_cap_at_the_reading_edge_and_the_rest_is_flag_invariant() {
    // 1. A reading-start-pinned body caps at the reading edge: the right-edge
    // seed keeps the `set_max_width` union guard coherent.
    {
        let ctx = Context::default();
        ctx.set_right_to_left(true);
        let offered = Rect::from_min_size(pos2(200.0, 0.0), vec2(400.0, 300.0));
        let mut max_rect = Rect::NOTHING;
        run(&ctx, input(SCREEN), |ui| {
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(offered)
                    .layout(Layout::top_down_justified(Align::Max)),
            );
            child.set_max_width(120.0);
            max_rect = child.max_rect();
        });
        assert!(
            (max_rect.right() - offered.right()).abs() <= 0.5,
            "RTL: the cap should sit at the reading-start (right) edge of the \
             offered span, got {max_rect:?}"
        );
        assert!(
            (max_rect.width() - 120.0).abs() <= 0.5,
            "RTL: the body should be capped at the 120-wide claim, got {max_rect:?}"
        );
    }

    // 2. Everything else is flag-invariant: each scope runs twice (flag off
    // and on) with identical layout, and the captured rects must be equal
    // between the runs.
    let offered = Rect::from_min_size(pos2(200.0, 0.0), vec2(400.0, 300.0));

    // 2a. Horizontal rows: the main-direction edge anchors the frame; the
    // interface flag must not matter.
    {
        let mut captured = [Rect::NOTHING; 2];
        for (i, rtl) in [false, true].into_iter().enumerate() {
            let ctx = Context::default();
            if rtl {
                ctx.set_right_to_left(true);
            }
            run(&ctx, input(SCREEN), |ui| {
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .max_rect(offered)
                        .layout(Layout::right_to_left(Align::Min)),
                );
                child.set_min_width(60.0);
                captured[i] = child.min_rect();
            });
        }
        assert_eq!(
            captured[0], captured[1],
            "horizontal rows must produce identical rects with the flag off and on"
        );
    }

    // 2b. `Align::Center` cross bodies: center is direction-neutral.
    {
        let mut captured = [Rect::NOTHING; 2];
        for (i, rtl) in [false, true].into_iter().enumerate() {
            let ctx = Context::default();
            if rtl {
                ctx.set_right_to_left(true);
            }
            run(&ctx, input(SCREEN), |ui| {
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .max_rect(offered)
                        .layout(Layout::top_down(Align::Center)),
                );
                child.set_min_width(50.0);
                captured[i] = child.min_rect();
            });
        }
        assert_eq!(
            captured[0], captured[1],
            "cross-Center bodies must produce identical rects with the flag off and on"
        );
    }

    // 2c. A claim that fills the available span (the `Ui::columns` shape) is
    // the same span from either edge, so it must be flag-invariant.
    {
        let mut captured = [Rect::NOTHING; 2];
        for (i, rtl) in [false, true].into_iter().enumerate() {
            let ctx = Context::default();
            if rtl {
                ctx.set_right_to_left(true);
            }
            run(&ctx, input(SCREEN), |ui| {
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .max_rect(offered)
                        .layout(Layout::top_down(Align::Min)),
                );
                child.set_width(child.available_width());
                captured[i] = child.min_rect();
            });
        }
        assert_eq!(
            captured[0], captured[1],
            "a claim filling the available span must produce identical rects \
             with the flag off and on"
        );
        assert!(
            (captured[0].width() - offered.width()).abs() <= 0.5,
            "the full-width claim should measure the offered span, got {:?}",
            captured[0]
        );
    }
}

/// The width setters' empty-body anchor: a bare `set_min_width(200)` in a vertical
/// `Align::Center` body claims the centered span `[220..420]` of the 8..632 offered
/// rect, and the measured y stays the seed row. A claim always contains the seed point
/// on its axis (the first `expand_to_include_x` yields the same span either way), so
/// the seed semantics must not change it: the claim anchors at the layout-derived
/// center and never balloons to the offered span.
#[test]
fn centered_body_bare_claim_keeps_the_layout_anchored_span() {
    for rtl in [false, true] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let offered = Rect::from_min_max(pos2(8.0, 8.0), pos2(632.0, 472.0));
        let mut min_rect = Rect::NOTHING;
        run(&ctx, input(SCREEN), |ui| {
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(offered)
                    .layout(Layout::top_down(Align::Center)),
            );
            child.set_min_width(200.0);
            min_rect = child.min_rect();
        });
        assert_eq!(
            min_rect,
            Rect::from_min_max(pos2(220.0, 8.0), pos2(420.0, 8.0)),
            "rtl = {rtl}: the bare claim must measure the centered 200-wide span with \
             the seed-row y, got {min_rect:?}"
        );
    }
}
