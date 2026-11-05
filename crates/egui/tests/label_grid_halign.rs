//! Grid cells must honor an explicit [`Label::halign`] (the grid branch used
//! to clobber the job's halign with `Align::LEFT`), while the default stays
//! unchanged in both interface directions.
//!
//! Alignment is observable through the painted ink only when the galley box is
//! wider than its ink — a label whose text carries trailing whitespace keeps
//! that width in its box while epaint strips the spaces from the anchored ink.
//! Arabic-base text needs no such helper: epaint's
//! `normalize_reading_direction` re-anchors `Align::LEFT` to the reading start
//! for right-to-left rows, so an explicit halign is *not* honored there (pinned
//! below as documented behavior).

use egui::{Align, Context, FullOutput, Grid, Label, Pos2, RawInput, Rect, Ui, Vec2, vec2};

const SCREEN: Vec2 = vec2(400.0, 200.0);

/// Alignment assertions tolerate layout rounding, not glyph metrics.
const TOL: f32 = 2.0;

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input(SCREEN), run_ui);
    output.textures_delta.clear();
    output
}

/// The ink rect of the non-whitespace glyphs of the painted text whose galley
/// job text equals `needle`.
///
/// This deliberately ignores the galley's logical rect and any whitespace
/// glyphs: a label's trailing spaces keep their width in the galley box, and
/// the anchor strips them from the ink — that gap is exactly what makes the
/// halign shift visible.
fn painted_ink(output: &FullOutput, needle: &str) -> Rect {
    fn collect(shape: &egui::epaint::Shape, needle: &str, out: &mut Option<Rect>) {
        match shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text == needle => {
                for row in &text.galley.rows {
                    for glyph in &row.glyphs {
                        if glyph.chr.is_whitespace() {
                            continue;
                        }
                        let rect = glyph
                            .logical_rect()
                            .translate(text.pos.to_vec2() + row.pos.to_vec2());
                        *out = Some(match *out {
                            Some(ink) => ink.union(rect),
                            None => rect,
                        });
                    }
                }
            }
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, needle, out);
                }
            }
            _ => {}
        }
    }
    let mut out = None;
    for clipped in &output.shapes {
        collect(&clipped.shape, needle, &mut out);
    }
    out.unwrap_or_else(|| panic!("no painted text galley for {needle:?}"))
}

/// Lay out two grid rows in one column and return both labels' responses plus
/// the frame's output. The long label widens the column; the short one gets
/// the explicit halign under test.
fn grid_pair(
    rtl: bool,
    short_text: &'static str,
    halign: Option<Align>,
) -> (egui::Response, egui::Response, FullOutput) {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let mut long = None;
    let mut short_response = None;
    let output = run(&ctx, |ui| {
        Grid::new("label-grid-halign").show(ui, |ui| {
            long = Some(ui.add(Label::new("a fairly long label text")));
            ui.end_row();
            let mut label = Label::new(short_text);
            if let Some(halign) = halign {
                label = label.halign(halign);
            }
            short_response = Some(ui.add(label));
            ui.end_row();
        });
    });
    (
        long.unwrap_or_else(|| panic!("the long label never ran")),
        short_response.unwrap_or_else(|| panic!("the short label never ran")),
        output,
    )
}

/// An explicit `halign(RIGHT)` must survive the grid branch: the ink anchors
/// to the galley box's right edge instead of its left. The gap that makes this
/// visible is the label's trailing whitespace, which the box keeps but the
/// anchored ink drops. A digits-only label (weak text, left-to-right base) is
/// the twin: it takes no reading-direction rewrite, so the clobbered
/// `Align::LEFT` used to pin its ink to the box's left edge.
#[test]
fn grid_honors_an_explicit_halign() {
    for text in ["hi          ", "3.14          "] {
        let (_, short, output) = grid_pair(false, text, Some(Align::RIGHT));
        let ink = painted_ink(&output, text);
        assert!(
            (ink.right() - short.rect.right()).abs() <= TOL,
            "the explicitly right-aligned label must hug its galley box's right edge \
             (text {text:?}: ink {ink:?}, box {:?})",
            short.rect
        );
    }
}

/// The long cell (no explicit halign) is unchanged by the fix: its ink hugs
/// its box's left edge, like every default grid cell.
#[test]
fn grid_default_cell_hugs_the_left_edge() {
    let (long, _, output) = grid_pair(false, "a fairly long label text", None);
    let ink = painted_ink(&output, "a fairly long label text");
    assert!(
        (ink.left() - long.rect.left()).abs() <= TOL,
        "the default grid cell must keep hugging its box's left edge (ink {ink:?}, box {:?})",
        long.rect
    );
    assert!(
        (ink.right() - long.rect.right()).abs() <= TOL,
        "the long label fills its box, pinning the box's edges (ink {ink:?}, box {:?})",
        long.rect
    );
}

/// Characterization (passes before and after the fix): Arabic text has a
/// right-to-left base, and epaint's `normalize_reading_direction` re-anchors
/// `Align::LEFT` to the reading start — so inside grids an explicit halign is
/// not honored for it: right, left, and no halign all produce the same ink,
/// hugging the box's right (reading-start) edge.
#[test]
fn grid_rtl_base_text_ignores_an_explicit_halign() {
    let mut reference = Rect::NOTHING;
    for (i, halign) in [None, Some(Align::LEFT), Some(Align::RIGHT)]
        .into_iter()
        .enumerate()
    {
        let (_, response, output) = grid_pair(false, "سلام", halign);
        let ink = painted_ink(&output, "سلام");
        assert!(
            (ink.right() - response.rect.right()).abs() <= TOL,
            "Arabic grid text must be reading-start (right) anchored \
             (halign {halign:?}: ink {ink:?}, box {:?})",
            response.rect
        );
        if i == 0 {
            reference = ink;
        } else {
            assert_eq!(
                ink, reference,
                "the halign must not move the ink (halign {halign:?})"
            );
        }
    }
}

/// The default grid cell follows the interface direction: left-to-right
/// interfaces keep the left edge (the unchanged regression), right-to-left
/// interfaces stamp the label right-to-left, which epaint re-anchors to the
/// reading start — the box's right edge.
#[test]
fn grid_default_cell_follows_the_interface_direction() {
    for (rtl, hug_left) in [(false, true), (true, false)] {
        let (_, response, output) = grid_pair(rtl, "hi", None);
        let ink = painted_ink(&output, "hi");
        if hug_left {
            assert!(
                (ink.left() - response.rect.left()).abs() <= TOL,
                "an ltr interface's default cell must hug the left edge (ink {ink:?}, box {:?})",
                response.rect
            );
        } else {
            assert!(
                (ink.right() - response.rect.right()).abs() <= TOL,
                "an rtl interface's default cell must hug the reading-start (right) edge \
                 (ink {ink:?}, box {:?})",
                response.rect
            );
        }
    }
}
