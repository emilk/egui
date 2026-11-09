//! Regression tests for the bidi isolation of formatted shortcut text:
//! [`egui::KeyboardShortcut::format`] (and with it [`egui::Context::format_shortcut`])
//! wraps its output in U+2066 LEFT-TO-RIGHT ISOLATE … U+2069 POP DIRECTIONAL
//! ISOLATE, so a shortcut embedded in right-to-left text renders as one
//! left-to-right run instead of letting its neutral glyphs (the `+`
//! separators, the symbol forms of keys) resolve to the surrounding paragraph
//! direction.
//!
//! The RTL case below is the discriminating one (UAX #9 rules N1/N2): every
//! glyph of the symbol form `⌃⇧−` is neutral, so in an RTL paragraph the whole
//! run (plus the space before it) resolves to the paragraph direction and lands
//! visually left of the label *reversed*; inside the isolate it stays a
//! left-to-right run. This is the real-world shape of the demo's zoom
//! shortcuts (`Ctrl`+`Minus`/`Plus`/`Equals`) under a right-to-left interface.
//! The NAMES form (`Ctrl+Shift+Minus`) is all strong-L after its separators and
//! never mis-orders; it is pinned so the isolation cannot break it. The LTR
//! twin is the byte-identity pin: in a fontful environment the two isolates are
//! zero-advance invisible glyphs that move nothing; the fontless default test
//! environment draws every char as a fixed-width box, so there the visible
//! glyphs are compared relative to the first one.
//!
//! Glyph order is asserted through the public
//! [`egui::epaint::text::Row::glyphs`], which is documented as the *visual*
//! order per UAX #9 (`pos.x` increases along the slice).

use egui::epaint::text::TextDirection;
use egui::text::LayoutJob;
use egui::{
    Color32, Context, Key, KeyboardShortcut, ModifierNames, Modifiers, Pos2, RawInput, Rect,
    TextStyle, Ui, Vec2, vec2,
};

const SCREEN: Vec2 = vec2(400.0, 300.0);

/// The glyph [`Key::Minus`] formats as in the symbol form
/// (`egui`'s minus char, U+2212).
const MINUS: char = '−';

/// The two directional isolates the formatter wraps shortcut text in.
const ISOLATES: [char; 2] = ['\u{2066}', '\u{2069}'];

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

/// Lay out one job inside a frame; returns the row's `(char, x)` pairs in
/// visual order plus the galley's ink bounds.
fn layout_single_row(
    ctx: &Context,
    text: &str,
    direction: Option<TextDirection>,
) -> (Vec<(char, f32)>, Rect) {
    let mut out = None;
    run(ctx, input(SCREEN), |ui| {
        let mut job = LayoutJob::simple(
            text.to_owned(),
            TextStyle::Body.resolve(ui.style()),
            Color32::PLACEHOLDER,
            400.0,
        );
        job.direction = direction;
        let galley = ui.painter().layout_job(job);
        assert_eq!(
            galley.rows.len(),
            1,
            "the test text must fit on a single row: {:?}",
            galley.rows.len()
        );
        let glyphs = galley.rows[0]
            .glyphs
            .iter()
            .map(|g| (g.chr, g.pos.x))
            .collect::<Vec<_>>();
        out = Some((glyphs, galley.mesh_bounds));
    });
    out.unwrap_or_else(|| panic!("layout should have run inside the frame"))
}

/// The row's glyphs without the two isolate control chars.
fn visible_glyphs(glyphs: &[(char, f32)]) -> Vec<(char, f32)> {
    glyphs
        .iter()
        .filter(|(c, _)| !ISOLATES.contains(c))
        .copied()
        .collect()
}

fn x_of(glyphs: &[(char, f32)], chr: char) -> f32 {
    glyphs
        .iter()
        .find(|(c, _)| *c == chr)
        .map(|(_, x)| *x)
        .unwrap_or_else(|| panic!("glyph {chr:?} missing from the row: {glyphs:?}"))
}

