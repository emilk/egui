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
use egui::widgets::text_edit::TextEditState;
use egui::{
    Align2, Context, Event, FullOutput, Id, Modifiers, PointerButton, Pos2, RawInput, Rect,
    TextEdit, Ui, Vec2, pos2,
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

/// Run one ui pass with pointer events at the given time, discarding the
/// texture deltas (these tests never render).
fn run_at(ctx: &Context, time: f64, events: Vec<Event>, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
            time: Some(time),
            events,
            ..Default::default()
        },
        run_ui,
    );
    output.textures_delta.clear();
    output
}

// ---------------------------------------------------------------------------
// Hover preview: the opt-in text cursor preview (`Visuals::text_cursor
// .preview`, default off) paints the caret a click would place, so it must
// follow the same scrolled-galley mapping as the real caret.

/// The preview caret ("where would a click land?") tracks the scrolled text:
/// an overflowing singleline field stores a scroll offset, and the previewed
/// caret must paint exactly where the real caret for the same galley-local
/// position paints, in both reading directions.
///
/// The preview paints while the pointer interacts with the field (its paint
/// block reads `Response::interact_pointer_pos`, which egui fills only while
/// the pointer is down, or on the release frame of a drag). A press and drag
/// move the real caret with the pointer, but the release frame does *not*
/// move it again — so on that frame the preview is the only caret near the
/// release position, 50 px past the last drag position. Before the fix the
/// preview was translated without the stored scroll offset, which displaced
/// it past the field's clip and left nothing near the release point.
#[test]
fn preview_caret_tracks_the_scrolled_text() {
    for (rtl, value) in [
        (true, "سلام سلام سلام سلام سلام سلام سلام سلام "),
        (false, "lorem lorem lorem lorem lorem lorem lorem lorem "),
    ] {
        let mut text = String::from(value);
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        blink_off(&ctx);
        ctx.style_mut_of(ctx.theme(), |style| {
            style.visuals.text_cursor.preview = true;
            style.visuals.text_cursor.blink = false;
        });
        let mut field = Rect::NOTHING;
        // Frame 1: focus + a mid-text caret so the clip/scroll branch engages
        // and stores a non-zero scroll offset:
        drop(run(&ctx, |ui| {
            ui.memory_mut(|mem| mem.request_focus(edit_id()));
            let mut out = ui
                .allocate_ui(vec2(120.0, 30.0), |ui| {
                    TextEdit::singleline(&mut text).id(edit_id()).show(ui)
                })
                .inner;
            out.state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(20))));
            out.state.store(ui.ctx(), edit_id());
            field = out.response.rect;
        }));

        // Press inside the field, drag a little, then release further on. The
        // press clears the pointer velocity history, so the three pointer
        // frames rebuild it and the release frame counts as moving. Pressing
        // and dragging never scrolls past the stored window, so the stored
        // scroll offset (the preview's missing term before the fix) stays
        // large enough to displace the preview out of the field's clip.
        let y = field.center().y;
        let at = |dx: f32| pos2(field.left() + dx, y);
        let press = Event::PointerButton {
            pos: at(18.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::default(),
        };
        let release = Event::PointerButton {
            pos: at(100.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        };
        let mut output = None;
        for events in [
            vec![press],
            vec![Event::PointerMoved(at(35.0))],
            vec![Event::PointerMoved(at(50.0))],
            vec![release],
        ] {
            output = Some(run_events(&ctx, events, |ui| {
                let out = ui
                    .allocate_ui(vec2(120.0, 30.0), |ui| {
                        TextEdit::singleline(&mut text).id(edit_id()).show(ui)
                    })
                    .inner;
                field = out.response.rect;
            }));
        }
        let output = output.unwrap_or_else(|| panic!("no interaction frames ran"));

        let release_x = field.left() + 100.0;
        let near_release = painted_line_segments(&output).into_iter().find(|[a, b]| {
            (a.x - b.x).abs() < 1.0 && (a.y - b.y).abs() > 4.0 && (a.x - release_x).abs() <= 12.0
        });
        assert!(
            near_release.is_some(),
            "the preview caret must paint within 12 px of the release point \
             (rtl = {rtl}, release {release_x}, field {field:?})"
        );
    }
}

