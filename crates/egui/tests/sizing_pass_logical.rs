//! Regression tests for direction-independent sizing passes: the measurement frame of
//! an [`egui::Area`]/[`egui::Popup`]/[`egui::Window`] measures a cross-`Align::Max`
//! body like an `Align::Min` body in both reading directions, the persisted
//! [`egui::AreaState::size`] matches across directions, and the sizing-pass
//! canonicalization never leaks into the visible frames.
//!
//! No snapshot assertions; geometry and persisted area state only.

use egui::{
    Align, AreaState, Context, Id, Layout, Popup, RawInput, Rect, SetOpenCommand, Ui, UiBuilder,
    pos2, vec2,
};

/// Run `n` frames on an 800x600 screen, dropping the texture deltas and shapes (these
/// tests never render).
fn run_frames(ctx: &Context, n: usize, mut f: impl FnMut(&mut Ui)) {
    for _ in 0..n {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| f(ui));
        out.textures_delta.clear();
        out.shapes.clear();
    }
}

/// A21.1: a sizing pass with a cross-`Align::Max` body (the mirrored-body shape)
/// measures the rows' natural width anchored on the reading-start edge, in both
/// directions: the measured width must not depend on the reading direction, the offered
/// width, or the cross alignment; the anchor edge follows the reading start (the left
/// edge under LTR, the right edge under RTL).
#[test]
fn sizing_pass_min_rect_is_direction_independent() {
    /// Measure a cross-`cross_align` sizing-pass body in an `offered_width`-wide rect,
    /// returning the offered rect and the measured `min_rect`.
    fn measure(rtl: bool, cross_align: Align, offered_width: f32) -> (Rect, Rect) {
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        let mut offered = Rect::NOTHING;
        let mut min_rect = Rect::NOTHING;
        run_frames(&ctx, 1, |ui| {
            offered = Rect::from_min_size(ui.cursor().min, vec2(offered_width, 200.0));
            let mut body = ui.new_child(
                UiBuilder::new()
                    .max_rect(offered)
                    .layout(Layout::top_down(cross_align))
                    .sizing_pass(),
            );
            let _ = body.button("Undo");
            let _ = body.button("Preferences");
            min_rect = body.min_rect();
        });
        (offered, min_rect)
    }

    for rtl in [false, true] {
        let direction = if rtl { "RTL" } else { "LTR" };
        let (offered, max_narrow) = measure(rtl, Align::Max, 300.0);
        let (_, max_wide) = measure(rtl, Align::Max, 500.0);
        let (_, min_narrow) = measure(rtl, Align::Min, 300.0);

        assert!(
            max_narrow.width() < 200.0,
            "{direction}: the cross-Max sizing pass should measure the rows' natural width, \
             not the offered width: {max_narrow:?}"
        );
        if rtl {
            assert!(
                (max_narrow.max.x - offered.max.x).abs() <= 0.5,
                "{direction}: the cross-Max sizing pass should anchor the measurement on the \
                 reading-start (right) edge of the offered rect {offered:?}, got {max_narrow:?}"
            );
        } else {
            assert!(
                (max_narrow.min.x - offered.min.x).abs() <= 0.5,
                "{direction}: the cross-Max sizing pass should anchor the measurement on the \
                 reading-start (left) edge of the offered rect {offered:?}, got {max_narrow:?}"
            );
        }
        assert!(
            (max_narrow.width() - max_wide.width()).abs() <= 0.5,
            "{direction}: the cross-Max sizing measurement must be offered-width-independent \
             (300 px offer {max_narrow:?}, 500 px offer {max_wide:?})"
        );
        assert!(
            (max_narrow.min.x - min_narrow.min.x).abs() <= 0.5
                && (max_narrow.max.x - min_narrow.max.x).abs() <= 0.5,
            "{direction}: cross-Max sizing should measure like cross-Min \
             (Max {max_narrow:?}, Min {min_narrow:?})"
        );
    }
}

