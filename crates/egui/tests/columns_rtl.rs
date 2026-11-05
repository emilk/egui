//! Regression tests for [`egui::Ui::columns`] under the whole-UI direction flag
//! ([`egui::Style::right_to_left`]): the columns are enumerated from the
//! reading-start edge, so the first column hugs the RIGHT edge of the container
//! and later columns march toward the reading end (right-to-left column order),
//! the same convention [`egui::Grid`] uses for its cells. Under LTR nothing
//! changes: the first column hugs the LEFT edge and columns run left-to-right.
//!
//! Every mirrored case also asserts its LTR counterpart, so this file doubles
//! as the LTR-unchanged regression. No snapshot assertions; column geometry
//! only. Covers both [`egui::Ui::columns`] and [`egui::Ui::columns_const`].

use egui::{Align, CentralPanel, Context, Layout, Pos2, RawInput, Rect, Sense, Ui, Vec2, vec2};

const SCREEN: Vec2 = vec2(600.0, 400.0);
const NUM_COLUMNS: usize = 3;
const TOL: f32 = 0.5;

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

/// One frame: a central panel whose content splits into `NUM_COLUMNS` columns
/// via [`egui::Ui::columns`]. Returns the panel content rect and each column's
/// rect in logical order (column 0 first).
fn columns_rects(rtl: bool) -> (Rect, Vec<Rect>) {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    let mut container = Rect::NOTHING;
    let mut rects: Vec<Rect> = vec![Rect::NOTHING; NUM_COLUMNS];
    run(&ctx, input(SCREEN), |ui| {
        CentralPanel::default().show(ui, |ui| {
            container = ui.max_rect();
            ui.columns(NUM_COLUMNS, |cols| {
                for (i, col) in cols.iter().enumerate() {
                    rects[i] = col.max_rect();
                }
            });
        });
    });
    (container, rects)
}

/// Same as [`columns_rects`], but through [`egui::Ui::columns_const`].
fn columns_const_rects(rtl: bool) -> (Rect, Vec<Rect>) {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    let mut container = Rect::NOTHING;
    let mut rects: Vec<Rect> = vec![Rect::NOTHING; NUM_COLUMNS];
    run(&ctx, input(SCREEN), |ui| {
        CentralPanel::default().show(ui, |ui| {
            container = ui.max_rect();
            ui.columns_const(|cols: &mut [Ui; NUM_COLUMNS]| {
                for (i, col) in cols.iter().enumerate() {
                    rects[i] = col.max_rect();
                }
            });
        });
    });
    (container, rects)
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOL
}

/// The shared column-order expectations, called once per direction with the
/// edge the reading starts from.
fn assert_column_layout(container: Rect, rects: &[Rect], rtl: bool) {
    assert_eq!(rects.len(), NUM_COLUMNS, "one rect per requested column");

    // Every column is as tall as the container and non-degenerate:
    for (i, rect) in rects.iter().enumerate() {
        assert!(
            rect.width() > 0.0
                && near(rect.top(), container.top())
                && near(rect.bottom(), container.bottom()),
            "column {i} spans the container vertically, got {rect:?} in {container:?}"
        );
    }

    // All columns share the same width:
    let width = rects[0].width();
    for (i, rect) in rects.iter().enumerate() {
        assert!(
            near(rect.width(), width),
            "columns share one width, column {i} is {rect:?} vs {width}"
        );
    }

    if rtl {
        // The first column hugs the RIGHT edge of the container:
        assert!(
            near(rects[0].right(), container.right()),
            "rtl: first column hugs the right edge, got {:?} in {container:?}",
            rects[0]
        );
        // Columns march toward the reading end (right-to-left order), with
        // room for the item spacing between them:
        for i in 1..NUM_COLUMNS {
            assert!(
                rects[i].right() < rects[i - 1].left(),
                "rtl: column {i} ({:?}) sits left of column {} ({:?})",
                rects[i],
                i - 1,
                rects[i - 1]
            );
        }
        // The last column's left edge is the reading-end edge of the span
        // (container left, up to spacing tolerance):
        assert!(
            rects[NUM_COLUMNS - 1].left() >= container.left() - TOL,
            "rtl: last column starts at the container's left edge, got {:?} in {container:?}",
            rects[NUM_COLUMNS - 1]
        );
    } else {
        // LTR is unchanged: the first column hugs the LEFT edge and columns
        // run left-to-right:
        assert!(
            near(rects[0].left(), container.left()),
            "ltr: first column hugs the left edge, got {:?} in {container:?}",
            rects[0]
        );
        for i in 1..NUM_COLUMNS {
            assert!(
                rects[i].left() > rects[i - 1].right(),
                "ltr: column {i} ({:?}) sits right of column {} ({:?})",
                rects[i],
                i - 1,
                rects[i - 1]
            );
        }
        assert!(
            rects[NUM_COLUMNS - 1].right() <= container.right() + TOL,
            "ltr: last column ends at the container's right edge, got {:?} in {container:?}",
            rects[NUM_COLUMNS - 1]
        );
    }
}

#[test]
fn columns_follow_reading_direction() {
    let (container_ltr, ltr) = columns_rects(false);
    assert_column_layout(container_ltr, &ltr, false);

    let (container_rtl, rtl) = columns_rects(true);
    assert_column_layout(container_rtl, &rtl, true);
}