/// The symbol form `⌃⇧−` is all neutral glyphs: in an RTL paragraph after an
/// Arabic label the run (with its leading space) resolves to the paragraph
/// direction and renders reversed, left of the label; inside the LRI…PDI
/// isolate it resolves left-to-right and keeps `−` right of `⌃`.
#[test]
fn rtl_paragraph_keeps_a_symbol_shortcut_left_to_right() {
    let ctx = Context::default();
    let shortcut = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Minus);
    let text = format!("عنوان {}", shortcut.format(&ModifierNames::SYMBOLS, false));

    let (glyphs, _) = layout_single_row(&ctx, &text, Some(TextDirection::RightToLeft));

    assert!(
        x_of(&glyphs, MINUS) > x_of(&glyphs, '⌃'),
        "inside the isolate the symbol shortcut must read left-to-right \
         ('−' right of '⌃'); glyphs: {glyphs:?}"
    );
}

/// The NAMES form (`Ctrl+Shift+Minus`) is strong-L around neutral separators
/// and never mis-orders in an RTL paragraph; pin that the isolation keeps it
/// one contiguous left-to-right block (not reversed, not split).
#[test]
fn rtl_paragraph_keeps_a_names_shortcut_one_ltr_block() {
    let ctx = Context::default();
    let shortcut = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Minus);
    let text = format!("عنوان {}", ctx.format_shortcut(&shortcut));

    let (glyphs, _) = layout_single_row(&ctx, &text, Some(TextDirection::RightToLeft));

    // The Arabic label contributes no Latin glyphs, so the shortcut's chars
    // lead the visible stream, in logical (left-to-right) order.
    let visible = visible_glyphs(&glyphs);
    let expected = "Ctrl+Shift+Minus";
    assert!(
        visible.len() >= expected.chars().count(),
        "the shortcut should be painted in full, got {visible:?}"
    );
    let mut prev_x: Option<f32> = None;
    for (i, (chr, x)) in visible.iter().enumerate().take(expected.chars().count()) {
        let want = expected.chars().nth(i).unwrap_or_else(|| panic!("ascii"));
        assert_eq!(
            *chr, want,
            "the shortcut block must render in logical char order; glyphs: {visible:?}"
        );
        if let Some(prev) = prev_x {
            assert!(
                *x > prev,
                "the block must read left-to-right; glyphs: {visible:?}"
            );
        }
        prev_x = Some(*x);
    }
}

/// The same wrapping laid out first-strong (LTR): the isolates are invisible
/// formatting chars, so the visible glyphs keep their order and relative
/// positions. In a fontful environment they are zero-advance and the absolute
/// positions and ink bounds are identical too (the kittest snapshot hashes are
/// the visual proof); the fontless default test environment draws every char
/// as a fixed-width box, so there the positions are compared relative to the
/// first visible glyph.
#[test]
fn ltr_wrapping_leaves_visible_glyphs_unchanged() {
    let ctx = Context::default();
    let shortcut = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Minus);
    let wrapped = ctx.format_shortcut(&shortcut);
    assert!(
        wrapped.starts_with(ISOLATES[0]) && wrapped.ends_with(ISOLATES[1]),
        "the formatted shortcut should be wrapped in LRI…PDI, got {wrapped:?}"
    );
    let raw = wrapped
        .trim_start_matches(ISOLATES[0])
        .trim_end_matches(ISOLATES[1]);
    assert_eq!(raw, "Ctrl+Shift+Minus");

    let (raw_glyphs, _) = layout_single_row(&ctx, raw, None);
    let (wrapped_glyphs, _) = layout_single_row(&ctx, &wrapped, None);

    // Compare per visible char, not per glyph: the isolates add glyph slots.
    let raw_visible = visible_glyphs(&raw_glyphs);
    let wrapped_visible = visible_glyphs(&wrapped_glyphs);
    assert_eq!(
        raw_visible.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        wrapped_visible.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        "the isolates must not reorder the visible glyphs in an LTR paragraph"
    );

    let anchored = |glyphs: Vec<(char, f32)>| {
        let first = glyphs.first().map_or(0.0, |(_, x)| *x);
        glyphs.into_iter().map(move |(c, x)| (c, x - first))
    };
    assert_eq!(
        anchored(raw_visible.clone()).collect::<Vec<_>>(),
        anchored(wrapped_visible).collect::<Vec<_>>(),
        "the isolates must not move the visible glyphs relative to each other \
         in an LTR paragraph"
    );
}