/// A21.5: the root-`Ui` half of the sizing-pass canonicalization ([`Ui::new`]) also holds
/// in LTR, where it newly fires: a vertical cross-`Align::Max` root body over an offered
/// rect measures the rows' natural width anchored at the offered rect's left edge,
/// offered-width independent, like its cross-`Align::Min` counterpart. This mirrors the
/// child-path probe above, but builds the body with [`Ui::new`] instead of
/// [`Ui::new_child`] (the root path is the one the areas' content `Ui`s take).
#[test]
fn root_sizing_pass_min_rect_is_left_anchored_in_ltr() {
    /// Measure a cross-`cross_align` root (`Ui::new`) sizing-pass body in an
    /// `offered_width`-wide rect, returning the offered rect and the measured
    /// `min_rect`.
    fn measure(cross_align: Align, offered_width: f32) -> (Rect, Rect) {
        let ctx = Context::default();
        let mut offered = Rect::NOTHING;
        let mut min_rect = Rect::NOTHING;
        run_frames(&ctx, 1, |ui| {
            offered = Rect::from_min_size(ui.cursor().min, vec2(offered_width, 200.0));
            let mut root = Ui::new(
                ctx.clone(),
                Id::unique("root-sizing-probe"),
                UiBuilder::new()
                    .max_rect(offered)
                    .layout(Layout::top_down(cross_align))
                    .sizing_pass(),
            );
            let _ = root.button("Undo");
            let _ = root.button("Preferences");
            min_rect = root.min_rect();
        });
        (offered, min_rect)
    }

    let (offered, max_narrow) = measure(Align::Max, 300.0);
    let (_, max_wide) = measure(Align::Max, 500.0);
    let (_, min_narrow) = measure(Align::Min, 300.0);

    assert!(
        max_narrow.width() < 200.0,
        "the cross-Max root sizing pass should measure the rows' natural width, \
         not the offered width: {max_narrow:?}"
    );
    assert!(
        (max_narrow.min.x - offered.min.x).abs() <= 0.5,
        "the cross-Max root sizing pass should anchor the measurement on the left edge \
         of the offered rect {offered:?}, got {max_narrow:?}"
    );
    assert!(
        (max_narrow.min.x - max_wide.min.x).abs() <= 0.5
            && (max_narrow.max.x - max_wide.max.x).abs() <= 0.5
            && (max_narrow.top() - max_wide.top()).abs() <= 0.5
            && (max_narrow.bottom() - max_wide.bottom()).abs() <= 0.5,
        "the cross-Max root sizing measurement must be offered-width-independent \
         (300 px offer {max_narrow:?}, 500 px offer {max_wide:?})"
    );
    assert!(
        (max_narrow.min.x - min_narrow.min.x).abs() <= 0.5
            && (max_narrow.max.x - min_narrow.max.x).abs() <= 0.5,
        "cross-Max root sizing should measure like cross-Min \
         (Max {max_narrow:?}, Min {min_narrow:?})"
    );
}

/// A21.2: a menu popup's persisted [`AreaState::size`] width matches in both directions:
/// the RTL sizing pass must not lock a direction-dependent width into the area state
/// that every steady frame reuses.
#[test]
fn popup_width_matches_in_rtl_and_ltr() {
    fn popup_width(rtl: bool) -> f32 {
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        let mut popup_id = Id::NULL;
        run_frames(&ctx, 3, |ui| {
            let button = ui.button("Layer");
            popup_id = Popup::default_response_id(&button);
            Popup::menu(&button)
                .open_memory(Some(SetOpenCommand::Bool(true)))
                .show(|ui| {
                    ui.set_min_width(220.0);
                    let _ = ui.button("Undo");
                    let _ = ui.button("Preferences");
                });
        });
        AreaState::load(&ctx, popup_id)
            .and_then(|state| state.size)
            .map(|size| size.x)
            .unwrap_or(0.0)
    }

    let ltr = popup_width(false);
    let rtl = popup_width(true);
    assert!(
        (ltr - rtl).abs() < 1.0,
        "the menu popup's persisted width must match in both directions: ltr {ltr}, rtl {rtl}"
    );
    assert!(
        ltr > 100.0 && rtl > 100.0,
        "the menu popup's measured width is degenerate: ltr {ltr}, rtl {rtl}"
    );
}

