//! Regression tests for the measured `min_rect` of a `Ui`: it must hug its content for
//! every vertical cross alignment, while placement stays untouched.
//!
//! A vertical layout with cross-`Align::Center` stretches every widget frame to the
//! available width (`Layout::next_frame_ignore_wrap`); folding that placement-only frame
//! into the reported minimum made centered bodies measure the full allotted width, so
//! containers that size themselves from the child's `min_rect` (`Window`, `Resize`,
//! `allocate_ui_with_layout`, popups, tooltips) over-sized such bodies.
//!
//! No snapshot assertions; exact geometry only. All geometry lands on integers, because
//! the default `CentralPanel` has an 8 pt inner margin and a `top_down(Align::Min)`
//! non-justified content layout, and egui rounds ui rects (`round_ui`).

use egui::{Align, Context, Layout, RawInput, Rect, Sense, Ui, Vec2, Window, pos2, vec2};

/// The screen size of every test frame.
const SCREEN: Vec2 = vec2(640.0, 480.0);

/// The desired size of the measured region: 300 x 100 inside the panel.
const DESIRED: Vec2 = vec2(300.0, 100.0);

/// A plain 50 x 10 widget.
const WIDGET: Vec2 = vec2(50.0, 10.0);

/// Run one ui pass: a default `CentralPanel` (8 pt inner margin) containing a
/// `DESIRED`-sized region laid out with `layout` and filled by `contents`.
/// Returns the region's response rect.
fn measure(layout: Layout, contents: impl FnMut(&mut Ui)) -> Rect {
    measure_with(&Context::default(), layout, contents)
}

/// Like [`measure`], but with the whole-UI direction flag set (before the first pass,
/// per the established rule: never inside a pass).
fn measure_rtl(layout: Layout, contents: impl FnMut(&mut Ui)) -> Rect {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    measure_with(&ctx, layout, contents)
}

fn measure_with(ctx: &Context, layout: Layout, mut contents: impl FnMut(&mut Ui)) -> Rect {
    let mut rect = Rect::NOTHING;
    let output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
            ..Default::default()
        },
        |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                rect = ui
                    .allocate_ui_with_layout(DESIRED, layout, |ui| contents(ui))
                    .response
                    .rect;
            });
        },
    );
    output.drop_without_applying_deltas();
    rect
}

/// A1: a centered body measures its content, not the full desired width, and the widget
/// stays centered in the 300-wide desired rect (which spans 8..308 inside the panel).
#[test]
fn vertical_center_min_rect_is_content_tight() {
    let rect = measure(Layout::top_down(Align::Center), |ui| {
        ui.allocate_exact_size(WIDGET, Sense::hover());
    });
    assert_eq!(
        rect.width(),
        50.0,
        "a centered body must measure its content, not the full desired width"
    );
    assert!(
        (rect.center().x - 158.0).abs() < 0.5,
        "the widget must stay centered in the desired rect, got {rect:?}"
    );
}

/// A2: the same plain widget measures the same width under `Min`, `Center` and `Max`,
/// with placement mirrored and unchanged.
#[test]
fn vertical_cross_align_measurement_parity() {
    let child = |ui: &mut Ui| {
        ui.allocate_exact_size(WIDGET, Sense::hover());
    };
    let min = measure(Layout::top_down(Align::Min), child);
    let center = measure(Layout::top_down(Align::Center), child);
    let max = measure(Layout::top_down(Align::Max), child);

    assert_eq!(min.width(), 50.0);
    assert_eq!(center.width(), 50.0, "Center must measure like Min");
    assert_eq!(max.width(), 50.0, "Max must measure like Min");

    // Mirrored placement inside the desired rect (panel origin 8.0, end edge 308.0):
    assert_eq!(min.left(), 8.0);
    assert_eq!(max.right(), 308.0);
    assert!(
        (center.center().x - 158.0).abs() < 0.5,
        "the Center rect must be centered in the desired rect, got {center:?}"
    );
}

/// A3: a nested centered body no longer leaks its full-width stretched frame into the
/// parent: a `Min` body and a `Max` body each measure the mirrored 175 pt content span.
#[test]
fn vertical_max_and_min_mirror_with_centered_child() {
    let child = |ui: &mut Ui| {
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            ui.allocate_exact_size(WIDGET, Sense::hover());
        });
    };
    let min = measure(Layout::top_down(Align::Min), child);
    let max = measure(Layout::top_down(Align::Max), child);

    assert_eq!(min.width(), 175.0);
    assert_eq!(max.width(), 175.0);
    // Mirrored geometry inside the desired rect (origin 8.0, right edge 308.0):
    assert_eq!(min.min, pos2(8.0, 8.0));
    assert_eq!(max.max, pos2(308.0, 18.0));
}

/// A4 golden: a plain `Align::Min` body keeps its exact pre-change geometry.
#[test]
fn min_layout_measurement_unchanged() {
    let rect = measure(Layout::top_down(Align::Min), |ui| {
        ui.allocate_exact_size(WIDGET, Sense::hover());
    });
    assert_eq!(rect, Rect::from_min_max(pos2(8.0, 8.0), pos2(58.0, 18.0)));
}

