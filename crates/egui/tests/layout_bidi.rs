//! Regression tests for bidi run segmentation + visual reordering through
//! the widget-text seam (`WidgetText::into_galley`):
//!
//! Mixed-direction text (e.g. `تقرير ٢٠٢٦.pdf` in a first-strong RTL label)
//! must render in *visual* order per UAX #9: the latin tail at the visual
//! left, the Arabic word at the visual right, numbers keeping their UAX #9
//! placement. Pure-LTR text must be unaffected.
//!
//! The glyphs of a reordered row are stored in visual (left-to-right) order.

use egui::{
    CentralPanel, Context, FontSelection, Pos2, RawInput, Rect, RichText, TextWrapMode, Vec2,
    WidgetText,
};

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Lay out one widget text in a central panel and return its visible glyphs
/// (advance > 0) as `(chr, x, advance)`, discarding the texture deltas
/// (this never renders).
fn layout_widget_text(ctx: &Context, rich_text: &RichText) -> Vec<(char, f32, f32)> {
    let mut result = None;
    let mut output = ctx.run_ui(input(Vec2::new(400.0, 300.0)), |ui| {
        CentralPanel::default().show(ui, |ui| {
            let galley = WidgetText::from(rich_text.clone()).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                400.0,
                FontSelection::Default,
            );
            assert_eq!(galley.rows.len(), 1, "expected a single-row galley");
            result = Some(
                galley.rows[0]
                    .row
                    .glyphs
                    .iter()
                    .filter(|g| g.advance_width > 0.0)
                    .map(|g| (g.chr, g.pos.x, g.advance_width))
                    .collect::<Vec<_>>(),
            );
        });
    });
    output.textures_delta.clear();
    result.expect("the ui closure always runs")
}

fn assert_x_nondecreasing(glyphs: &[(char, f32, f32)]) {
    for pair in glyphs.windows(2) {
        assert!(
            pair[0].1 <= pair[1].1 + 1e-3,
            "glyph x positions must not decrease left to right: {:?} then {:?}",
            pair[0],
            pair[1]
        );
    }
}

/// A first-strong RTL paragraph renders in visual order
/// — the glyphs read left-to-right as `pdf . ٢٠٢٦ [space] [Arabic word]`,
/// with the Arabic word's own glyphs running right-to-left.
#[test]
fn mixed_direction_widget_text_renders_visual_order() {
    let ctx = Context::default();
    let glyphs = layout_widget_text(&ctx, &RichText::new("تقرير ٢٠٢٦.pdf"));
    let rendered: Vec<char> = glyphs.iter().map(|g| g.0).collect();
    // Visual (left-to-right) glyph order; the Arabic word's glyphs run
    // right-to-left (its first logical letter ت is the visually-rightmost).
    let expected: Vec<char> = [
        'p', 'd', 'f', '.', '٢', '٠', '٢', '٦', ' ', 'ر', 'ي', 'ر', 'ق', 'ت',
    ]
    .into();
    assert_eq!(rendered, expected, "glyphs: {glyphs:?}");
    assert_x_nondecreasing(&glyphs);
}

/// LTR base with an embedded RTL run: the embedded run renders in visual
/// order (reversed), the surrounding LTR text stays in logical order.
#[test]
fn ltr_base_reverses_only_embedded_rtl_run() {
    let ctx = Context::default();
    let glyphs = layout_widget_text(&ctx, &RichText::new("Hello שלום world"));
    let rendered: Vec<char> = glyphs.iter().map(|g| g.0).collect();
    let expected: Vec<char> = [
        'H', 'e', 'l', 'l', 'o', ' ', 'ם', 'ו', 'ל', 'ש', ' ', 'w', 'o', 'r', 'l', 'd',
    ]
    .into();
    assert_eq!(rendered, expected, "glyphs: {glyphs:?}");
    assert_x_nondecreasing(&glyphs);
}

/// Pure-LTR widget text keeps its logical glyph order (byte-identity twin).
#[test]
fn pure_ltr_widget_text_keeps_logical_order() {
    let ctx = Context::default();
    let glyphs = layout_widget_text(&ctx, &RichText::new("Hello World 123"));
    let rendered: Vec<char> = glyphs.iter().map(|g| g.0).collect();
    let expected: Vec<char> = "Hello World 123".chars().collect();
    assert_eq!(rendered, expected, "glyphs: {glyphs:?}");
}
