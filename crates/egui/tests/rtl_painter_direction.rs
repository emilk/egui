//! Regression tests for the reading-direction painter vocabulary
//! ([`egui::Ui::reading_start_x`], [`egui::Ui::reading_start_x_w`],
//! [`egui::Ui::reading_end_x`], [`egui::Ui::start_align2`], [`egui::Ui::end_align2`],
//! [`egui::Ui::forward`]), the [`egui::Image::mirrored_x`] UV flip, and their progress-bar
//! consumers: the fill and the indeterminate arc.
//!
//! Every mirrored case also asserts its LTR counterpart (flag off), so this file doubles
//! as the LTR-unchanged regression. No snapshot assertions; geometry, UV and painted
//! shapes only.

use egui::epaint::{PathShape, RectShape, Shape, TextureId};
use egui::{
    Align2, CentralPanel, Color32, Context, FullOutput, Image, Pos2, ProgressBar, RawInput, Rect,
    Rgba, Ui, Vec2, load::SizedTexture, pos2, vec2,
};

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

fn input_at(size: Vec2, time: f64) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(time),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
    output
}

/// A context with the direction flag set BEFORE the first frame, so the flag reaches the
/// first mirrored pass (setting it inside a `run_ui` closure would land one frame late).
fn ctx_with_direction(rtl: bool) -> Context {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    ctx
}

/// The reading-start/end helpers hug the reading edge of the given rect in both
/// directions, and the width-aware start helper grows the anchored element inward.
#[test]
fn reading_start_x_hugs_the_reading_edge_in_both_directions() {
    fn probe(rtl: bool) -> (Rect, f32, f32, f32) {
        let ctx = ctx_with_direction(rtl);
        let mut captured = (Rect::NOTHING, f32::NAN, f32::NAN, f32::NAN);
        run(&ctx, input(vec2(400.0, 300.0)), |ui| {
            let r = Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 8.0));
            captured = (
                r,
                ui.reading_start_x(r, 7.0),
                ui.reading_start_x_w(r, 0.0, 25.0),
                ui.reading_end_x(r, 7.0),
            );
        });
        captured
    }

    let (r, start, start_w, end) = probe(false);
    assert!(
        (start - (r.left() + 7.0)).abs() < 1e-4,
        "LTR: reading_start_x should offset inward from the left edge"
    );
    assert!(
        (start_w - r.left()).abs() < 1e-4,
        "LTR: reading_start_x_w should return the left edge at zero margin"
    );
    assert!(
        (end - (r.right() - 7.0)).abs() < 1e-4,
        "LTR: reading_end_x should offset inward from the right edge"
    );

    let (r, start, start_w, end) = probe(true);
    assert!(
        (start - (r.right() - 7.0)).abs() < 1e-4,
        "RTL: reading_start_x should offset inward from the right edge"
    );
    assert!(
        (start_w - (r.right() - 25.0)).abs() < 1e-4,
        "RTL: reading_start_x_w should return the min-x of a 25 px element \
         hugging the right edge"
    );
    assert!(
        (end - (r.left() + 7.0)).abs() < 1e-4,
        "RTL: reading_end_x should offset inward from the left edge"
    );
}

/// The align helpers mirror and `forward` flips, so
/// `reading_start_x(rect, 0.0) + forward() * d` walks the reading direction in both.
#[test]
fn start_and_end_align2_mirror() {
    fn probe(rtl: bool) -> (Align2, Align2, f32, Rect, f32) {
        let ctx = ctx_with_direction(rtl);
        let mut captured = (
            Align2::LEFT_TOP,
            Align2::LEFT_TOP,
            f32::NAN,
            Rect::NOTHING,
            f32::NAN,
        );
        run(&ctx, input(vec2(400.0, 300.0)), |ui| {
            let r = Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 8.0));
            let forward = ui.forward();
            captured = (
                ui.start_align2(),
                ui.end_align2(),
                forward,
                r,
                ui.reading_start_x(r, 0.0) + forward * 8.0,
            );
        });
        captured
    }

    let (start, end, forward, r, walk) = probe(false);
    assert_eq!(start, Align2::LEFT_CENTER, "LTR: text hugs the left edge");
    assert_eq!(
        end,
        Align2::RIGHT_CENTER,
        "LTR: end text hugs the right edge"
    );
    assert!((forward - 1.0).abs() < 1e-6, "LTR: forward is +1");
    assert!(
        (walk - (r.left() + 8.0)).abs() < 1e-4,
        "LTR: walking 8 px forward from the reading start moves right"
    );

    let (start, end, forward, r, walk) = probe(true);
    assert_eq!(start, Align2::RIGHT_CENTER, "RTL: text hugs the right edge");
    assert_eq!(end, Align2::LEFT_CENTER, "RTL: end text hugs the left edge");
    assert!((forward + 1.0).abs() < 1e-6, "RTL: forward is -1");
    assert!(
        (walk - (r.right() - 8.0)).abs() < 1e-4,
        "RTL: walking 8 px forward from the reading start moves left"
    );
}