/// A5 gate guard: horizontal rows keep folding their frame, whose height is the row
/// height (`interact_size.y`), and a following widget starts one item spacing below it.
#[test]
fn horizontal_row_measurement_unchanged() {
    let ctx = Context::default();
    let mut row = Rect::NOTHING;
    let mut follower = Rect::NOTHING;
    let mut item_spacing_y = 0.0;
    let output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
            ..Default::default()
        },
        |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                item_spacing_y = ui.spacing().item_spacing.y;
                row = ui
                    .horizontal(|ui| {
                        ui.allocate_exact_size(vec2(50.0, 15.0), Sense::hover());
                    })
                    .response
                    .rect;
                follower = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover()).0;
            });
        },
    );
    output.drop_without_applying_deltas();

    assert_eq!(
        row.height(),
        20.0,
        "the row height is frame-derived (interact_size.y) and must not shrink"
    );
    assert_eq!(
        row.width(),
        50.0,
        "a non-justified row measures its content width"
    );
    assert_eq!(
        follower.top(),
        row.bottom() + item_spacing_y,
        "the following widget must start one item spacing below the row"
    );
}

/// A6: the same centered measurement holds under a right-to-left UI — the fold must not
/// key on the direction flag. The widget stays centered in the desired region, whose
/// anchor mirrors with the interface direction (the region right-hugs under RTL, so its
/// center is the LTR center mirrored about the screen center).
#[test]
fn rtl_measurement_is_direction_independent() {
    let mut is_rtl = false;
    let rect = measure_rtl(Layout::top_down(Align::Center), |ui| {
        is_rtl = ui.is_rtl();
        ui.allocate_exact_size(WIDGET, Sense::hover());
    });
    assert!(is_rtl, "the direction flag must have taken effect");
    assert_eq!(rect.width(), 50.0);
    assert!(
        (rect.center().x - (SCREEN.x - 158.0)).abs() < 0.5,
        "the widget must stay centered in the mirrored desired region, got {rect:?}"
    );
}

/// A7 guard: the grid arm of the measurement fold — a grid body keeps folding its
/// cell-derived frame, so a widget narrower (and shorter) than its cell must not make
/// the grid cell report a content-tight `min_rect` (the `else frame_rect` arm of
/// `Placer::advance_after_rects`; grids are the one non-horizontal layout that keeps
/// folding, because there the frame is the cell size, not a placement-only stretch).
#[test]
fn grid_cell_min_rect_keeps_the_cell_frame() {
    let ctx = Context::default();
    let mut cell = Rect::NOTHING;
    let mut min_col_width = 0.0;
    let mut min_row_height = 0.0;

    // Two frames: the first (discarded, invisible sizing) pass seeds the grid's
    // column/row state, and only afterwards are cells sized from it.
    for _ in 0..2 {
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::Grid::new("probe").show(ui, |ui| {
                        min_col_width = ui.spacing().interact_size.x;
                        min_row_height = ui.spacing().interact_size.y;
                        ui.allocate_exact_size(vec2(10.0, 5.0), Sense::hover());
                        cell = ui.min_rect();
                    });
                });
            },
        );
        output.drop_without_applying_deltas();
    }

    assert_eq!(
        cell.width(),
        min_col_width,
        "a grid cell must keep the cell-derived (frame-folded) width, not its content width"
    );
    assert_eq!(
        cell.height(),
        min_row_height,
        "a grid cell must keep the cell-derived (frame-folded) height, not its content height"
    );
}

/// Task-302 probe (directive-native-rtl): a vertical autosized body (desired
/// smaller than offered) containing a flag-mirrored `ui.horizontal` row with
/// plain widgets, inside a [`Window`] end-to-end: the body — the direct
/// `min_rect`/response-rect consumer the report is about — must hug its
/// content in every frame (sizing pass and steady frames).
///
/// Characterization probe for the reported mirrored-row measurement inflation
/// (reported as steady-frame full-width bodies): expected GREEN at baseline.
/// The window itself is only the end-to-end container here: egui windows keep
/// their 340 pt `default_size` width regardless of content, direction-neutrally
/// (verified against plain LTR/RTL windows), so only the body measurement is
/// asserted. If it ever goes red, a mirrored horizontal row has started
/// leaking the offered width into the measured span.
#[test]
fn probe_window_with_mirrored_row_hugs_content_in_all_frames() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let mut body_widths: Vec<f32> = Vec::new();

    for _ in 0..4 {
        let mut body_width = 0.0;
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    Window::new("probe302").show(ui, |ui| {
                        let body = ui
                            .allocate_ui_with_layout(
                                DESIRED,
                                Layout::top_down(Align::Min),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        ui.allocate_exact_size(WIDGET, Sense::hover());
                                        ui.allocate_exact_size(WIDGET, Sense::hover());
                                    });
                                },
                            )
                            .response;
                        body_width = body.rect.width();
                    });
                });
            },
        );
        output.drop_without_applying_deltas();
        body_widths.push(body_width);
    }

    // The content is one row of two `WIDGET` widgets (50 + 8 + 50 = 108 wide);
    // the body must hug it in every frame — not the 300 pt desired width and
    // certainly not the full offered width.
    for (frame, &body_width) in body_widths.iter().enumerate() {
        assert!(
            (body_width - 108.0).abs() <= 0.5,
            "frame {frame}: the autosized body must hug the mirrored row \
             (108 pt), got {body_width}"
        );
    }
}