// Collapsing a selection with the arrow keys: ArrowLeft/ArrowRight collapse a
// non-empty selection onto the visual edge they point at (the reading end /
// start of the row), and typing afterwards lands there. The LTR twins pin the
// unchanged logical behavior.

/// Builds a key event, spelling all five `Event::Key` fields (`Event` derives
/// no `Default`, so functional-record-update cannot be used on the variant).
fn key_event(key: egui::Key, pressed: bool, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        pressed,
        modifiers,
        repeat: false,
        physical_key: None,
    }
}

/// Run one frame with explicit events and discard the texture deltas (these
/// tests never render).
fn run_events(ctx: &Context, events: Vec<egui::Event>, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(
        RawInput {
            events,
            ..input(SCREEN)
        },
        run_ui,
    );
    output.textures_delta.clear();
    output
}

fn pointer_press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn caret_x_is_inside_field(x: f32, field: Rect) -> bool {
    field.left() - 1.0 <= x && x <= field.right() + 1.0
}

/// An overflowing singleline field with an explicit centered alignment keeps
/// the generic anchoring: the text paints centered (mid-clipped, intersecting
/// the field) instead of being pushed almost fully out of it, and pointer
/// hit-testing stays the exact inverse of the painted position — a click
/// inside the field lands on a column that repaints back at the click point,
/// and clicking the same point again keeps that column.
///
/// (The RTL scroll branch's stored-offset algebra assumes a reading-start
/// anchored galley and used to steal this case from the generic anchoring:
/// the galley was pushed out of the field and clicks landed in the wrong
/// columns.)
#[test]
fn centered_overflow_keeps_click_hit_testing_exact() {
    /// Slack for the click-to-caret round trip: the caret snaps to the cluster
    /// boundary nearest the click.
    const CLICK_TOL: f32 = 30.0;

    for (rtl, value) in [(true, "سلام "), (false, "lorem ")] {
        let mut text = value.repeat(8); // far wider than the narrow field
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        blink_off(&ctx);

        // Focus the field with the caret at the middle column, then paint one
        // frame: the text span must still cross the field and the caret must
        // be visible inside it.
        let mut field = Rect::NOTHING;
        drop(run(&ctx, |ui| {
            ui.memory_mut(|mem| mem.request_focus(edit_id()));
            let mut out = ui
                .allocate_ui(vec2(120.0, 30.0), |ui| {
                    TextEdit::singleline(&mut text)
                        .id(edit_id())
                        .align(Align2::CENTER_CENTER)
                        .show(ui)
                })
                .inner;
            let middle = text.chars().count() / 2;
            out.state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(middle))));
            out.state.store(ui.ctx(), edit_id());
            field = out.response.rect;
        }));
        let output = run(&ctx, |ui| {
            let out = ui
                .allocate_ui(vec2(120.0, 30.0), |ui| {
                    TextEdit::singleline(&mut text)
                        .id(edit_id())
                        .align(Align2::CENTER_CENTER)
                        .show(ui)
                })
                .inner;
            field = out.response.rect;
        });
        let (ink, _) = painted_text(&output, &text);
        assert!(
            ink.intersects(field),
            "the overflowing centered text must still paint across the field \
             (rtl = {rtl}: ink {ink:?}, field {field:?})"
        );
        let caret = painted_caret(&output, field)
            .unwrap_or_else(|| panic!("no caret painted at the middle column (rtl = {rtl})"));
        assert!(
            caret_x_is_inside_field(caret[0].x, field),
            "the middle-column caret must be visible inside the centered field \
             (rtl = {rtl}: caret {:?}, field {field:?})",
            caret[0]
        );

        // Click inside the field: the caret must land on a column that repaints
        // back at the click point.
        let click_pos = field.center();
        let clicked_column = |ctx: &Context| {
            TextEditState::load(ctx, edit_id())
                .and_then(|state| state.cursor.char_range())
                .and_then(|range| range.single())
                .map(|cursor| cursor.index)
        };
        let mut show = |ui: &mut Ui| {
            ui.allocate_ui(vec2(120.0, 30.0), |ui| {
                TextEdit::singleline(&mut text)
                    .id(edit_id())
                    .align(Align2::CENTER_CENTER)
                    .show(ui);
            });
        };
        drop(run_at(
            &ctx,
            1.0,
            vec![Event::PointerMoved(click_pos)],
            &mut show,
        ));
        drop(run_at(
            &ctx,
            1.1,
            vec![pointer_press(click_pos, true)],
            &mut show,
        ));
        drop(run_at(
            &ctx,
            1.2,
            vec![pointer_press(click_pos, false)],
            &mut show,
        ));
        let column = clicked_column(&ctx)
            .unwrap_or_else(|| panic!("the click must place a caret (rtl = {rtl})"));
        let output = run(&ctx, &mut show);
        let caret = painted_caret(&output, field)
            .unwrap_or_else(|| panic!("no caret painted after the click (rtl = {rtl})"));
        assert!(
            (caret[0].x - click_pos.x).abs() <= CLICK_TOL,
            "the clicked caret must repaint back at the click point \
             (rtl = {rtl}: caret {:?}, click {click_pos:?})",
            caret[0]
        );
        assert!(
            caret_x_is_inside_field(caret[0].x, field),
            "the clicked caret must be visible inside the field \
             (rtl = {rtl}: caret {:?}, field {field:?})",
            caret[0]
        );

        // Clicking the same point again, well past the double-click window,
        // must keep the same column:
        drop(run_at(
            &ctx,
            3.0,
            vec![Event::PointerMoved(click_pos)],
            &mut show,
        ));
        drop(run_at(
            &ctx,
            3.1,
            vec![pointer_press(click_pos, true)],
            &mut show,
        ));
        drop(run_at(
            &ctx,
            3.2,
            vec![pointer_press(click_pos, false)],
            &mut show,
        ));
        let column_again = clicked_column(&ctx)
            .unwrap_or_else(|| panic!("the second click must place a caret (rtl = {rtl})"));
        assert_eq!(
            column_again, column,
            "clicking the same point must keep the same column (rtl = {rtl})"
        );
    }
}

