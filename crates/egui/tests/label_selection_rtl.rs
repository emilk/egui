//! Regression tests for cross-widget text selection over selectable labels:
//! when a selection spans two widgets, each widget must select the half the
//! *reading order* dictates, not the half its layout-encounter order suggests.
//! [`egui::Sides`] places its two panels physically (it does not mirror them)
//! and adds the left one first — so under a right-to-left interface the
//! reading-first label sits on the physical right while being encountered
//! second, which is exactly the case the encounter-order assumption gets
//! wrong.
//!
//! The labels' texts follow their interface direction (Arabic under RTL,
//! Latin under LTR), because the reading-order decision is signed by the
//! galley's paragraph direction: a mixed-direction widget pair is an
//! approximation the fill logic documents, not something these tests pin.
//!
//! No snapshot assertions; painted selection-rect geometry and copied text
//! only. Every mirrored case carries its LTR twin, so this file doubles as
//! the LTR-unchanged regression.

use std::sync::Arc;

use egui::epaint::{Galley, Shape};
use egui::{
    Color32, Context, Event, FullOutput, Label, Modifiers, PointerButton, Pos2, RawInput, Rect,
    Sides, Ui, Vec2, pos2, text::CCursorRange, vec2,
};

const SCREEN: Vec2 = vec2(400.0, 120.0);

/// The selectable labels, one per `Sides` panel: pure Arabic under RTL (so
/// both lay out right-to-left) and pure Latin under LTR, matching the
/// interface's reading direction.
fn panel_texts(rtl: bool) -> (&'static str, &'static str) {
    if rtl {
        ("نهار بارد", "ليل طويل") // (left panel, right panel)
    } else {
        ("north wind", "south sun")
    }
}

/// The geometry of one label: where its galley was painted and the galley.
type GalleyInfo = (Pos2, Arc<Galley>);

fn input(time: f64, events: Vec<Event>) -> RawInput {
    RawInput {
        time: Some(time),
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
        events,
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, time: f64, events: Vec<Event>, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input(time, events), run_ui);
    output.textures_delta.clear();
    output
}

fn pointer_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn sides_labels(ui: &mut Ui, left_text: &str, right_text: &str) {
    Sides::new().show(
        ui,
        |ui| {
            ui.add(Label::new(left_text).selectable(true));
        },
        |ui| {
            ui.add(Label::new(right_text).selectable(true));
        },
    );
}

/// (galley job text, shape position, galley) for every text shape painted.
fn painted_galleys(output: &FullOutput) -> Vec<(String, Pos2, Arc<Galley>)> {
    fn collect(shape: &Shape, out: &mut Vec<(String, Pos2, Arc<Galley>)>) {
        match shape {
            Shape::Text(text) => out.push((
                text.galley.job.text.clone(),
                text.pos,
                Arc::clone(&text.galley),
            )),
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

/// The ink rect and galley of the painted text whose galley job text equals
/// `needle`.
fn painted_galley(output: &FullOutput, needle: &str) -> GalleyInfo {
    painted_galleys(output)
        .into_iter()
        .find(|(text, _, _)| text == needle)
        .map(|(_, pos, galley)| (pos, galley))
        .unwrap_or_else(|| panic!("no painted text galley for {needle:?}"))
}

fn galley_center(info: &GalleyInfo) -> Pos2 {
    let (shape_pos, galley) = info;
    galley.rect.translate(shape_pos.to_vec2()).center()
}

/// The x-range of the painted selection highlight inside the label whose
/// galley text is `needle`, if any.
fn painted_selection_x_range(
    output: &FullOutput,
    selection_color: Color32,
    needle: &str,
) -> Option<(f32, f32)> {
    fn collect(shape: &Shape, color: Color32, needle: &str, out: &mut Option<(f32, f32)>) {
        match shape {
            Shape::Text(text) if text.galley.job.text == needle => {
                for row in &text.galley.rows {
                    for vertex in &row.visuals.mesh.vertices {
                        if vertex.color == color {
                            let x = text.pos.x + row.pos.x + vertex.pos.x;
                            *out = Some(match *out {
                                Some((min, max)) => (min.min(x), max.max(x)),
                                None => (x, x),
                            });
                        }
                    }
                }
            }
            Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, color, needle, out);
                }
            }
            _ => {}
        }
    }
    let mut out = None;
    for clipped in &output.shapes {
        collect(&clipped.shape, selection_color, needle, &mut out);
    }
    out
}

/// The screen-space x-range a selection over `range` paints with, computed
/// from the label's own galley.
fn expected_x_range(info: &GalleyInfo, range: CCursorRange) -> (f32, f32) {
    let (shape_pos, galley) = info;
    let [min, max] = range.sorted_cursors();
    let lo = galley.pos_from_cursor(min).left() + shape_pos.x;
    let hi = galley.pos_from_cursor(max).left() + shape_pos.x;
    (lo.min(hi), lo.max(hi))
}

