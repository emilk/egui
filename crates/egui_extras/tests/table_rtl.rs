//! Regression tests for `egui_extras` tables and strips under the reading
//! direction ([`egui::Style::right_to_left`]): `StripLayout` places cells with
//! explicit cursor arithmetic rather than through the `Placer`, so declared
//! columns must be laid out from the reading-start edge (the right under RTL),
//! consecutive lines must restart at that edge, and dragging a column
//! separator must grow the column in the reading direction (leftward under
//! RTL).
//!
//! Every mirrored case also asserts its LTR counterpart, so this file doubles
//! as the LTR-unchanged regression. No snapshot assertions; geometry only.

use egui::{
    Context, Event, FullOutput, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2, pos2, vec2,
};
use egui_extras::{Column, Size, StripBuilder, TableBuilder};

/// All tests run on this screen.
const SCREEN: Vec2 = vec2(600.0, 400.0);

/// The width of the exact test columns.
const COLUMN_WIDTH: f32 = 100.0;

/// The default `item_spacing.x`.
const SPACING_X: f32 = 8.0;

/// Tolerance for pixel-rounded geometry.
const TOLERANCE: f32 = 1.0;

fn input_at(size: Vec2, time: f64, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(time),
        events,
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut egui::Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
    output
}

/// A context whose direction flag matches `rtl`, for tests driven purely by the flag.
fn context(rtl: bool) -> Context {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    ctx
}

