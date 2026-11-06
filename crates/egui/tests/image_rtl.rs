//! Regression tests for the image widget's reading direction: the fallback galley
//! painted when a texture fails to load carries the interface reading direction (the
//! same stamp `WidgetText::into_galley_impl` applies), so mixed-direction alt text
//! resolves to a right-to-left base under a right-to-left interface instead of
//! first-strong's left-to-right fallback. The URI hover texts isolate the uri with
//! bidi isolates (unit-tested in `widgets/image.rs`).
//!
//! The glyphs of a reordered row are stored in visual (left-to-right) order; see
//! `layout_bidi.rs` for the underlying run-segmentation oracles. Every mirrored case
//! asserts its left-to-right counterpart.

use egui::epaint::Shape;
use egui::{Context, FullOutput, Image, Pos2, RawInput, Rect, Ui, Vec2, text::TextDirection, vec2};

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

/// A uri no default loader accepts, so the image paints its error fallback.
const BROKEN_URI: &str = "https://example.invalid/missing.png";

/// The alt text used by the tests: a Latin run followed by an Arabic run, so the base
/// direction decides which run anchors at the visual left.
const MIXED_ALT_TEXT: &str = "Docs: الوثائق";

/// `(direction stamp, x of the first visual 'D' glyph, x of the first visual Arabic
/// glyph)` of the error galley painted for a failed image load.
///
/// The error galley is the only painted text of the pass.
fn error_galley_glyphs(rtl: bool) -> (Option<TextDirection>, f32, f32) {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    let screen = vec2(400.0, 300.0);
    let mut found = None;
    let output = run(&ctx, input(screen), |ui| {
        ui.add(
            Image::new(BROKEN_URI)
                .fit_to_exact_size(vec2(240.0, 60.0))
                .maintain_aspect_ratio(false)
                .alt_text(MIXED_ALT_TEXT),
        );
    });
    for clipped in &output.shapes {
        if let Shape::Text(text) = &clipped.shape
            && text.galley.job.text.contains("Docs")
        {
            assert!(
                found.is_none(),
                "expected exactly one error galley, got a second one"
            );
            let row = &text.galley.rows[0];
            let d_x = row
                .glyphs
                .iter()
                .find(|g| g.chr == 'D')
                .map(|g| g.pos.x)
                .expect("the Latin run should paint");
            let arabic_x = row
                .glyphs
                .iter()
                .find(|g| g.chr == 'ا')
                .map(|g| g.pos.x)
                .expect("the Arabic run should paint");
            found = Some((text.galley.job.direction, d_x, arabic_x));
        }
    }
    found.expect("the error fallback galley should paint")
}

/// Under the interface flag the error galley gets an explicit right-to-left base, so
/// the Arabic run anchors at the visual right and the Latin run sits to its left.
/// At baseline the job carried no direction, first-strong picked left-to-right, and
/// the Latin run anchored left of the Arabic run instead.
#[test]
fn image_error_alt_text_follows_the_interface_direction() {
    let (direction, latin_x, arabic_x) = error_galley_glyphs(true);
    assert_eq!(
        direction,
        Some(TextDirection::RightToLeft),
        "RTL: the error galley should carry an explicit right-to-left base"
    );
    assert!(
        latin_x > arabic_x,
        "RTL: the Latin run should sit right of the Arabic run (D at {latin_x}, \
         Arabic at {arabic_x})"
    );

    // LTR twin: the stamp is inert, so first-strong's fallback resolution stays
    // left-to-right and the Latin run stays left of the Arabic run (the galley's
    // job carries the resolved direction after layout):
    let (direction, latin_x, arabic_x) = error_galley_glyphs(false);
    assert_eq!(
        direction,
        Some(TextDirection::LeftToRight),
        "LTR: the error galley should resolve left-to-right"
    );
    assert!(
        latin_x < arabic_x,
        "LTR: the Latin run should sit left of the Arabic run (D at {latin_x}, \
         Arabic at {arabic_x})"
    );
}