/// Everything one cross-widget drag produces.
struct CrossDrag {
    output: FullOutput,
    left: GalleyInfo,
    right: GalleyInfo,
    selection_color: Color32,
    press: Pos2,
    release: Pos2,
}

/// Drag-select from one `Sides` panel label into the other, starting in the
/// right-panel label when `press_right`, and return the release-frame output
/// plus both labels' geometry.
fn drag_between_sides(rtl: bool, press_right: bool) -> CrossDrag {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    ctx.style_mut_of(ctx.theme(), |style| {
        style.interaction.selectable_labels = true;
        style.interaction.multi_widget_text_select = true;
    });
    let selection_color = ctx.style_of(ctx.theme()).visuals.selection.bg_fill;
    let (left_text, right_text) = panel_texts(rtl);

    // Settle the layout and collect the two labels' galley geometry:
    let settle = run(&ctx, 0.0, vec![], |ui| {
        sides_labels(ui, left_text, right_text);
    });
    let left = painted_galley(&settle, left_text);
    let right = painted_galley(&settle, right_text);

    // The panels are physical: the right-panel label must sit to the right of
    // the left-panel one, on the same row band, in both interface directions:
    assert!(
        right.0.x > left.0.x,
        "Sides must place its panels physically (right {right:?}, left {left:?})"
    );
    assert!(
        (right.0.y - left.0.y).abs() <= 2.0,
        "the two panel labels must share one row band (right {right:?}, left {left:?})"
    );

    let (press, release) = if press_right {
        (galley_center(&right), galley_center(&left))
    } else {
        (galley_center(&left), galley_center(&right))
    };

    drop(run(&ctx, 1.0, vec![pointer_button(press, true)], |ui| {
        sides_labels(ui, left_text, right_text);
    }));
    drop(run(&ctx, 1.1, vec![Event::PointerMoved(release)], |ui| {
        sides_labels(ui, left_text, right_text);
    }));
    let output = run(&ctx, 1.2, vec![pointer_button(release, false)], |ui| {
        sides_labels(ui, left_text, right_text);
    });

    CrossDrag {
        output,
        left,
        right,
        selection_color,
        press,
        release,
    }
}

/// Drag-select from the reading-first label into the reading-second one: each
/// label must select the half the reading order dictates — the reading-first
/// label from the press point reading-forward to its end, the reading-second
/// label from its start up to the release point. Under RTL the `Sides`
/// encounter order is reversed relative to the reading order, which is what
/// the layout-encounter-order fill logic gets wrong.
#[test]
fn cross_widget_selection_selects_the_reading_order_halves() {
    let (rtl, press_right) = (true, true);
    assert_cross_widget_halves(rtl, press_right);
}

/// LTR twin: in a left-to-right interface the encounter order already equals
/// the reading order, so the selection halves are identical before and after.
#[test]
fn cross_widget_selection_halves_ltr_twin() {
    let (rtl, press_right) = (false, false);
    assert_cross_widget_halves(rtl, press_right);
}

/// The shared drag-and-assert body of the two cross-widget tests above.
fn assert_cross_widget_halves(rtl: bool, press_right: bool) {
    let drag = drag_between_sides(rtl, press_right);
    let (reading_first, reading_second) = if rtl {
        (&drag.right, &drag.left)
    } else {
        (&drag.left, &drag.right)
    };
    let (press_info, release_info) = if press_right {
        (&drag.right, &drag.left)
    } else {
        (&drag.left, &drag.right)
    };
    let press_cursor = press_info.1.cursor_from_pos(drag.press - press_info.0);
    let release_cursor = release_info
        .1
        .cursor_from_pos(drag.release - release_info.0);

    // Expected halves: from the press point reading-forward to the end of
    // the reading-first label, and from the start of the reading-second
    // label up to the release point:
    let first_half = CCursorRange::two(press_cursor, reading_first.1.end());
    let second_half = CCursorRange::two(reading_second.1.begin(), release_cursor);

    for (info, expected_range, name) in [
        (reading_first, first_half, "reading-first"),
        (reading_second, second_half, "reading-second"),
    ] {
        let painted = painted_selection_x_range(&drag.output, drag.selection_color, info.1.text())
            .unwrap_or_else(|| panic!("no selection painted in the {name} label (rtl = {rtl})"));
        let expected = expected_x_range(info, expected_range);
        assert!(
            (painted.0 - expected.0).abs() <= 1.0 && (painted.1 - expected.1).abs() <= 1.0,
            "the {name} label must select its reading-order half \
             (rtl = {rtl}: painted {painted:?}, expected {expected:?})"
        );
    }
}

// ---------------------------------------------------------------------------
// Granular (double-click) drag across two labels on one row: which side of
// the anchor word the pointer lands on must follow the ANCHOR's reading
// direction, not the raw screen x.