/// A21.3: the sizing pass does not leak into the visible frames: after the sizing pass,
/// the steady-frame body still reports the mirrored cross alignment and direction flag,
/// and its layout stays right-anchored in the offered rect.
#[test]
fn visible_frame_keeps_the_mirrored_layout() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);

    let mut steady: Option<(bool, Align, Rect, Rect)> = None;
    let mut frame = 0_usize;
    run_frames(&ctx, 3, |ui| {
        frame += 1;
        let offered = Rect::from_min_size(ui.cursor().min, vec2(300.0, 200.0));
        let mut builder = UiBuilder::new()
            .max_rect(offered)
            .layout(Layout::top_down(Align::Max));
        if frame == 1 {
            builder = builder.sizing_pass();
        }
        let mut body = ui.new_child(builder);
        let _ = body.button("Undo");
        let _ = body.button("Preferences");
        if frame > 1 {
            steady = Some((
                body.style().right_to_left,
                body.layout().cross_align(),
                offered,
                body.min_rect(),
            ));
        }
    });

    let (right_to_left, cross_align, offered, min_rect) =
        steady.expect("frames 2-3 should have populated the steady state");
    assert!(
        right_to_left,
        "the steady frame lost the right-to-left direction flag"
    );
    assert_eq!(
        cross_align,
        Align::Max,
        "the steady frame lost the mirrored cross alignment"
    );
    assert!(
        (min_rect.min.x - (offered.right() - min_rect.width())).abs() <= 0.5,
        "the steady frame should keep the body right-anchored in the offered rect \
         {offered:?}, got {min_rect:?}"
    );
}

/// A21.4: the sizing-pass canonicalization mutates only the child's local layout copy:
/// the direction flag and the caller's layout are untouched during and after the pass,
/// so there is nothing to restore. Under RTL the construction-time mirror resolves the
/// vertical `Align::Min` root body (and the canonicalized `Align::Max` child, after its
/// canonicalization) to the effective `Align::Max` the placer carries.
#[test]
fn sizing_pass_canonicalization_does_not_leak() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);

    run_frames(&ctx, 1, |ui| {
        assert_eq!(
            ui.layout().cross_align(),
            Align::Max,
            "the vertical Align::Min parent body should carry the mirrored cross alignment"
        );
        let parent_align_before = ui.layout().cross_align();

        // (a) During an RTL cross-Max sizing pass the child's layout is a local copy —
        // canonicalized to `Min` for the measurement frame, then mirrored back — and
        // the direction flag is NOT cleared:
        let mut sizing = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(ui.cursor().min, vec2(300.0, 200.0)))
                .layout(Layout::top_down(Align::Max))
                .sizing_pass(),
        );
        assert_eq!(
            sizing.layout().cross_align(),
            Align::Max,
            "the sizing child should resolve to the mirrored effective cross alignment"
        );
        assert!(
            sizing.is_rtl(),
            "the sizing pass must not clear the direction flag"
        );
        let _ = sizing.button("Undo");
        let _ = sizing.button("Preferences");

        // (b) A later child built without the sizing flag keeps the mirrored layout:
        let mut visible = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(ui.cursor().min, vec2(300.0, 200.0)))
                .layout(Layout::top_down(Align::Max)),
        );
        assert_eq!(
            visible.layout().cross_align(),
            Align::Max,
            "a non-sizing child must keep the mirrored cross alignment"
        );
        assert!(
            visible.is_rtl(),
            "a non-sizing child keeps the direction flag"
        );
        let _ = visible.button("Undo");
        let _ = visible.button("Preferences");

        // (c) Nothing shared was mutated by the sizing pass:
        assert_eq!(
            ui.layout().cross_align(),
            parent_align_before,
            "the parent's layout must be untouched by the sizing pass"
        );
        assert!(
            ctx.is_rtl(),
            "the context direction must stay right-to-left"
        );
    });
}
