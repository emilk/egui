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
use egui::{Align2, Context, FullOutput, Pos2, RawInput, Rect, TextEdit, Ui, Vec2, vec2};

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