/// Select everything in a focused singleline field, collapse the selection
/// with `key`, type "X" at the collapsed caret, and return the bound string.
fn collapse_selection_and_type(rtl: bool, word: &str, key: egui::Key) -> String {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let mut text = String::from(word);

    // Prime a select-all selection on the focused field:
    drop(run(&ctx, |ui| {
        ui.memory_mut(|mem| mem.request_focus(edit_id()));
        let mut out = TextEdit::singleline(&mut text).id(edit_id()).show(ui);
        out.state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(0),
            CCursor::new(word.chars().count()),
        )));
        out.state.store(ui.ctx(), edit_id());
    }));

    // Collapse the selection onto one of its visual edges:
    run_events(
        &ctx,
        vec![key_event(key, true, egui::Modifiers::NONE)],
        |ui| {
            ui.memory_mut(|mem| mem.request_focus(edit_id()));
            let out = TextEdit::singleline(&mut text).id(edit_id()).show(ui);
            out.state.store(ui.ctx(), edit_id());
        },
    );

    // Type at the collapsed caret:
    run_events(&ctx, vec![egui::Event::Text("X".into())], |ui| {
        ui.memory_mut(|mem| mem.request_focus(edit_id()));
        let out = TextEdit::singleline(&mut text).id(edit_id()).show(ui);
        out.state.store(ui.ctx(), edit_id());
    });

    text
}

/// `ArrowLeft` over a selection collapses the caret onto the reading end of a
/// right-to-left row — its visual left edge, the logical maximum — so typing
/// appends after the word.
#[test]
fn arrow_left_collapses_onto_the_reading_end_edge() {
    assert_eq!(
        collapse_selection_and_type(true, "سلام", egui::Key::ArrowLeft),
        "سلامX",
        "ArrowLeft must collapse onto the visual left (reading end) edge of an \
         RTL row, so typing appends after the word"
    );
}

/// `ArrowRight` over a selection collapses the caret onto the reading start of a
/// right-to-left row — its visual right edge, the logical minimum — so typing
/// prepends before the word.
#[test]
fn arrow_right_collapses_onto_the_reading_start_edge() {
    assert_eq!(
        collapse_selection_and_type(true, "سلام", egui::Key::ArrowRight),
        "Xسلام",
        "ArrowRight must collapse onto the visual right (reading start) edge of \
         an RTL row, so typing prepends before the word"
    );
}

