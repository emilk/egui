//! Regression tests for the [`egui::TextEdit`] reading direction: the input text
//! and the hint (placeholder) text anchor on the field's reading-start edge with
//! the interface's base reading direction — right under a right-to-left
//! interface, left under the default — while an explicit [`egui::TextEdit::align`]
//! still wins over the flipped default.
//!
//! No snapshot assertions; painted text layout-rect geometry and the galley job's
//! resolved direction only. Every mirrored case carries its LTR twin, so this
//! file doubles as the LTR-unchanged regression.
//!
//! Note on explicit alignment: the widget's `align` places the text block
//! physically (left/center/right within the field) in both interface
//! directions — only the *default* is reading-direction-aware (the style's
//! flipped `align2` plus the galley job's pinned base direction).

use egui::epaint::Shape;
use egui::epaint::text::TextDirection;
use egui::{
    Align2, Context, FullOutput, Id, Pos2, RawInput, Rect, TextEdit, Ui, Vec2,
    text::{CCursor, CCursorRange},
    vec2,
};

const SCREEN: Vec2 = vec2(400.0, 120.0);

/// Slack for the frame margin and glyph bearings around an "hugs this edge"
/// assertion.
const EDGE_TOL: f32 = 12.0;

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

/// (job text, absolute layout rect, resolved direction) for every text shape
/// painted this pass.
///
/// `Galley::rect` is anchored on its halign edge (`rect.left() == 0.0` for
/// `Align::LEFT`, `rect.right() == 0.0` for `Align::RIGHT`), so the translated
/// rect shows exactly where the anchor put the text.
fn painted_texts(output: &FullOutput) -> Vec<(String, Rect, Option<TextDirection>)> {
    fn collect(shape: &Shape, out: &mut Vec<(String, Rect, Option<TextDirection>)>) {
        match shape {
            Shape::Text(text) => {
                out.push((
                    text.galley.job.text.clone(),
                    text.galley.rect.translate(text.pos.to_vec2()),
                    text.galley.job.direction,
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

/// The ink rect and resolved direction of the painted text whose galley job text
/// equals `needle` (the input value, or the hint text while the field is empty).
fn painted_text(output: &FullOutput, needle: &str) -> (Rect, Option<TextDirection>) {
    painted_texts(output)
        .into_iter()
        .find(|(text, _, _)| text == needle)
        .map(|(_, rect, direction)| (rect, direction))
        .unwrap_or_else(|| panic!("no painted text galley for {needle:?}"))
}

/// Show a singleline text edit with the given value and optional explicit align,
/// and return (widget rect, painted value ink rect, resolved direction).
fn value_geometry(
    rtl: bool,
    text: &mut String,
    align: Option<Align2>,
) -> (Rect, Rect, Option<TextDirection>) {
    let needle = text.clone();
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let mut widget = Rect::NOTHING;
    let output = run(&ctx, |ui| {
        let mut edit = TextEdit::singleline(&mut *text);
        if let Some(align) = align {
            edit = edit.align(align);
        }
        widget = ui.add(edit).rect;
    });
    let (ink, direction) = painted_text(&output, &needle);
    (widget, ink, direction)
}

/// The input text hugs the field's reading-start edge (right under a
/// right-to-left interface, left under the default) with a right-to-left base
/// direction under RTL.
#[test]
fn text_edit_input_hugs_the_reading_start() {
    for rtl in [false, true] {
        let mut text = String::from("سلام");
        let (field, ink, direction) = value_geometry(rtl, &mut text, None);
        assert_eq!(
            direction,
            Some(TextDirection::RightToLeft),
            "the Arabic value should lay out with a right-to-left base direction (rtl = {rtl})"
        );
        let start_hug = if rtl {
            field.right() - ink.right()
        } else {
            ink.left() - field.left()
        };
        let end_clearance = if rtl {
            ink.left() - field.left()
        } else {
            field.right() - ink.right()
        };
        assert!(
            (0.0..=EDGE_TOL).contains(&start_hug),
            "the text should hug the reading-start edge (rtl = {rtl}: hug {start_hug}, ink {ink:?}, field {field:?})"
        );
        assert!(
            end_clearance > EDGE_TOL,
            "the text should clear the reading-end edge (rtl = {rtl}: clearance {end_clearance}, ink {ink:?}, field {field:?})"
        );
    }
}

/// A Latin-first value in a right-to-left field still reads as a right-to-left
/// row anchored at the reading start (the direction is pinned from the
/// interface, like `menu_item_button` stamps its labels); under LTR it stays
/// left-anchored with a left-to-right direction — the unchanged regression.
#[test]
fn latin_input_direction_follows_the_interface() {
    for (rtl, expected_direction) in [
        (false, TextDirection::LeftToRight),
        (true, TextDirection::RightToLeft),
    ] {
        let mut text = String::from("Hello");
        let (field, ink, direction) = value_geometry(rtl, &mut text, None);
        assert_eq!(
            direction,
            Some(expected_direction),
            "the Latin value's base direction should follow the interface (rtl = {rtl})"
        );
        let start_hug = if rtl {
            field.right() - ink.right()
        } else {
            ink.left() - field.left()
        };
        assert!(
            (0.0..=EDGE_TOL).contains(&start_hug),
            "the text should hug the reading-start edge (rtl = {rtl}: hug {start_hug}, ink {ink:?}, field {field:?})"
        );
    }
}

/// An explicit `align` still wins over the flipped default, and places the text
/// block physically in both interface directions: centered fields stay centered
/// and left/right-aligned fields hug their named edge under both LTR and RTL.
/// (Only the *default* is reading-direction-aware — see the module comment.)
#[test]
fn explicit_align_is_still_honored() {
    for rtl in [false, true] {
        let mut centered = String::from("سلام");
        let (field, ink, _) = value_geometry(rtl, &mut centered, Some(Align2::CENTER_CENTER));
        let center_off = (ink.center().x - field.center().x).abs();
        assert!(
            center_off <= EDGE_TOL,
            "centered text should sit centered (rtl = {rtl}: offset {center_off}, ink {ink:?}, field {field:?})"
        );

        let mut right = String::from("سلام");
        let (field, ink, _) = value_geometry(rtl, &mut right, Some(Align2::RIGHT_TOP));
        assert!(
            (field.right() - ink.right()).abs() <= EDGE_TOL,
            "right-aligned text should hug the right edge (rtl = {rtl}: ink {ink:?}, field {field:?})"
        );

        let mut left = String::from("سلام");
        let (field, ink, _) = value_geometry(rtl, &mut left, Some(Align2::LEFT_TOP));
        assert!(
            (ink.left() - field.left()).abs() <= EDGE_TOL,
            "left-aligned text should hug the left edge (rtl = {rtl}: ink {ink:?}, field {field:?})"
        );
    }
}

/// The hint (placeholder) text mirrors the input: under RTL it anchors at the
/// reading start with a right-to-left base direction — including a Latin-first
/// hint, whose direction is pinned like the value's — and under LTR it stays
/// left-anchored and left-to-right.
#[test]
fn hint_text_mirrors_the_input() {
    for rtl in [false, true] {
        for (hint, ltr_direction) in [
            // An Arabic hint under LTR still infers a right-to-left base
            // direction from its first strong character (first-strong rules);
            // only a Latin-first hint under RTL needs the direction pinned.
            ("اكتب هنا", TextDirection::RightToLeft),
            ("Type here", TextDirection::LeftToRight),
        ] {
            let expected_direction = if rtl {
                TextDirection::RightToLeft
            } else {
                ltr_direction
            };
            let mut text = String::new();
            let ctx = Context::default();
            ctx.set_right_to_left(rtl);
            let mut widget = Rect::NOTHING;
            let output = run(&ctx, |ui| {
                widget = ui.add(TextEdit::singleline(&mut text).hint_text(hint)).rect;
            });
            let (ink, direction) = painted_text(&output, hint);
            assert_eq!(
                direction,
                Some(expected_direction),
                "the hint's base direction should follow the interface (rtl = {rtl}, hint = {hint:?})"
            );
            let start_hug = if rtl {
                widget.right() - ink.right()
            } else {
                ink.left() - widget.left()
            };
            let end_clearance = if rtl {
                ink.left() - widget.left()
            } else {
                widget.right() - ink.right()
            };
            assert!(
                (0.0..=EDGE_TOL).contains(&start_hug),
                "the hint should hug the reading-start edge (rtl = {rtl}, hint = {hint:?}: hug {start_hug}, ink {ink:?}, widget {widget:?})"
            );
            assert!(
                end_clearance > EDGE_TOL,
                "the hint should clear the reading-end edge (rtl = {rtl}, hint = {hint:?}: clearance {end_clearance})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Caret/cursor geometry: the caret and selection layers must follow the
// reading direction (cursor columns are logical offsets over visual-order
// glyphs).

fn edit_id() -> Id {
    Id::unique("text_edit_rtl_caret_test")
}

fn blink_off(ctx: &Context) {
    // Deterministic caret: no blinking phase that could hide the segment.
    ctx.style_mut_of(ctx.theme(), |style| style.visuals.text_cursor.blink = false);
}

/// All line segments painted this pass, in screen coordinates (the caret is
/// one).
fn painted_line_segments(output: &FullOutput) -> Vec<[Pos2; 2]> {
    fn collect(shape: &Shape, out: &mut Vec<[Pos2; 2]>) {
        match shape {
            Shape::LineSegment { points, .. } => out.push([points[0], points[1]]),
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

/// The near-vertical caret segment painted for the focused widget (a line
/// segment taller than it is wide).
fn painted_caret(output: &FullOutput, field: Rect) -> Option<[Pos2; 2]> {
    // The caret rect is expanded 1.5 px when painted, so tolerate a sliver
    // outside the field edge:
    let field = field.expand(2.0);
    painted_line_segments(output).into_iter().find(|[a, b]| {
        (a.x - b.x).abs() < 1.0
            && (a.y - b.y).abs() > 4.0
            && (field.contains(*a) || field.contains(*b))
    })
}

/// Store a single-char caret at `index` on a focused widget, then paint one
/// frame and return (field rect, output).
fn run_with_caret(rtl: bool, text: &mut String, index: usize) -> (Rect, FullOutput) {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    blink_off(&ctx);
    let mut field = Rect::NOTHING;
    drop(run(&ctx, |ui| {
        ui.memory_mut(|mem| mem.request_focus(edit_id()));
        let mut out = TextEdit::singleline(&mut *text).id(edit_id()).show(ui);
        out.state
            .cursor
            .set_char_range(Some(CCursorRange::one(CCursor::new(index))));
        out.state.store(ui.ctx(), edit_id());
        field = out.response.rect;
    }));

    let output = run(&ctx, |ui| {
        let out = TextEdit::singleline(&mut *text).id(edit_id()).show(ui);
        field = out.response.rect;
    });
    (field, output)
}

/// The painted text shape's position and resolved direction.
fn painted_text_shape(output: &FullOutput) -> Option<(Pos2, Option<TextDirection>)> {
    fn collect(shape: &Shape, out: &mut Option<(Pos2, Option<TextDirection>)>) {
        match shape {
            Shape::Text(text) => {
                *out = Some((text.pos, text.galley.job.direction));
            }
            Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut out = None;
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut out);
    }
    out
}

/// The caret at text-start sits at the ink's reading-start edge: the visual
/// RIGHT edge of a right-to-left galley (the Arabic value reads RTL under
/// either interface direction — the direction is the galley's, not the
/// interface's), the visual LEFT edge of a left-to-right galley. The painted
/// text shape's position is exactly that reading-start edge in both
/// directions.
#[test]
fn caret_at_text_start_sits_at_the_reading_start_edge() {
    for (rtl_interface, value) in [(true, "سلام عربي"), (false, "سلام عربي"), (false, "Hello")]
    {
        let mut text = String::from(value);
        let (field, output) = run_with_caret(rtl_interface, &mut text, 0);
        let (ink_start, direction) = painted_text_shape(&output)
            .unwrap_or_else(|| panic!("no text painted for {value:?} (rtl = {rtl_interface})"));
        let caret = painted_caret(&output, field)
            .unwrap_or_else(|| panic!("no caret painted for {value:?} (rtl = {rtl_interface})"));
        assert!(
            (caret[0].x - ink_start.x).abs() <= EDGE_TOL,
            "the caret at text-start must hug the ink's reading-start edge \
             (value {value:?}, rtl interface = {rtl_interface}, direction {direction:?}: \
             caret {}, ink start {}, field {field:?})",
            caret[0].x,
            ink_start.x
        );
        if value == "سلام عربي" {
            // The Arabic value must lay out right-to-left under either
            // interface direction, so its reading-start edge (where the caret
            // hugs) is the ink's visual RIGHT edge:
            assert_eq!(direction, Some(TextDirection::RightToLeft));
        } else {
            assert_eq!(direction, Some(TextDirection::LeftToRight));
        }
    }
}

/// The screen-space bounding box of the painted selection highlight: the
/// galley-mesh vertices carrying the selection background color.
fn painted_selection_rect(output: &FullOutput, selection_color: egui::Color32) -> Option<Rect> {
    fn collect(shape: &Shape, color: egui::Color32, out: &mut Option<Rect>) {
        match shape {
            Shape::Text(text) => {
                for row in &text.galley.rows {
                    for vertex in &row.visuals.mesh.vertices {
                        if vertex.color == color {
                            // Rendered vertex position: shape pos + row pos +
                            // mesh (row-relative) position:
                            let pos = text.pos + row.pos.to_vec2() + vertex.pos.to_vec2();
                            *out = Some(match *out {
                                Some(rect) => rect.union(Rect::from_min_size(pos, Vec2::ZERO)),
                                None => Rect::from_min_size(pos, Vec2::ZERO),
                            });
                        }
                    }
                }
            }
            Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, color, out);
                }
            }
            _ => {}
        }
    }
    let mut out = None;
    for clipped in &output.shapes {
        collect(&clipped.shape, selection_color, &mut out);
    }
    out
}

/// Selecting everything highlights exactly the visual ink: under RTL the
/// highlight hugs the field's right (reading-start) edge and spans the text
/// width; under LTR it hugs the left edge.
#[test]
fn select_all_highlights_the_visual_range() {
    for (rtl, start_edge) in [(true, "right"), (false, "left")] {
        let mut text = String::from("سلام عربي");
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        let selection_color = ctx.style_of(ctx.theme()).visuals.selection.bg_fill;
        let mut field = Rect::NOTHING;
        drop(run(&ctx, |ui| {
            ui.memory_mut(|mem| mem.request_focus(edit_id()));
            let mut out = TextEdit::singleline(&mut text).id(edit_id()).show(ui);
            out.state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(0),
                CCursor::new(text.chars().count()),
            )));
            out.state.store(ui.ctx(), edit_id());
            field = out.response.rect;
        }));

        let output = run(&ctx, |ui| {
            let out = TextEdit::singleline(&mut text).id(edit_id()).show(ui);
            field = out.response.rect;
        });

        let selection = painted_selection_rect(&output, selection_color)
            .unwrap_or_else(|| panic!("no selection highlight painted (rtl = {rtl})"));
        let distance_from_start = if rtl {
            field.right() - selection.right()
        } else {
            selection.left() - field.left()
        };
        assert!(
            (0.0..=EDGE_TOL).contains(&distance_from_start),
            "the select-all highlight must hug the reading-start ({start_edge}) edge \
             (rtl = {rtl}: selection {selection:?}, field {field:?})"
        );
    }
}

/// An overflowing singleline field scrolls to reveal the caret: an RTL field
/// slides its text rightward to expose the visually-left reading end, and
/// returning to text-start re-anchors the text at the reading start. The LTR
/// behaviour keeps working.
#[test]
fn singleline_overflow_scrolls_to_the_caret() {
    for (rtl, caret_index) in [(true, 40), (true, 0), (false, 40)] {
        let mut text = "سلام ".repeat(8); // far wider than the narrow field
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        blink_off(&ctx);
        let mut field = Rect::NOTHING;
        drop(run(&ctx, |ui| {
            ui.memory_mut(|mem| mem.request_focus(edit_id()));
            let mut out = ui
                .allocate_ui(vec2(120.0, 30.0), |ui| {
                    TextEdit::singleline(&mut text).id(edit_id()).show(ui)
                })
                .inner;
            out.state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(caret_index))));
            out.state.store(ui.ctx(), edit_id());
            field = out.response.rect;
        }));

        let output = run(&ctx, |ui| {
            let out = ui
                .allocate_ui(vec2(120.0, 30.0), |ui| {
                    TextEdit::singleline(&mut text).id(edit_id()).show(ui)
                })
                .inner;
            field = out.response.rect;
        });

        let caret = painted_caret(&output, field)
            .unwrap_or_else(|| panic!("no caret painted (rtl = {rtl}, caret = {caret_index})"));
        let caret_inside = field.left() - 1.0 <= caret[0].x && caret[0].x <= field.right() + 1.0;
        match (rtl, caret_index) {
            (true, 40) => assert!(
                caret_inside,
                "an overflowing RTL field must scroll so the caret at the reading end \
                 stays visible (caret {:?}, field {field:?})",
                caret[0]
            ),
            (true, 0) => assert!(
                (field.right() - caret[0].x).abs() <= EDGE_TOL,
                "returning to text-start must re-anchor the text at the reading start \
                 (caret {:?}, field {field:?})",
                caret[0]
            ),
            (false, 40) => assert!(
                caret_inside,
                "the LTR scroll behaviour must keep working (caret {:?}, field {field:?})",
                caret[0]
            ),
            _ => unreachable!(),
        }
    }
}
