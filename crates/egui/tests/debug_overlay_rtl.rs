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
use egui::{Color32, Context, Event, FullOutput, Pos2, RawInput, Rect, Ui, Vec2, pos2, vec2};

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

// ---------------------------------------------------------------------------

/// Raw input with the given pointer events, for driving `debug_text::print`'s anchor.
fn input_with_events(size: Vec2, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        events,
        ..Default::default()
    }
}

/// The location text `debug_text::print` stamps next to the entry
/// (`format!("{}:{}", Location::file(), line)` of the test call site).
fn painted_location(texts: &[(String, Rect)]) -> &Rect {
    texts
        .iter()
        .find(|(text, _)| text.contains(".rs:"))
        .map(|(_, rect)| rect)
        .expect("the debug-text location should paint")
}

/// `debug_text::print` with the pointer near the right edge (common for right-hugging
/// widgets under right-to-left): the entry text no longer fits the room right of the
/// pointer, so the sides flip — the entry hangs leftward from the pointer on a single
/// line, and the location (metadata) wraps in the narrow strip that remains right of
/// the pointer.
///
/// The assertion is on the *layout*, not the screen bounds: wrapping already keeps the
/// squashed entry inside the screen, so that would not fail at base.
///
/// Gated on `debug_assertions`: `print` compiles to a no-op without it, so under
/// `cargo test --release` there are no shapes to assert on (CI runs tests in debug).
#[cfg(debug_assertions)]
#[test]
fn debug_text_entries_flip_to_the_roomier_side_only_on_overflow() {
    // Overflow case: the pointer at x = 380 of a 400 px screen leaves a 20 px column
    // to the right, which squashes the ~70 px-wide entry into a multi-line tower.
    let ctx = Context::default();
    let output = run(
        &ctx,
        input_with_events(SCREEN, vec![Event::PointerMoved(pos2(380.0, 150.0))]),
        |ui| {
            egui::debug_text::print(ui.ctx(), "DBGTEXTENTRY");
        },
    );

    let texts = painted_texts(&output);
    let entry = painted_rect(&texts, "DBGTEXTENTRY").expect("the entry text should paint");
    assert!(
        entry.height() < 20.0,
        "the entry must be a single line (10 px font), got {entry:?}"
    );
    assert!(
        (entry.right() - 376.0).abs() < INK_TOLERANCE,
        "the entry should hang leftward from the pointer (right edge at 380 - 4), \
         got {entry:?}"
    );

    let location = painted_location(&texts);
    assert!(
        (location.left() - 384.0).abs() < INK_TOLERANCE,
        "the location should grow rightward from pointer + 4 px in the flipped branch, \
         got {location:?}"
    );

    // Companion A (today-geometry pin): a left-half pointer with room keeps today's
    // layout — location right-aligned ending at pointer - 4 px, entry starting at the
    // pointer, both single-line.
    let ctx = Context::default();
    let output = run(
        &ctx,
        input_with_events(SCREEN, vec![Event::PointerMoved(pos2(100.0, 150.0))]),
        |ui| {
            egui::debug_text::print(ui.ctx(), "DBGTEXTENTRY");
        },
    );

    let texts = painted_texts(&output);
    let entry = painted_rect(&texts, "DBGTEXTENTRY").expect("the entry text should paint");
    let location = painted_location(&texts);
    assert!(
        (location.right() - 96.0).abs() < INK_TOLERANCE,
        "the location stays right-aligned at pointer - 4 px, got {location:?}"
    );
    assert!(
        (entry.left() - 100.0).abs() < INK_TOLERANCE,
        "the entry stays left-anchored at the pointer, got {entry:?}"
    );
    assert!(
        entry.height() < 20.0 && location.height() < 20.0,
        "both stay single-line with room, got entry {entry:?}, location {location:?}"
    );

    // Companion B (overflow-only-gate pin): a right-half pointer WITH room does not
    // flip. This fails if the gate is ever loosened back to a right-half heuristic.
    let ctx = Context::default();
    let output = run(
        &ctx,
        input_with_events(SCREEN, vec![Event::PointerMoved(pos2(250.0, 150.0))]),
        |ui| {
            egui::debug_text::print(ui.ctx(), "DBGTEXTENTRY");
        },
    );

    let texts = painted_texts(&output);
    let entry = painted_rect(&texts, "DBGTEXTENTRY").expect("the entry text should paint");
    let location = painted_location(&texts);
    assert!(
        (location.right() - 246.0).abs() < INK_TOLERANCE,
        "a right-half pointer with room keeps the location left of the pointer, \
         got {location:?}"
    );
    assert!(
        (entry.left() - 250.0).abs() < INK_TOLERANCE,
        "a right-half pointer with room keeps the entry right of the pointer, \
         got {entry:?}"
    );

    // Companion C (strictly-better pin): a very long entry (natural width beyond the
    // whole content rect) at a left-half pointer does NOT flip — the right side is the
    // roomier side, so today's 300 px right-side wrap (readable, not clipped) stays.
    // This fails if the `room_left > room_right` half of the gate is ever dropped.
    let ctx = Context::default();
    let output = run(
        &ctx,
        input_with_events(SCREEN, vec![Event::PointerMoved(pos2(100.0, 150.0))]),
        |ui| {
            egui::debug_text::print(ui.ctx(), "LONGENTRY".repeat(20));
        },
    );

    let texts = painted_texts(&output);
    let long_entry = texts
        .iter()
        .find(|(text, _)| text.starts_with("LONGENTRY"))
        .map(|(_, rect)| rect)
        .expect("the long entry text should paint");
    assert!(
        (long_entry.left() - 100.0).abs() < INK_TOLERANCE,
        "a long entry at a left-half pointer wraps on the roomier right side \
         (left edge at the pointer), got {long_entry:?}"
    );
    assert!(
        long_entry.max.x <= 402.0,
        "a long entry at a left-half pointer wraps inside the content rect, \
         got {long_entry:?}"
    );
    let location = painted_location(&texts);
    assert!(
        (location.right() - 96.0).abs() < INK_TOLERANCE,
        "the location stays left of the pointer, got {location:?}"
    );
}
