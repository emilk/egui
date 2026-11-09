//! Regression tests for `Painter::layout`/`layout_no_wrap` text: the default jobs the
//! painter builds are stamped with the context language and the ambient reading
//! direction (the same stamp `WidgetText::into_galley_impl` applies), so third-party
//! overlay text resolves to a right-to-left base under a right-to-left interface
//! instead of first-strong's left-to-right fallback. `Painter::layout_job` stays
//! caller-authoritative.
//!
//! The glyphs of a reordered row are stored in visual (left-to-right) order; see
//! `layout_bidi.rs` for the underlying run-segmentation oracles. Every mirrored case
//! asserts its left-to-right counterpart.

use egui::epaint::Shape;
use egui::{
    Align2, Color32, Context, FontId, FullOutput, Pos2, RawInput, Rect, Ui, Vec2, pos2,
    text::TextDirection, vec2,
};
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

/// `(resolved job direction, (chr, x) of the painted overlay text's glyphs)` for text
/// painted with `Painter::text` under the given interface direction.
fn overlay_glyphs(rtl: bool, text: &str) -> (Option<TextDirection>, Vec<(char, f32)>) {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    let screen = vec2(400.0, 300.0);
    let mut found = None;
    let output = run(&ctx, input(screen), |ui| {
        ui.painter().text(
            pos2(50.0, 50.0),
            Align2::LEFT_TOP,
            text,
            FontId::proportional(14.0),
            Color32::WHITE,
        );
    });
    for clipped in &output.shapes {
        if let Shape::Text(text_shape) = &clipped.shape {
            assert!(
                found.is_none(),
                "expected exactly one painted text, got a second one"
            );
            found = Some((
                text_shape.galley.job.direction,
                text_shape.galley.rows[0]
                    .glyphs
                    .iter()
                    .filter(|g| g.advance_width > 0.0)
                    .map(|g| (g.chr, g.pos.x))
                    .collect::<Vec<_>>(),
            ));
        }
    }
    found.expect("the overlay text should paint")
}

/// The x of the first visual glyph `chr`, or `None`.
fn glyph_x(glyphs: &[(char, f32)], chr: char) -> Option<f32> {
    glyphs.iter().find(|(c, _)| *c == chr).map(|(_, x)| *x)
}

/// Under the interface flag, mixed-direction overlay text laid out through
/// `Painter::text` (and so `layout_no_wrap`) gets an explicit right-to-left base: the
/// logical-start Latin run anchors at the visual right. At baseline the job carried no
/// direction, first-strong picked left-to-right, and the Latin run anchored left.
#[test]
fn painter_text_follows_the_interface_direction() {
    let (direction, glyphs) = overlay_glyphs(true, "Docs: الوثائق");
    assert_eq!(
        direction,
        Some(TextDirection::RightToLeft),
        "RTL: the overlay galley should carry an explicit right-to-left base"
    );
    let latin_x = glyph_x(&glyphs, 'D').expect("the Latin run should paint");
    let arabic_x = glyph_x(&glyphs, 'ا').expect("the Arabic run should paint");
    assert!(
        latin_x > arabic_x,
        "RTL: the Latin run should sit right of the Arabic run (D at {latin_x}, \
         Arabic at {arabic_x})"
    );

    // LTR twin: the stamp is inert, first-strong's fallback stays left-to-right and
    // the Latin run stays left of the Arabic run:
    let (direction, glyphs) = overlay_glyphs(false, "Docs: الوثائق");
    assert_eq!(
        direction,
        Some(TextDirection::LeftToRight),
        "LTR: the overlay galley should resolve left-to-right"
    );
    let latin_x = glyph_x(&glyphs, 'D').expect("the Latin run should paint");
    let arabic_x = glyph_x(&glyphs, 'ا').expect("the Arabic run should paint");
    assert!(
        latin_x < arabic_x,
        "LTR: the Latin run should sit left of the Arabic run (D at {latin_x}, \
         Arabic at {arabic_x})"
    );
}

/// Direction-agnostic overlay text (a single unidirectional run: no first-strong RTL
/// candidate) is unaffected by the stamp in BOTH directions: under the flag the job
/// carries an explicit right-to-left base, but one left-to-right run renders
/// identically under either base, so the glyph layout is pinned identical.
#[test]
fn painter_text_direction_agnostic_runs_stay_unchanged() {
    let (rtl_direction, rtl_glyphs) = overlay_glyphs(true, "42px");
    assert_eq!(
        rtl_direction,
        Some(TextDirection::RightToLeft),
        "RTL: the stamped job carries an explicit right-to-left base"
    );
    let (ltr_direction, ltr_glyphs) = overlay_glyphs(false, "42px");
    assert_eq!(
        ltr_direction,
        Some(TextDirection::LeftToRight),
        "LTR: first-strong resolves left-to-right"
    );

    // The single left-to-right run renders identically under either base:
    assert_eq!(
        rtl_glyphs, ltr_glyphs,
        "a single unidirectional run must render identically under either base"
    );
}
