//! Regression tests for the id-clash debug overlay through the real mirrored
//! layout path, under left-to-right and right-to-left interfaces.
//!
//! The landed flip (anchor the label inward from the widget's right edge when
//! painting it rightward would run past the content rect) is pinned here through
//! the chain the original issue documented: a real `CollapsingHeader` inside the
//! root `Ui` of a mirrored interface, whose content-sized rect hugs the window's
//! right edge under right-to-left. The landed `debug_overlay_rtl.rs` pins the
//! same overlay only with synthetic rects in LTR.
//!
//! Debug-overlay shapes reach the captured `FullOutput.shapes` (they are painted
//! before the frame's graphics drain), so these tests assert on shape geometry,
//! which is *unclipped*: clipping happens only at tessellation time. That makes
//! off-window labels observable, which the narrower-screen tests rely on.
//!
//! Assertions are made on the *layout box* of the painted text only
//! (`galley.rect` translated to absolute position), never on glyph ink: the
//! layout box is the exact geometry the overlay anchors compute and is
//! independent of the font provider and of the galley's horizontal alignment
//! (`Painter::debug_text` re-positions RTL-stamped galleys so their frame lands
//! on the anchor either way). The label text starts with the bidi-neutral 🔥
//! emoji, which visually reorders to the label's right end under a
//! right-to-left interface, so its ink must never be asserted; tests match
//! galleys by their ASCII substring instead.
//!
//! No snapshot assertions; painted text geometry only.

use egui::epaint::Shape;
use egui::{Context, FullOutput, Pos2, RawInput, Rect, Ui, Vec2, vec2};

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

/// One painted text shape: its job text and absolute layout box.
///
/// The layout box (`galley.rect` translated to absolute position) is the exact
/// geometry the debug-overlay anchors position, so assertions pin it and stay
/// font-provider independent. Glyph ink is deliberately not captured: the 🔥
/// emoji's ink depends on font coverage and reorders under RTL (see the module
/// docs).
struct PaintedText {
    text: String,
    layout_box: Rect,
}