fn button_event(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

// ---------------------------------------------------------------------------

/// The cell rects of the first two rows of a three-column table
/// (two exact columns followed by a remainder column), captured on a
/// settled frame.
fn table_cell_rects(rtl: bool) -> [Vec<Rect>; 2] {
    let ctx = context(rtl);
    let mut rows: [Vec<Rect>; 2] = [Vec::new(), Vec::new()];
    for frame in 0..2 {
        let mut captured: [Vec<Rect>; 2] = [Vec::new(), Vec::new()];
        let _ = run(&ctx, input_at(SCREEN, 0.0, vec![]), |ui| {
            TableBuilder::new(ui)
                .vscroll(false)
                .column(Column::exact(COLUMN_WIDTH))
                .column(Column::exact(COLUMN_WIDTH))
                .column(Column::remainder())
                .body(|mut body| {
                    for cells in &mut captured {
                        body.row(20.0, |mut row| {
                            row.col(|ui| cells.push(ui.max_rect()));
                            row.col(|ui| cells.push(ui.max_rect()));
                            row.col(|ui| cells.push(ui.max_rect()));
                        });
                    }
                });
        });
        if frame == 1 {
            rows = captured;
        }
    }
    rows
}

/// M1: declared columns are laid out from the reading-start edge: the first
/// declared column hugs the LEFT edge under LTR and the RIGHT edge under RTL,
/// the following columns advance toward the reading end, and the remainder
/// column fills what is left. Consecutive rows restart at the same edge.
#[test]
fn table_columns_follow_the_reading_direction() {
    for rtl in [false, true] {
        let rows = table_cell_rects(rtl);
        let (first, second) = (&rows[0], &rows[1]);
        let advance = COLUMN_WIDTH + SPACING_X;

        if rtl {
            assert!(
                (SCREEN.x - first[0].right()) <= TOLERANCE,
                "RTL: the first declared column should hug the right edge, got {first:?}"
            );
            assert!(
                (first[1].right() - (SCREEN.x - advance)) <= TOLERANCE,
                "RTL: the second column should advance leftward, got {first:?}"
            );
            assert!(
                first[2].left() <= TOLERANCE,
                "RTL: the remainder column should fill toward the left edge, got {first:?}"
            );
            assert!(
                (first[2].right() - (SCREEN.x - 2.0 * advance)) <= TOLERANCE,
                "RTL: the remainder column should start left of the exact columns, got {first:?}"
            );
            assert!(
                first[0].left() > first[1].left(),
                "RTL: declared column order should be mirrored, got {first:?}"
            );
        } else {
            assert!(
                first[0].left() <= TOLERANCE,
                "LTR: the first declared column should hug the left edge, got {first:?}"
            );
            assert!(
                (first[1].left() - advance) <= TOLERANCE,
                "LTR: the second column should advance rightward, got {first:?}"
            );
            assert!(
                (SCREEN.x - first[2].right()) <= TOLERANCE,
                "LTR: the remainder column should fill toward the right edge, got {first:?}"
            );
            assert!(
                (first[2].left() - 2.0 * advance) <= TOLERANCE,
                "LTR: the remainder column should start right of the exact columns, got {first:?}"
            );
            assert!(
                first[0].left() < first[1].left(),
                "LTR: declared column order should be preserved, got {first:?}"
            );
        }

        // A new line restarts at the same reading-start edge:
        for (i, (a, b)) in first.iter().zip(second.iter()).enumerate() {
            assert!(
                (a.left() - b.left()).abs() <= TOLERANCE
                    && (a.right() - b.right()).abs() <= TOLERANCE,
                "row 2 column {i} should reuse row 1's x-range in {} (row 1: {a:?}, row 2: {b:?})",
                if rtl { "RTL" } else { "LTR" }
            );
        }
    }
}

/// M2: a horizontal strip mirrors its cells the same way (it shares
/// `StripLayout` with tables).
#[test]
fn horizontal_strip_cells_follow_the_reading_direction() {
    for rtl in [false, true] {
        let ctx = context(rtl);
        let mut cells = Vec::new();
        for _ in 0..2 {
            cells.clear();
            let _ = run(&ctx, input_at(SCREEN, 0.0, vec![]), |ui| {
                StripBuilder::new(ui)
                    .size(Size::exact(COLUMN_WIDTH))
                    .size(Size::exact(COLUMN_WIDTH))
                    .horizontal(|mut strip| {
                        strip.cell(|ui| cells.push(ui.max_rect()));
                        strip.cell(|ui| cells.push(ui.max_rect()));
                    });
            });
        }

        if rtl {
            assert!(
                (SCREEN.x - cells[0].right()) <= TOLERANCE,
                "RTL: the first strip cell should hug the right edge, got {cells:?}"
            );
            assert!(
                (cells[1].right() - (SCREEN.x - COLUMN_WIDTH - SPACING_X)) <= TOLERANCE,
                "RTL: the second strip cell should advance leftward, got {cells:?}"
            );
        } else {
            assert!(
                cells[0].left() <= TOLERANCE,
                "LTR: the first strip cell should hug the left edge, got {cells:?}"
            );
            assert!(
                (cells[1].left() - (COLUMN_WIDTH + SPACING_X)) <= TOLERANCE,
                "LTR: the second strip cell should advance rightward, got {cells:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------

/// Drag the separator after the first column of a two-column table
/// (exact + resizable, then remainder) by 40 points in the direction that
/// should widen the first column, and return its settled rect.
fn dragged_first_column_rect(rtl: bool) -> Rect {
    let ctx = context(rtl);
    let mut rect = Rect::NOTHING;
    let mut time = 0.0;

    let mut frame = |events: Vec<Event>, out: &mut Rect, capture: bool| {
        time += 0.1;
        let _ = run(&ctx, input_at(SCREEN, time, events), |ui| {
            TableBuilder::new(ui)
                .vscroll(false)
                // `Column::exact` pins its width range, so use a resizable
                // ranged column instead:
                .column(Column::initial(COLUMN_WIDTH).resizable(true))
                .column(Column::remainder())
                .body(|mut body| {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            if capture {
                                *out = ui.max_rect();
                            }
                        });
                        row.col(|_ui| {});
                    });
                });
        });
    };

    // Two idle frames so the table and its state settle, capturing the
    // pre-drag geometry:
    frame(vec![], &mut rect, true);
    frame(vec![], &mut rect, true);

    // The separator sits in the middle of the gap after the first column,
    // which is the column's reading-end side:
    let separator_x = if rtl {
        rect.left() - 0.5 * SPACING_X
    } else {
        rect.right() + 0.5 * SPACING_X
    };
    let y = rect.center().y;

    // Drag 40 points in the widening direction: rightward under LTR,
    // leftward under RTL.
    let delta = if rtl { -40.0 } else { 40.0 };
    let drag_to = pos2(separator_x + delta, y);

    frame(
        vec![Event::PointerMoved(pos2(separator_x, y))],
        &mut rect,
        false,
    );
    frame(
        vec![button_event(
            pos2(separator_x, y),
            PointerButton::Primary,
            true,
        )],
        &mut rect,
        false,
    );
    for _ in 0..2 {
        frame(vec![Event::PointerMoved(drag_to)], &mut rect, false);
    }
    frame(
        vec![button_event(drag_to, PointerButton::Primary, false)],
        &mut rect,
        false,
    );

    // Settle frames so the new width is laid out, and capture:
    frame(vec![], &mut rect, false);
    frame(vec![], &mut rect, true);
    rect
}

/// M3: dragging a column separator grows the column in the reading
/// direction: rightward under LTR, leftward under RTL.
#[test]
fn column_drag_follows_the_reading_direction() {
    let ltr = dragged_first_column_rect(false);
    assert!(
        (ltr.width() - (COLUMN_WIDTH + 40.0)).abs() <= TOLERANCE,
        "LTR: dragging the separator rightward should widen the first column to {}, got {ltr:?}",
        COLUMN_WIDTH + 40.0
    );
    assert!(
        ltr.left() <= TOLERANCE,
        "LTR: the widened first column should keep hugging the left edge, got {ltr:?}"
    );

    let rtl = dragged_first_column_rect(true);
    assert!(
        (rtl.width() - (COLUMN_WIDTH + 40.0)).abs() <= TOLERANCE,
        "RTL: dragging the separator leftward should widen the first column to {}, got {rtl:?}",
        COLUMN_WIDTH + 40.0
    );
    assert!(
        (SCREEN.x - rtl.right()) <= TOLERANCE,
        "RTL: the widened first column should keep hugging the right edge, got {rtl:?}"
    );
}
