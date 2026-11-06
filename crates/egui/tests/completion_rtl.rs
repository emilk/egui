//! The [`egui::CompletionPopup`] lays its suggestions out in reading order:
//! the insert text hugs the row's reading-start edge and a
//! [`egui::Suggestion::description`] sits past a growing spacer at the reading
//! end — the visual left in a right-to-left popup, the visual right in a
//! left-to-right one.
//!
//! Behavioral pin: the doc wording was LTR-physical,
//! not the behavior. If these assertions ever fail, the mirrored atom stream
//! is broken and the fix belongs in the atom layout, not in this test.

use egui::{Context, FullOutput, Id, Pos2, RawInput, Rect, Ui, Vec2, vec2};

const SCREEN: Vec2 = vec2(400.0, 200.0);

/// Slack for the button padding and glyph bearings between an ink edge and
/// the row edge it hugs.
const EDGE_TOL: f32 = 24.0;

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
/// job text equals `needle` (a suggestion may carry trailing whitespace).
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

/// The smallest filled rectangle that contains both inks — the suggestion
/// button's background row (the growing spacer between the two inks is
/// invisible, so the button's own rect is what brackets them).
fn suggestion_row(output: &FullOutput, insert: Rect, description: Rect) -> Rect {
    fn collect(
        shape: &egui::epaint::Shape,
        insert: Rect,
        description: Rect,
        out: &mut Option<Rect>,
    ) {
        match shape {
            egui::epaint::Shape::Rect(rect_shape)
                if rect_shape.fill != egui::Color32::TRANSPARENT
                    && rect_shape.rect.contains_rect(insert)
                    && rect_shape.rect.contains_rect(description) =>
            {
                *out = Some(match *out {
                    Some(row) if row.area() <= rect_shape.rect.area() => row,
                    _ => rect_shape.rect,
                });
            }
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, insert, description, out);
                }
            }
            _ => {}
        }
    }
    let mut out = None;
    for clipped in &output.shapes {
        collect(&clipped.shape, insert, description, &mut out);
    }
    out.unwrap_or_else(|| panic!("no suggestion row rect containing both inks"))
}

/// Show the popup with one described suggestion and return the frame where it
/// is open, plus its two ink rects and the button row rect.
fn popup_geometry(rtl: bool) -> (Rect, Rect, Rect) {
    let mut text = String::from("he");
    let id = Id::unique("completion_rtl_popup");
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let mut output = None;
    for frame in 0..3 {
        output = Some(run(&ctx, |ui| {
            if frame == 0 {
                ui.memory_mut(|mem| mem.request_focus(id));
            }
            let mut out = egui::CompletionPopup::new(id).show(
                ui,
                egui::TextEdit::singleline(&mut text),
                |_| vec![egui::Suggestion::new("hello ").description("a greeting")],
            );
            if frame == 0 {
                // Put the caret at the end so the current word is "he":
                out.text_edit
                    .state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(
                        egui::text::CCursor::new(2),
                    )));
                out.text_edit.state.store(ui.ctx(), id);
            }
            assert!(out.is_open, "the popup must be open by frame {frame}");
        }));
    }
    let output = output.unwrap_or_else(|| panic!("no frames ran"));
    let insert = painted_ink(&output, "hello ");
    let description = painted_ink(&output, "a greeting");
    let row = suggestion_row(&output, insert, description);
    (insert, description, row)
}

/// In a right-to-left popup the insert text hugs the reading-start (right)
/// edge and the weak description sits past the growing spacer at the reading
/// end, entirely left of the insert.
#[test]
fn rtl_popup_orders_suggestion_by_reading_order() {
    let (insert, description, row) = popup_geometry(true);
    assert!(
        description.right() < insert.left() + 1.0,
        "the description must sit entirely left of the insert in a right-to-left popup \
         (description {description:?}, insert {insert:?})"
    );
    assert!(
        (row.right() - insert.right()) <= EDGE_TOL,
        "the insert must hug the row's reading-start (right) edge \
         (insert {insert:?}, row {row:?})"
    );
}

/// The left-to-right twin: insert at the left edge, description at the
/// reading end on the right.
#[test]
fn ltr_popup_orders_suggestion_by_reading_order() {
    let (insert, description, row) = popup_geometry(false);
    assert!(
        insert.right() < description.left() + 1.0,
        "the description must sit entirely right of the insert in a left-to-right popup \
         (description {description:?}, insert {insert:?})"
    );
    assert!(
        (insert.left() - row.left()) <= EDGE_TOL,
        "the insert must hug the row's reading-start (left) edge \
         (insert {insert:?}, row {row:?})"
    );
}