/// The bar rect and the one rect painted in the widget's own fill colour
/// (`selection.bg_fill` at `color_factor == bright == 1.0`, the non-animated fill),
/// so the colour match is exact by construction.
fn painted_fill(rtl: bool, progress: f32) -> (Rect, Rect) {
    let ctx = ctx_with_direction(rtl);
    let mut bar = Rect::NOTHING;
    let mut fill_color = Color32::BLACK;
    let output = run(&ctx, input(vec2(400.0, 300.0)), |ui| {
        CentralPanel::default().show(ui, |ui| {
            fill_color = Color32::from(Rgba::from(ui.style().visuals.selection.bg_fill) * 1.0);
            bar = ui.add(ProgressBar::new(progress).desired_width(100.0)).rect;
        });
    });
    let fills: Vec<Rect> = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(RectShape { rect, fill, .. }) if *fill == fill_color => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(
        fills.len(),
        1,
        "expected exactly one fill rect (rtl = {rtl}, progress = {progress}), got {fills:?}"
    );
    (bar, fills[0])
}

/// The progress fill hugs the reading start — left-anchored under LTR, right-anchored
/// under RTL, same width in both; the min-width floor and the 0..=1 clamp apply in both.
#[test]
fn progress_fill_hugs_the_reading_start() {
    for rtl in [false, true] {
        // At 25 % the fill is a quarter of the 100 px bar, anchored at the reading start:
        let (bar, fill) = painted_fill(rtl, 0.25);
        if rtl {
            assert_eq!(
                fill.right(),
                bar.right(),
                "RTL: the fill should hug the bar's right edge (fill {fill:?}, bar {bar:?})"
            );
        } else {
            assert_eq!(
                fill.left(),
                bar.left(),
                "LTR: the fill should hug the bar's left edge (fill {fill:?}, bar {bar:?})"
            );
        }
        assert!(
            (fill.width() - 25.0).abs() < 0.5,
            "25 % of a 100 px bar should paint 25 px (fill {fill:?}, bar {bar:?})"
        );

        // The min-width floor: at 0.5 % the raw fill (0.5 px) is clamped up so the rounded
        // caps stay visible. `min_width` is `2 * max(cap radii).at_most(half_height)`, so
        // with the default corner radius (half_height) the floor is the full bar height,
        // in BOTH directions:
        let (bar, fill) = painted_fill(rtl, 0.005);
        if rtl {
            assert_eq!(
                fill.right(),
                bar.right(),
                "RTL: the floor cap should hug the bar's right edge (fill {fill:?}, bar {bar:?})"
            );
        } else {
            assert_eq!(
                fill.left(),
                bar.left(),
                "LTR: the floor cap should hug the bar's left edge (fill {fill:?}, bar {bar:?})"
            );
        }
        assert!(
            (fill.width() - bar.height()).abs() < 0.5,
            "the min-width floor should keep the rounded caps visible \
             (fill {fill:?}, bar {bar:?})"
        );

        // The constructor clamps progress to 0..=1, so 2.0 fills the whole bar:
        let (bar, fill) = painted_fill(rtl, 2.0);
        assert!(
            (fill.left() - bar.left()).abs() < 1e-3 && (fill.right() - bar.right()).abs() < 1e-3,
            "a clamped progress of 1.0 should fill the whole bar (fill {fill:?}, bar {bar:?})"
        );
    }
}