fn horizontal_labels(ui: &mut Ui, first_text: &str, second_text: &str) {
    ui.horizontal(|ui| {
        ui.add(Label::new(first_text).selectable(true));
        ui.add(Label::new(second_text).selectable(true));
    });
}

/// The x of a point strictly inside a row's reading-start (first) word: the
/// visual right quarter of an RTL row, the visual left quarter of an LTR one.
fn reading_start_word_x(rect: Rect, rtl: bool) -> f32 {
    if rtl {
        rect.right() - 0.15 * rect.width()
    } else {
        rect.left() + 0.15 * rect.width()
    }
}

/// Double-click the reading-start word of the first label, drag horizontally
/// into the second label, release there, and copy. The granular anchor is the
/// double-clicked word; while the pointer is over the second label, the
/// before/after-anchor decision must be signed by the anchor's reading
/// direction, so the selection ends up covering the anchor word's
/// reading-forward span in the first label plus the unit under the pointer in
/// the second label.
fn granular_drag_copy(rtl: bool) -> String {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    ctx.style_mut_of(ctx.theme(), |style| {
        style.interaction.selectable_labels = true;
        style.interaction.multi_widget_text_select = true;
    });
    let (first_text, second_text) = if rtl {
        ("ليل طويل", "نهار بارد")
    } else {
        ("one two", "three four")
    };

    let settle = run(&ctx, 0.0, vec![], |ui| {
        horizontal_labels(ui, first_text, second_text);
    });
    let first = painted_galley(&settle, first_text);
    let second = painted_galley(&settle, second_text);

    // The mirrored horizontal row places the first label at the reading start
    // (the right under RTL, the left under LTR), on one row band — so the
    // layout-encounter order also equals the reading order and the copy text
    // below is the final reading-order string:
    let first_rect = first.1.rect.translate(first.0.to_vec2());
    let second_rect = second.1.rect.translate(second.0.to_vec2());
    let first_is_reading_first = if rtl {
        first_rect.left() > second_rect.right() - 1.0
    } else {
        first_rect.right() < second_rect.left() + 1.0
    };
    assert!(
        first_is_reading_first,
        "the first label must sit reading-first (first {first_rect:?}, second {second_rect:?})"
    );
    assert!(
        (first_rect.top() - second_rect.top()).abs() <= 2.0,
        "the two labels must share one row band (first {first_rect:?}, second {second_rect:?})"
    );

    let press = pos2(reading_start_word_x(first_rect, rtl), first_rect.center().y);
    let release = pos2(
        reading_start_word_x(second_rect, rtl),
        second_rect.center().y,
    );

    // Double-click (two presses within the multi-click window at the same
    // spot), then drag into the second label and release:
    drop(run(&ctx, 2.0, vec![pointer_button(press, true)], |ui| {
        horizontal_labels(ui, first_text, second_text);
    }));
    drop(run(&ctx, 2.1, vec![pointer_button(press, false)], |ui| {
        horizontal_labels(ui, first_text, second_text);
    }));
    drop(run(&ctx, 2.2, vec![pointer_button(press, true)], |ui| {
        horizontal_labels(ui, first_text, second_text);
    }));
    drop(run(&ctx, 2.3, vec![Event::PointerMoved(release)], |ui| {
        horizontal_labels(ui, first_text, second_text);
    }));
    drop(run(&ctx, 2.4, vec![pointer_button(release, false)], |ui| {
        horizontal_labels(ui, first_text, second_text);
    }));
    let output = run(&ctx, 2.5, vec![Event::Copy], |ui| {
        horizontal_labels(ui, first_text, second_text);
    });
    copied_text(&output)
}

/// The text a frame's copy commands put on the clipboard.
fn copied_text(output: &FullOutput) -> String {
    output
        .platform_output
        .commands
        .iter()
        .find_map(|cmd| match cmd {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Double-click the first word of the right (reading-first) label, drag into
/// the left label and release there: the selection must cover the whole
/// reading-first label (from the anchor word's start reading-forward) plus
/// the second label's word under the pointer, copied in reading order. Before
/// the fix the raw-x compare inverted the sides: the anchor's reading-start
/// was treated as its end and the picked unit end flipped, garbling the copy.
#[test]
fn granular_drag_extends_in_reading_order() {
    assert_eq!(
        granular_drag_copy(true),
        "ليل طويل نهار",
        "the copied text must be the anchor word's reading-forward span \
         (the whole first label) followed by the second label's unit, \
         in reading order"
    );
}

/// LTR twin: with an LTR-reading anchor the signed compare reduces to the raw
/// screen-x compare, so the copied text is unchanged.
#[test]
fn granular_drag_reading_order_ltr_twin() {
    assert_eq!(
        granular_drag_copy(false),
        "one two three",
        "the LTR granular drag must keep copying first-label word-one plus \
         the second label's unit in encounter (= reading) order"
    );
}