/// Every text shape painted this pass, with its job text and layout box.
fn painted_texts(output: &FullOutput) -> Vec<PaintedText> {
    fn collect(shape: &Shape, out: &mut Vec<PaintedText>) {
        match shape {
            Shape::Text(text) => {
                out.push(PaintedText {
                    text: text.galley.job.text.clone(),
                    layout_box: text.galley.rect.translate(text.pos.to_vec2()),
                });
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

/// The absolute layout box of the first painted text containing `needle`, or
/// `None`.
fn painted_layout<'a>(texts: &'a [PaintedText], needle: &str) -> Option<&'a Rect> {
    texts
        .iter()
        .find(|painted| painted.text.contains(needle))
        .map(|painted| &painted.layout_box)
}

/// Tolerance for layout-box (anchor) assertions. The layout box is the exact
/// geometry the debug anchors compute from widget/clip rects, independent of
/// glyph bearings and therefore of the font provider; the slack only absorbs
/// float noise. (Mirrors the landed constant in `debug_overlay_rtl.rs`.)
const ANCHOR_TOLERANCE: f32 = 0.5;

// ---------------------------------------------------------------------------

/// The id-clash overlay through the real mirrored layout under right-to-left:
/// two `CollapsingHeader`s sharing an id inside the root `Ui` produce
/// content-sized rects hugging the window's right edge (the original issue's
/// scenario), so the labels must hang inward from the content rect's right
/// edge and stay fully inside the window.
///
/// The 400 px screen leaves room for the ~230 px label, so the horizontal
/// clamp (when it lands) is a no-op here — this pin is independent of it.
#[test]
fn id_clash_error_hangs_inward_from_the_right_edge_under_rtl() {
    let ctx = Context::default();
    // The gate already defaults to true under `debug_assertions`; set it
    // explicitly so the test is independent of the build profile.
    ctx.options_mut(|o| o.warn_on_id_clash = true);
    ctx.set_right_to_left(true);

    let output = run(&ctx, input(vec2(400.0, 300.0)), |ui| {
        ui.collapsing("Clashing header", |ui| {
            ui.label("x");
        });
        ui.collapsing("Clashing header", |ui| {
            ui.label("x");
        });
    });

    let texts = painted_texts(&output);
    for (what, needle) in [("first", "First use of"), ("second", "Second use of")] {
        let layout = painted_layout(&texts, needle)
            .unwrap_or_else(|| panic!("the {what} id-clash error should paint"));
        assert!(
            (layout.right() - 400.0).abs() < ANCHOR_TOLERANCE,
            "RTL: the {what} error should hang inward from the content rect's \
             right edge, got {layout:?}"
        );
        assert!(
            layout.left() >= 0.0 - ANCHOR_TOLERANCE,
            "RTL: the {what} error must stay inside the window's left edge, \
             got {layout:?}"
        );
    }
}

// ---------------------------------------------------------------------------

/// LTR twin of the right-edge pin: the same collapsing pair in a
/// left-to-right interface sits at the window's left edge, does not overflow,
/// and keeps the physical-left anchor — the LTR-unchanged regression for the
/// real path.
#[test]
fn id_clash_error_ltr_twin_keeps_the_left_edge_anchor() {
    let ctx = Context::default();
    ctx.options_mut(|o| o.warn_on_id_clash = true);

    let output = run(&ctx, input(vec2(400.0, 300.0)), |ui| {
        ui.collapsing("Clashing header", |ui| {
            ui.label("x");
        });
        ui.collapsing("Clashing header", |ui| {
            ui.label("x");
        });
    });

    let texts = painted_texts(&output);
    for (what, needle) in [("first", "First use of"), ("second", "Second use of")] {
        let layout = painted_layout(&texts, needle)
            .unwrap_or_else(|| panic!("the {what} id-clash error should paint"));
        assert!(
            (layout.left() - 0.0).abs() < ANCHOR_TOLERANCE,
            "LTR: the {what} error keeps the LEFT anchor at the content rect's \
             left edge, got {layout:?}"
        );
        assert!(
            layout.right() <= 400.0 + ANCHOR_TOLERANCE,
            "LTR: the {what} error must stay inside the window, got {layout:?}"
        );
    }
}

// ---------------------------------------------------------------------------

/// The clamp companion: on a wide screen a fitting label is never moved, in
/// either direction. LTR keeps the physical-left anchor; RTL keeps the
/// flipped anchor at the content rect's right edge. This pins that the
/// horizontal clamp (which lands with the narrow-screen tests below) is a
/// no-op whenever the flipped label already fits.
#[test]
fn id_clash_error_never_moves_a_label_that_fits() {
    // LTR: no overflow (the collapser sits at the left edge), LEFT branch.
    let ctx = Context::default();
    ctx.options_mut(|o| o.warn_on_id_clash = true);

    let output = run(&ctx, input(vec2(400.0, 300.0)), |ui| {
        ui.collapsing("Clashing header", |ui| {
            ui.label("x");
        });
        ui.collapsing("Clashing header", |ui| {
            ui.label("x");
        });
    });

    let texts = painted_texts(&output);
    for (what, needle) in [("first", "First use of"), ("second", "Second use of")] {
        let layout = painted_layout(&texts, needle)
            .unwrap_or_else(|| panic!("the LTR {what} id-clash error should paint"));
        assert!(
            (layout.left() - 0.0).abs() < ANCHOR_TOLERANCE,
            "LTR: a fitting {what} error keeps the LEFT anchor at the content \
             rect's left edge, got {layout:?}"
        );
        assert!(
            layout.right() <= 400.0 + ANCHOR_TOLERANCE,
            "LTR: a fitting {what} error must stay inside the window, \
             got {layout:?}"
        );
    }

    // RTL: overflow flips the anchor to the content rect's right edge, where
    // the label fits (w < 400), so the clamp must leave it there.
    let ctx = Context::default();
    ctx.options_mut(|o| o.warn_on_id_clash = true);
    ctx.set_right_to_left(true);

    let output = run(&ctx, input(vec2(400.0, 300.0)), |ui| {
        ui.collapsing("Clashing header", |ui| {
            ui.label("x");
        });
        ui.collapsing("Clashing header", |ui| {
            ui.label("x");
        });
    });

    let texts = painted_texts(&output);
    for (what, needle) in [("first", "First use of"), ("second", "Second use of")] {
        let layout = painted_layout(&texts, needle)
            .unwrap_or_else(|| panic!("the RTL {what} id-clash error should paint"));
        assert!(
            (layout.right() - 400.0).abs() < ANCHOR_TOLERANCE,
            "RTL: a fitting {what} error keeps the flipped anchor at the \
             content rect's right edge, got {layout:?}"
        );
        assert!(
            layout.left() >= 0.0 - ANCHOR_TOLERANCE,
            "RTL: a fitting {what} error must stay inside the window's left \
             edge, got {layout:?}"
        );
    }
}