/// LTR twin: in a left-to-right row the visual edges are the logical ones, so
/// collapsing keeps putting the caret at the logical start/end and typing
/// lands exactly as before.
#[test]
fn arrow_collapse_keeps_the_logical_edges_ltr() {
    assert_eq!(
        collapse_selection_and_type(false, "abc", egui::Key::ArrowLeft),
        "Xabc",
        "LTR: ArrowLeft must keep collapsing onto the logical start"
    );
    assert_eq!(
        collapse_selection_and_type(false, "abc", egui::Key::ArrowRight),
        "abcX",
        "LTR: ArrowRight must keep collapsing onto the logical end"
    );
}

// ---------------------------------------------------------------------------
// The shared direction-stamp policy: a field stamps the interface reading
// direction unconditionally (the HTML `dir=rtl` input convention), so even
// digits-only — first-weak — values read as a right-to-left row.

/// Digits-only values have no strong character, so first-strong inference has
/// no opinion; the field's unconditional stamp decides: right-to-left under a
/// right-to-left interface, left-to-right under the default. Characterization
/// for the stamp policy shared with labels (which resolve the other way — see
/// `tests/widget_text_direction.rs`).
#[test]
fn digits_only_value_direction_follows_the_interface() {
    for (rtl, expected) in [
        (false, TextDirection::LeftToRight),
        (true, TextDirection::RightToLeft),
    ] {
        let mut text = String::from("3.14");
        let (field, ink, direction) = value_geometry(rtl, &mut text, None);
        assert_eq!(
            direction,
            Some(expected),
            "the digits-only value's base direction should follow the interface (rtl = {rtl})"
        );
        let start_hug = if rtl {
            field.right() - ink.right()
        } else {
            ink.left() - field.left()
        };
        assert!(
            (0.0..=EDGE_TOL).contains(&start_hug),
            "the digits-only value should hug the reading-start edge (rtl = {rtl}: hug {start_hug}, ink {ink:?}, field {field:?})"
        );
    }
}

// ---------------------------------------------------------------------------
// Wrapped right-to-left rows (NG9): rows are produced in logical order, so
// the cursor machinery and the selection highlight follow the reading order
// across wrapped rows for the first time. The LTR twins pin the unchanged
// geometry.

