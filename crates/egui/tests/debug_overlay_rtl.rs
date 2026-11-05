//! Regression tests for the debug overlays under right-to-left interfaces.
//!
//! Debug-overlay shapes reach the captured `FullOutput.shapes` (they are painted
//! before the frame's graphics drain), so these tests assert on shape geometry,
//! which is *unclipped*: clipping happens only at tessellation time. That makes the
//! pre-fix off-screen garbage visible to these assertions.
//!
//! Every mirrored case also asserts its LTR counterpart, so this file doubles as the
//! LTR-unchanged regression. No snapshot assertions; painted text geometry only.

use egui::epaint::Shape;
use egui::{Color32, Context, FullOutput, Pos2, RawInput, Rect, Ui, Vec2, pos2, vec2};

const SCREEN: Vec2 = vec2(400.0, 300.0);

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

/// (text, absolute ink rect) for every text shape painted this pass.
///
/// `Galley::mesh_bounds` excludes leading/trailing whitespace, so the rect is the true
/// ink bounds of the glyphs, translated to their absolute position.
fn painted_texts(output: &FullOutput) -> Vec<(String, Rect)> {
    fn collect(shape: &Shape, out: &mut Vec<(String, Rect)>) {
        match shape {
            Shape::Text(text) => {
                out.push((
                    text.galley.job.text.clone(),
                    text.galley.mesh_bounds.translate(text.pos.to_vec2()),
                ));
            }
            Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut out);
    }
    out
}

/// The absolute ink rect of the first painted text equal to `text`, or `None`.
fn painted_rect<'a>(texts: &'a [(String, Rect)], text: &str) -> Option<&'a Rect> {
    texts
        .iter()
        .find(|(painted, _)| painted == text)
        .map(|(_, rect)| rect)
}

/// Tolerance for ink-edge assertions: `mesh_bounds` excludes glyph bearings, so the
/// ink of an anchored galley can sit a couple of px inside its anchor point.
const INK_TOLERANCE: f32 = 2.0;

// ---------------------------------------------------------------------------

/// `Painter::debug_rect` on a right-hugging widget (as mirrored panels and windows
/// produce under right-to-left): a label painted rightward from the widget's left
/// edge would run past the screen and be clipped off, so it must hang inward from
/// the widget's right edge instead. A label that fits the clip rect keeps today's
/// `LEFT_TOP` anchor at `rect.min` (left-to-right output unchanged).
#[test]
fn debug_rect_label_hangs_inward_only_when_it_overflows_the_clip_rect() {
    // Overflow case: 8 monospace-12 characters span ~60 px, so a label painted
    // rightward from x = 360 on a 400 px screen runs past the edge.
    let ctx = Context::default();
    let output = run(&ctx, input(SCREEN), |ui| {
        ui.ctx().debug_painter().debug_rect(
            Rect::from_min_size(pos2(360.0, 20.0), vec2(30.0, 20.0)),
            Color32::RED,
            "RCTLABEL",
        );
    });

    let texts = painted_texts(&output);
    let ink =
        painted_rect(&texts, "RCTLABEL").expect("the overflowing debug_rect label should paint");
    assert!(
        ink.max.x <= 400.0,
        "the overflowing label must stay inside the 400 px content rect, got {ink:?}"
    );
    assert!(
        (ink.max.x - 390.0).abs() < INK_TOLERANCE,
        "the label should hang inward from the widget's right edge (390), got {ink:?}"
    );

    // LTR fit companion: a label that fits keeps the LEFT_TOP anchor at rect.min,
    // i.e. today's geometry exactly.
    let ctx = Context::default();
    let output = run(&ctx, input(SCREEN), |ui| {
        ui.ctx().debug_painter().debug_rect(
            Rect::from_min_size(pos2(10.0, 20.0), vec2(30.0, 20.0)),
            Color32::RED,
            "RCTLABEL",
        );
    });

    let texts = painted_texts(&output);
    let ink = painted_rect(&texts, "RCTLABEL").expect("the fitting debug_rect label should paint");
    assert!(
        (ink.left() - 10.0).abs() < INK_TOLERANCE,
        "a fitting label keeps the LEFT_TOP anchor at rect.min (LTR unchanged), got {ink:?}"
    );
    assert!(
        (ink.top() - 20.0).abs() < INK_TOLERANCE,
        "a fitting label keeps its top at rect.min.y, got {ink:?}"
    );
}