/// The indeterminate arc orbits the fill's progress-front edge and sweeps the mirrored
/// way: at equal times each RTL arc point is the x-mirror of the LTR point within the bar
/// (same count, same y), so the loading indicator mirrors about the bar's centre line.
#[test]
fn progress_indeterminate_arc_mirrors_about_the_bar_center() {
    /// The bar rect and the arc's points, at a fixed time.
    fn arc(rtl: bool, time: f64) -> (Rect, Vec<Pos2>) {
        let ctx = ctx_with_direction(rtl);
        let mut bar = Rect::NOTHING;
        let output = run(&ctx, input_at(vec2(400.0, 300.0), time), |ui| {
            CentralPanel::default().show(ui, |ui| {
                bar = ui
                    .add(ProgressBar::new(0.5).animate(true).desired_width(100.0))
                    .rect;
            });
        });
        let mut paths: Vec<Vec<Pos2>> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Path(PathShape { points, .. }) => Some(points.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            paths.len(),
            1,
            "a bare panel with one progress bar paints exactly one path, the arc, got {paths:?}"
        );
        (bar, paths.remove(0))
    }

    for time in [0.0_f64, 0.3, 1.7] {
        let (ltr_bar, ltr) = arc(false, time);
        let (rtl_bar, rtl) = arc(true, time);
        assert_eq!(ltr.len(), rtl.len(), "same point count at time {time}");
        assert!(
            (ltr_bar.width() - rtl_bar.width()).abs() < 1e-3,
            "the bar allocates the same size in both directions"
        );
        for (l, r) in core::iter::zip(&ltr, &rtl) {
            // The offsets of each mirrored pair within their own bars sum to the bar
            // width. Formulated per bar, so it holds wherever each direction places it:
            // the orbit stays anchored at the fill's progress-front edge and the sweep
            // flips, which is the arc mirrored about the bar's centre line.
            let ltr_into = l.x - ltr_bar.left();
            let rtl_into = r.x - rtl_bar.left();
            assert!(
                (ltr_into + rtl_into - ltr_bar.width()).abs() < 1e-3,
                "time {time}: the RTL arc should mirror the LTR arc within the bar \
                 (ltr into {ltr_into:?}, rtl into {rtl_into:?}, bar width {})",
                ltr_bar.width()
            );
            assert!(
                (r.y - l.y).abs() < 1e-4,
                "time {time}: the arc's y should be untouched by the mirror"
            );
        }
    }
}

/// `mirrored_x` flips the UV rect about x, and a later explicit `uv` call wins,
/// as with any builder.
#[test]
fn mirrored_x_flips_the_uv_rect() {
    let unit = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let mirrored = Rect::from_min_max(pos2(1.0, 0.0), pos2(0.0, 1.0));

    assert_eq!(
        Image::new("").image_options().uv,
        unit,
        "the default UV is the unit rect"
    );
    assert_eq!(
        Image::new("").mirrored_x().image_options().uv,
        mirrored,
        "mirrored_x flips the UV rect about x"
    );
    assert_eq!(
        Image::new("").mirrored_x().uv(unit).image_options().uv,
        unit,
        "a later explicit uv overrides the mirror"
    );
    assert_eq!(
        Image::new("").uv(unit).mirrored_x().image_options().uv,
        mirrored,
        "an earlier explicit uv does not survive a later mirrored_x"
    );
}

/// All rects textured with `id`: their painted rect and UV rect.
fn textured_rects(output: &FullOutput, id: TextureId) -> Vec<(Rect, Rect)> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rs) => {
                let brush = rs.brush.as_deref()?;
                (brush.fill_texture_id == id).then_some((rs.rect, brush.uv))
            }
            _ => None,
        })
        .collect()
}

/// The mirrored UV reaches the painted shape: a plain image paints the unit UV rect,
/// a `mirrored_x` image paints the flipped one.
#[test]
fn mirrored_uv_reaches_the_painted_shape() {
    let unit = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let mirrored = Rect::from_min_max(pos2(1.0, 0.0), pos2(0.0, 1.0));
    let texture_id = TextureId::User(0);
    let rect_a = Rect::from_min_size(pos2(10.0, 10.0), vec2(8.0, 8.0));
    let rect_b = Rect::from_min_size(pos2(30.0, 10.0), vec2(8.0, 8.0));

    let ctx = Context::default();
    let output = run(&ctx, input(vec2(400.0, 300.0)), |ui| {
        let img = Image::new(SizedTexture::new(texture_id, vec2(8.0, 8.0)));
        img.paint_at(ui, rect_a);
        img.mirrored_x().paint_at(ui, rect_b);
    });

    let textures = textured_rects(&output, texture_id);
    assert_eq!(
        textures.len(),
        2,
        "expected two textured rects, one per paint, got {textures:?}"
    );
    let uv_of = |rect: Rect| {
        textures
            .iter()
            .find(|(shape_rect, _)| *shape_rect == rect)
            .map_or(Rect::NOTHING, |(_, uv)| *uv)
    };
    assert_eq!(uv_of(rect_a), unit, "a plain image paints the unit UV rect");
    assert_eq!(
        uv_of(rect_b),
        mirrored,
        "mirrored_x reaches the painted shape's UV"
    );
}