/// The painted text galley whose job text equals `needle`, with the shape
/// position it was painted at.
fn painted_galley(
    output: &FullOutput,
    needle: &str,
) -> (std::sync::Arc<egui::epaint::Galley>, Pos2) {
    fn collect(
        shape: &Shape,
        needle: &str,
        out: &mut Option<(std::sync::Arc<egui::epaint::Galley>, Pos2)>,
    ) {
        match shape {
            Shape::Text(text) if text.galley.job.text == needle => {
                *out = Some((std::sync::Arc::clone(&text.galley), text.pos));
            }
            Shape::Vec(shapes) => {
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

/// One screen-space bounding rect per painted row carrying selection
/// vertices (as opposed to [`painted_selection_rect`], which unions them).
fn painted_selection_row_rects(
    output: &FullOutput,
    selection_color: egui::Color32,
) -> Option<Vec<Rect>> {
    fn collect(shape: &Shape, color: egui::Color32, out: &mut Vec<Rect>) {
        match shape {
            Shape::Text(text) => {
                for row in &text.galley.rows {
                    let mut row_rect: Option<Rect> = None;
                    for vertex in &row.visuals.mesh.vertices {
                        if vertex.color == color {
                            let pos = text.pos + row.pos.to_vec2() + vertex.pos.to_vec2();
                            row_rect = Some(match row_rect {
                                Some(rect) => rect.union(Rect::from_min_size(pos, Vec2::ZERO)),
                                None => Rect::from_min_size(pos, Vec2::ZERO),
                            });
                        }
                    }
                    if let Some(rect) = row_rect {
                        out.push(rect);
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
    let mut out = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, selection_color, &mut out);
    }
    if out.is_empty() { None } else { Some(out) }
}

/// A wrapped RTL galley's cursor machinery is logical-order-correct across
/// rows: each row's first logical column maps into that row's own band, the
/// caret advances toward the visual left within a right-to-left row and
/// top-down across rows, and every logical column round-trips through its
/// caret position. The LTR twin keeps the historic left-to-right geometry.
#[test]
fn wrapped_rtl_cursor_round_trips_through_caret_positions() {
    for (rtl, text) in [(true, "أول ثاني ثالث"), (false, "one two three")] {
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        let output = run(&ctx, |ui| {
            ui.allocate_ui(vec2(60.0, 90.0), |ui| {
                ui.label(text);
            });
        });
        let (galley, _) = painted_galley(&output, text);
        assert!(
            galley.rows.len() >= 2,
            "{text:?} must wrap in a 60px field (rtl = {rtl})"
        );
        let n: usize = galley
            .rows
            .iter()
            .map(|row| row.char_count_including_newline().0)
            .sum();
        assert_eq!(n, text.chars().count(), "rtl = {rtl}");

        // Rows are in reading order: column 0's caret is in the FIRST row's
        // band, the text-end caret in the LAST row's band, and every row's
        // first column maps into that row's own band:
        let caret0 = galley.pos_from_cursor(CCursor::new(0));
        let first_row = &galley.rows[0];
        assert!(
            caret0.min.y >= first_row.min_y() - 1.0 && caret0.max.y <= first_row.max_y() + 1.0,
            "column 0's caret must sit in the first row's band (rtl = {rtl})"
        );
        let caret_n = galley.pos_from_cursor(galley.end());
        let last_row = galley.rows.last().expect("at least one row");
        assert!(
            caret_n.min.y >= last_row.min_y() - 1.0 && caret_n.max.y <= last_row.max_y() + 1.0,
            "the text-end caret must sit in the last row's band (rtl = {rtl})"
        );
        let mut ccursor_index = 0usize;
        for (k, row) in galley.rows.iter().enumerate() {
            // Interior row starts are ambiguous CCursors (the previous row's
            // end is the same index); prefer the next row to probe THIS row:
            let caret = galley.pos_from_cursor(CCursor {
                index: egui::epaint::text::CharIndex(ccursor_index),
                prefer_next_row: true,
            });
            assert!(
                caret.min.y >= row.min_y() - 1.0 && caret.max.y <= row.max_y() + 1.0,
                "row {k}'s first column must map into row {k}'s band (rtl = {rtl})"
            );
            ccursor_index += row.char_count_including_newline().0;
        }

        // Caret direction within a row: toward the visual left for RTL,
        // the visual right for LTR. The row's interior columns probe with a
        // plain CCursor; the row's first column is an ambiguous CCursor (the
        // previous row's end is the same index) and prefers the next row:
        ccursor_index = 0usize;
        for row in &galley.rows {
            let count = row.char_count_including_newline().0;
            if count >= 2 {
                let xs: Vec<f32> = (0..count)
                    .map(|c| {
                        let ccursor = if c == 0 {
                            CCursor {
                                index: egui::epaint::text::CharIndex(ccursor_index),
                                prefer_next_row: true,
                            }
                        } else {
                            CCursor::new(ccursor_index + c)
                        };
                        galley.pos_from_cursor(ccursor).min.x
                    })
                    .collect();
                for pair in xs.windows(2) {
                    if rtl {
                        assert!(
                            pair[0] >= pair[1] - 1.0,
                            "RTL carets must not advance to the visual right within a row: {xs:?}"
                        );
                    } else {
                        assert!(
                            pair[0] <= pair[1] + 1.0,
                            "LTR carets must not advance to the visual left within a row: {xs:?}"
                        );
                    }
                }
            }
            ccursor_index += count;
        }

        // Round-trip: clicking each column's caret x returns that column:
        for c in 0..=n {
            let caret = galley.pos_from_cursor(CCursor::new(c));
            let back = galley.cursor_from_pos(vec2(caret.min.x, caret.min.y + 1.0));
            assert_eq!(
                back.index.0, c,
                "column {c} of {text:?} must round-trip through its caret (rtl = {rtl})"
            );
        }
    }
}

/// A selection spanning wrapped RTL rows highlights the right rows in the
/// reading direction: on every partially-selected row the highlight covers
/// the selected columns' reading-side extent — reaching the row's reading
/// end (visual left under RTL) when the selection covers the row's last
/// column, and the row's reading start (visual right under RTL) when it
/// covers the row's first column. The LTR twin mirrors.
///
/// Known residual (documented, not fixed here): with `keep_trailing_whitespace`
/// ([`TextEdit`]), a wrapped RTL-base row whose glyph vector leads with the
/// L1-reset break space excludes that vector-leading space from the halign ink
/// span (the pre-existing leading-whitespace semantic in
/// `halign_and_justify_row`), so the covered column highlights up to the
/// allocated edge rather than the ink edge. Caret positions and hit-testing
/// are glyph-driven and unaffected.
#[test]
fn wrapped_rtl_selection_spans_rows_in_reading_order() {
    for (rtl, text_str) in [(true, "أول ثاني ثالث"), (false, "one two three")] {
        let mut text = String::from(text_str);
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        let selection_color = ctx.style_of(ctx.theme()).visuals.selection.bg_fill;
        let mut field = Rect::NOTHING;
        drop(run(&ctx, |ui| {
            ui.memory_mut(|mem| mem.request_focus(edit_id()));
            let mut out = ui
                .allocate_ui(vec2(60.0, 90.0), |ui| {
                    TextEdit::multiline(&mut text).id(edit_id()).show(ui)
                })
                .inner;
            out.state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(1),
                CCursor::new(text.chars().count() - 2),
            )));
            out.state.store(ui.ctx(), edit_id());
            field = out.response.rect;
        }));

        let output = run(&ctx, |ui| {
            let out = ui
                .allocate_ui(vec2(60.0, 90.0), |ui| {
                    TextEdit::multiline(&mut text).id(edit_id()).show(ui)
                })
                .inner;
            field = out.response.rect;
        });

        let (galley, shape_pos) = painted_galley(&output, &text);
        assert!(
            galley.rows.len() >= 2,
            "{text:?} must wrap in a 60px field (rtl = {rtl})"
        );
        let selection_rects = painted_selection_row_rects(&output, selection_color)
            .unwrap_or_else(|| panic!("no selection painted (rtl = {rtl})"));

        // Match each painted selection rect to its row by y band, and check
        // the highlight's extent against the selected columns' reading side:
        let mut ccursor_index = 0usize;
        let mut matched = 0usize;
        for row in &galley.rows {
            let row_len = row.char_count_including_newline().0;
            let selected = (
                ccursor_index.max(1),
                (ccursor_index + row_len).min(text.chars().count() - 2),
            );
            let row_rect = Rect::from_min_size(shape_pos + row.pos.to_vec2(), row.size);
            let sel = selection_rects.iter().copied().find(|rect| {
                rect.top() >= row_rect.top() - 1.0 && rect.bottom() <= row_rect.bottom() + 1.0
            });
            if let (Some(sel), true) = (sel, selected.0 < selected.1) {
                matched += 1;
                // The highlight reaches the row's reading end iff the
                // selection covers the row's last column, and its reading
                // start iff it covers the row's first column:
                let covers_to_end = selected.1 == ccursor_index + row_len;
                let covers_from_start = selected.0 == ccursor_index;
                if covers_to_end {
                    let reaches_end = if rtl {
                        sel.left() <= row_rect.left() + EDGE_TOL
                    } else {
                        sel.right() >= row_rect.right() - EDGE_TOL
                    };
                    assert!(
                        reaches_end,
                        "the selection must reach the row's reading end (rtl = {rtl}: selection {sel:?}, row {row_rect:?}, columns {:?})",
                        (selected.0 - ccursor_index, selected.1 - ccursor_index)
                    );
                }
                if covers_from_start {
                    let reaches_start = if rtl {
                        sel.right() >= row_rect.right() - EDGE_TOL
                    } else {
                        sel.left() <= row_rect.left() + EDGE_TOL
                    };
                    assert!(
                        reaches_start,
                        "the selection must reach the row's reading start (rtl = {rtl}: selection {sel:?}, row {row_rect:?}, columns {:?})",
                        (selected.0 - ccursor_index, selected.1 - ccursor_index)
                    );
                }
            }
            ccursor_index += row_len;
        }
        assert!(
            matched >= 2,
            "the selection must span wrapped rows (rtl = {rtl}): {selection_rects:?}"
        );
    }
}