#[test]
fn columns_const_follow_reading_direction() {
    let (container_ltr, ltr) = columns_const_rects(false);
    assert_column_layout(container_ltr, &ltr, false);

    let (container_rtl, rtl) = columns_const_rects(true);
    assert_column_layout(container_rtl, &rtl, true);
}

#[test]
fn columns_order_is_mirrored_between_directions() {
    let (_, ltr) = columns_rects(false);
    let (_, rtl) = columns_rects(true);

    // Reading the RTL columns from the right edge yields the same physical
    // layout as reading the LTR columns from the left edge:
    let mut rtl_mirrored = rtl.clone();
    rtl_mirrored.reverse();
    for (i, (l, r)) in ltr.iter().zip(rtl_mirrored.iter()).enumerate() {
        assert!(
            near(l.left(), r.left()) && near(l.right(), r.right()),
            "column {i}: mirrored rtl column ({r:?}) matches ltr column ({l:?})"
        );
    }
}

/// The region for the overflow scenario: 100 pt wide, so `NUM_COLUMNS`
/// columns claiming 60 pt each (plus spacing) cannot fit inside it.
const NARROW_REGION: Vec2 = vec2(100.0, 60.0);

/// The region for the fit-case pin: 200 pt wide, so the default columns fit
/// exactly (available width == total required width).
const FIT_REGION: Vec2 = vec2(200.0, 60.0);

/// A 10 x 10 content widget for the columns' cells.
const CELL: Vec2 = vec2(10.0, 10.0);

/// One pass: a central panel containing a `region`-sized region whose contents
/// split into `NUM_COLUMNS` columns, each optionally claiming 60 pt of width
/// (overflowing the 100 pt region). Returns the region's max rect and the
/// advance rect the columns reported back to the parent (the region's
/// response rect, which the parent's cursor advance folded in).
fn columns_advance_rect(rtl: bool, claim: bool, region_size: Vec2) -> (Rect, Rect) {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    let mut region = Rect::NOTHING;
    let mut response = Rect::NOTHING;
    run(&ctx, input(SCREEN), |ui| {
        CentralPanel::default().show(ui, |ui| {
            let inner = ui
                .allocate_ui_with_layout(region_size, Layout::top_down(Align::Min), |ui| {
                    region = ui.max_rect();
                    ui.columns(NUM_COLUMNS, |cols| {
                        for col in cols.iter_mut() {
                            if claim {
                                col.set_min_width(60.0);
                            }
                            col.allocate_exact_size(CELL, Sense::hover());
                        }
                    });
                })
                .response;
            response = inner.rect;
        });
    });
    (region, response)
}

/// On overflow the columns' advance rect anchors at the reading-start edge:
/// under RTL it hugs the region's right edge and spills past the reading end
/// (left), instead of growing rightward from the region's left edge and
/// pushing the parent's measured span past the reading start.
#[test]
fn columns_overflow_advance_rect_anchors_at_reading_start() {
    let (region, response) = columns_advance_rect(true, true, NARROW_REGION);
    assert!(
        (response.right() - region.right()).abs() <= TOL,
        "rtl overflow: the advance rect must hug the reading-start (right) \
         edge, got {response:?} in {region:?}"
    );
    assert!(
        response.left() < region.left() - TOL,
        "rtl overflow: the advance rect must spill past the reading end \
         (left), got {response:?} in {region:?}"
    );
}

/// LTR twin: unchanged — on overflow the advance rect hugs the region's left
/// edge and spills rightward, exactly as before.
#[test]
fn columns_overflow_advance_rect_ltr_unchanged() {
    let (region, response) = columns_advance_rect(false, true, NARROW_REGION);
    assert!(
        (response.left() - region.left()).abs() <= TOL,
        "ltr overflow: the advance rect must hug the left edge, got \
         {response:?} in {region:?}"
    );
    assert!(
        response.right() > region.right() + TOL,
        "ltr overflow: the advance rect must spill rightward, got \
         {response:?} in {region:?}"
    );
}

/// Fit-case pin: when the columns fit a fresh region (cursor width ==
/// available width == total required width), the reading-start anchor
/// coincides with the old left-anchored rect — only overflow changes.
#[test]
fn columns_fit_advance_rect_identical_in_both_directions() {
    for rtl in [false, true] {
        let (region, response) = columns_advance_rect(rtl, false, FIT_REGION);
        assert!(
            (response.left() - region.left()).abs() <= TOL
                && (response.right() - region.right()).abs() <= TOL,
            "fit (rtl={rtl}): the advance rect must span the region exactly, \
             got {response:?} in {region:?}"
        );
    }
}

/// A `columns` call inside a mirrored (`RightToLeft` main dir) horizontal row
/// keeps a finite measured width: the old left-anchored advance rect started
/// at the row's −∞ cursor and poisoned the row's span with an infinite
/// width in every frame. The reading-start anchor pins it at the row's
/// finite right edge.
#[test]
fn columns_inside_mirrored_horizontal_row_stays_finite() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let mut row = Rect::NOTHING;
    run(&ctx, input(SCREEN), |ui| {
        CentralPanel::default().show(ui, |ui| {
            row = ui
                .horizontal(|row| {
                    row.columns(2, |cols| {
                        for col in cols.iter_mut() {
                            col.allocate_exact_size(CELL, Sense::hover());
                        }
                    });
                })
                .response
                .rect;
        });
    });
    assert!(
        row.width().is_finite(),
        "rtl: a mirrored row containing columns must keep a finite measured \
         width, got {row:?}"
    );
}
