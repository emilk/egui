//! Regression tests for the `region_from_max_rect` alignment seed: the zero-size
//! placeholder that seeds every new region's `min_rect` at the next widget position —
//! for a vertical `Align::Center` body, the center of the offered rect.
//!
//! The seed is a placeholder while nothing has been measured on an axis: it gives the
//! empty body a finite, layout-anchored form, but it is not content. The first measured
//! rect on an axis *replaces* the seed's contribution on that axis instead of unioning
//! with it. While the seed unioned, a centered body whose content hugged one edge (a
//! nested row, a grid, an advanced cursor) measured from the content to the offered
//! center — exactly half the offered width, in both directions — and autosized
//! containers (`Window`, `Resize`) jumped between the tight sizing-pass size and the
//! seed-widened steady size.
//!
//! Every seed-widening case asserts both reading directions with mirrored spans and in
//! two frames (the sizing pass and a steady frame must agree). The unchanged-behavior
//! cases cover the consumers that rely on the seed: the empty-body anchor, the width
//! setters, cursor-advance spans.
//!
//! No snapshot assertions; exact geometry only. All geometry lands on integers, because
//! the default `CentralPanel` has an 8 pt inner margin and a `top_down(Align::Min)`
//! non-justified content layout, and egui rounds ui rects (`round_ui`).

use egui::{Align, Context, Layout, RawInput, Rect, Sense, Ui, Vec2, Window, pos2, vec2};

/// The screen size of every test frame.
const SCREEN: Vec2 = vec2(640.0, 480.0);

/// A plain 108 x 20 widget: also the width every centered body below must measure. The
/// panel inner rect spans 8..632 = 624 wide, and a seed-unioned centered body measures
/// exactly half of it (312) — the bug class these tests guard against.
const WIDGET: Vec2 = vec2(108.0, 20.0);

/// Tolerance for every coordinate comparison (egui rounds ui rects).
const TOL: f32 = 0.5;

fn ltr_ctx() -> Context {
    Context::default()
}

fn rtl_ctx() -> Context {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    ctx
}

/// Run `frames` ui passes, each a default `CentralPanel` (8 pt inner margin) filled by
/// `contents`.
fn run_passes(ctx: &Context, frames: usize, mut contents: impl FnMut(&mut Ui)) {
    for _ in 0..frames {
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| contents(ui));
            },
        );
        output.drop_without_applying_deltas();
    }
}

/// The mirrored row rect inside the panel: the row hugs the reading edge.
fn expected_row_rect(ltr: bool) -> Rect {
    if ltr {
        Rect::from_min_max(pos2(8.0, 8.0), pos2(116.0, 28.0))
    } else {
        Rect::from_min_max(pos2(524.0, 8.0), pos2(632.0, 28.0))
    }
}

/// Assert rect equality within [`TOL`], naming the direction in the failure message.
fn assert_rect(ltr: bool, what: &str, actual: Rect, expected: Rect) {
    assert!(
        (actual.min.x - expected.min.x).abs() <= TOL
            && (actual.min.y - expected.min.y).abs() <= TOL
            && (actual.max.x - expected.max.x).abs() <= TOL
            && (actual.max.y - expected.max.y).abs() <= TOL,
        "{}: {what} must be {expected:?}, got {actual:?}",
        if ltr { "LTR" } else { "RTL" }
    );
}

/// A plain widget is centered in the 624-wide panel, so its span `[266..374]` contains
/// the seed point 320: the centered body must keep measuring it (the reported
/// `min_rect` stays the content bounding box).
#[test]
fn plain_widget_in_centered_body_unchanged() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut body = Rect::NOTHING;
        run_passes(&ctx, 2, |ui| {
            body = ui
                .vertical_centered(|ui| {
                    ui.allocate_exact_size(WIDGET, Sense::hover());
                })
                .response
                .rect;
        });
        assert_rect(
            ltr,
            "the centered body must measure the centered widget",
            body,
            Rect::from_min_max(pos2(266.0, 8.0), pos2(374.0, 28.0)),
        );
    }
}

/// An empty centered body keeps reporting the zero-size seed point at the offered
/// center `(320, 8)` — the finite, layout-anchored form empty-body readers rely on.
#[test]
fn empty_centered_body_is_the_seed_point() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut body = Rect::NOTHING;
        run_passes(&ctx, 2, |ui| {
            body = ui.vertical_centered(|_ui| {}).response.rect;
        });
        assert_rect(
            ltr,
            "the empty centered body must stay the zero-size seed point",
            body,
            Rect::from_min_size(pos2(320.0, 8.0), Vec2::ZERO),
        );
    }
}

/// Two centered children measure their bounding box (both children are centered, so
/// the seed stays inside their span).
#[test]
fn two_centered_children_measure_their_bbox() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut body = Rect::NOTHING;
        run_passes(&ctx, 2, |ui| {
            body = ui
                .vertical_centered(|ui| {
                    ui.allocate_exact_size(WIDGET, Sense::hover());
                    ui.allocate_exact_size(WIDGET, Sense::hover());
                })
                .response
                .rect;
        });
        // Item spacing.y = 3: the second widget starts at 28 + 3 = 31.
        assert_rect(
            ltr,
            "two centered children must measure their bounding box",
            body,
            Rect::from_min_max(pos2(266.0, 8.0), pos2(374.0, 51.0)),
        );
    }
}

/// A label measures its galley width — identical in a `Min` body and a centered body,
/// in both directions (the seed sits inside the centered label's span).
#[test]
fn label_measures_its_galley_width_in_centered_bodies() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut centered = Rect::NOTHING;
        let mut plain = Rect::NOTHING;
        run_passes(&ctx, 2, |ui| {
            centered = ui
                .vertical_centered(|ui| {
                    ui.label("horizontal centering seed");
                })
                .response
                .rect;
            plain = ui
                .vertical(|ui| {
                    ui.label("horizontal centering seed");
                })
                .response
                .rect;
        });
        assert!(
            (centered.width() - plain.width()).abs() <= TOL,
            "{}: the centered label must measure its galley width like the plain one \
             (centered {centered:?}, plain {plain:?})",
            if ltr { "LTR" } else { "RTL" }
        );
        assert!(
            centered.width() > 0.0,
            "{}: the label must measure its galley width, got {centered:?}",
            if ltr { "LTR" } else { "RTL" }
        );
    }
}

/// A `Min` body with a nested row measures the row in both directions (the
/// reading-edge seed sits inside the row span there).
#[test]
fn min_body_with_row_measures_the_row() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut body = Rect::NOTHING;
        run_passes(&ctx, 2, |ui| {
            body = ui
                .vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.allocate_exact_size(WIDGET, Sense::hover());
                    });
                })
                .response
                .rect;
        });
        assert_rect(
            ltr,
            "the Min body must measure the row",
            body,
            expected_row_rect(ltr),
        );
    }
}

/// A bare row measures its content in both directions.
#[test]
fn row_alone_measures_its_content() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut row = Rect::NOTHING;
        run_passes(&ctx, 2, |ui| {
            row = ui
                .horizontal(|ui| {
                    ui.allocate_exact_size(WIDGET, Sense::hover());
                })
                .response
                .rect;
        });
        assert_rect(
            ltr,
            "the row must measure its content",
            row,
            expected_row_rect(ltr),
        );
    }
}

/// The mirrored horizontal constructors measure tight in both directions (their seeds
/// sit at the reading-start edge where their content starts).
#[test]
fn mirrored_horizontal_constructors_measure_tight() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut centered = Rect::NOTHING;
        let mut top = Rect::NOTHING;
        run_passes(&ctx, 2, |ui| {
            centered = ui
                .horizontal_centered(|ui| {
                    ui.allocate_exact_size(WIDGET, Sense::hover());
                })
                .response
                .rect;
            top = ui
                .horizontal_top(|ui| {
                    ui.allocate_exact_size(WIDGET, Sense::hover());
                })
                .response
                .rect;
        });
        assert!(
            (centered.width() - 108.0).abs() <= TOL,
            "{}: horizontal_centered must measure its content width, got {centered:?}",
            if ltr { "LTR" } else { "RTL" }
        );
        assert!(
            (top.width() - 108.0).abs() <= TOL,
            "{}: horizontal_top must measure its content width, got {top:?}",
            if ltr { "LTR" } else { "RTL" }
        );
    }
}

/// The width setters: `set_min_width(200)` claims its span with the layout-derived
/// anchor, so a row after it measures the union of row and claim — 412, in both
/// directions. The claim always contains the seed point on its axis, so the seed
/// semantics must not change it.
#[test]
fn set_min_width_then_row_keeps_the_claim_contribution() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut body = Rect::NOTHING;
        run_passes(&ctx, 2, |ui| {
            body = ui
                .vertical_centered(|ui| {
                    ui.set_min_width(200.0);
                    ui.horizontal(|ui| {
                        ui.allocate_exact_size(WIDGET, Sense::hover());
                    });
                })
                .response
                .rect;
        });
        // Claim `[220..420]` (centered in the panel) unioned with the row
        // `[8..116]` LTR / `[524..632]` RTL.
        let expected = if ltr {
            Rect::from_min_max(pos2(8.0, 8.0), pos2(420.0, 28.0))
        } else {
            Rect::from_min_max(pos2(220.0, 8.0), pos2(632.0, 28.0))
        };
        assert_rect(
            ltr,
            "the body must keep the set_min_width claim and the row",
            body,
            expected,
        );
    }
}

/// A `vertical_centered` body containing a nested `ui.horizontal` row must measure
/// the row (108), not the union of the row with the offered-rect center seed
/// (312 = half the 624-wide offered rect, both directions, every frame).
#[test]
fn centered_body_with_nested_row_measures_the_row() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut body_rects = [Rect::NOTHING; 2];
        for (frame, body) in body_rects.iter_mut().enumerate() {
            let mut row = Rect::NOTHING;
            let mut body_rect = Rect::NOTHING;
            run_passes(&ctx, 1, |ui| {
                body_rect = ui
                    .vertical_centered(|ui| {
                        row = ui
                            .horizontal(|ui| {
                                ui.allocate_exact_size(WIDGET, Sense::hover());
                            })
                            .response
                            .rect;
                    })
                    .response
                    .rect;
            });
            *body = body_rect;
            assert_rect(
                ltr,
                &format!("frame {frame}: the row"),
                row,
                expected_row_rect(ltr),
            );
        }
        assert_rect(
            ltr,
            "the centered body must measure the row",
            body_rects[0],
            expected_row_rect(ltr),
        );
        assert_rect(
            ltr,
            "the steady frame must match the sizing pass",
            body_rects[1],
            body_rects[0],
        );
    }
}

/// The generic-helper twin — `with_layout(top_down(Align::Center))` with a
/// nested row measures the row, both directions, every frame.
#[test]
fn with_layout_center_with_nested_row_measures_the_row() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut body_rects = [Rect::NOTHING; 2];
        for body_rect in &mut body_rects {
            run_passes(&ctx, 1, |ui| {
                *body_rect = ui
                    .with_layout(Layout::top_down(Align::Center), |ui| {
                        ui.horizontal(|ui| {
                            ui.allocate_exact_size(WIDGET, Sense::hover());
                        });
                    })
                    .response
                    .rect;
            });
        }
        assert_rect(
            ltr,
            "the with_layout body must measure the row",
            body_rects[0],
            expected_row_rect(ltr),
        );
        assert_rect(
            ltr,
            "the steady frame must match the sizing pass",
            body_rects[1],
            body_rects[0],
        );
    }
}

/// `vertical_centered_justified` with a nested row measures the row —
/// cross-justify does not change the seed.
#[test]
fn centered_justified_with_nested_row_measures_the_row() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut body_rects = [Rect::NOTHING; 2];
        for body_rect in &mut body_rects {
            run_passes(&ctx, 1, |ui| {
                *body_rect = ui
                    .vertical_centered_justified(|ui| {
                        ui.horizontal(|ui| {
                            ui.allocate_exact_size(WIDGET, Sense::hover());
                        });
                    })
                    .response
                    .rect;
            });
        }
        assert_rect(
            ltr,
            "the justified centered body must measure the row",
            body_rects[0],
            expected_row_rect(ltr),
        );
        assert_rect(
            ltr,
            "the steady frame must match the sizing pass",
            body_rects[1],
            body_rects[0],
        );
    }
}

/// A nested `allocate_ui_with_layout` body with a cross-`Center` horizontal
/// layout inside `vertical_centered` measures its content — a nested region's own seed
/// would widen its span and the parent's seed would union into it (312, both
/// directions). The explicit `left_to_right` layout is not mirrored, so the expected
/// rect is the same in both directions.
#[test]
fn nested_center_cross_body_in_centered_parent_measures_content() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut body_rects = [Rect::NOTHING; 2];
        for body_rect in &mut body_rects {
            run_passes(&ctx, 1, |ui| {
                *body_rect = ui
                    .vertical_centered(|ui| {
                        ui.allocate_ui_with_layout(
                            vec2(ui.available_width(), 20.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.allocate_exact_size(WIDGET, Sense::hover());
                            },
                        );
                    })
                    .response
                    .rect;
            });
        }
        let expected = Rect::from_min_max(pos2(8.0, 8.0), pos2(116.0, 28.0));
        assert_rect(
            ltr,
            "the centered body must measure the nested body's content",
            body_rects[0],
            expected,
        );
        assert_rect(
            ltr,
            "the steady frame must match the sizing pass",
            body_rects[1],
            body_rects[0],
        );
    }
}

/// `add_space(40)` then a row inside `vertical_centered`: the y-advance must
/// keep its measured contribution (span `[8..68]`, height 60) while the row replaces the
/// x seed (108 wide) — advancing the cursor spends only the main-axis seed.
#[test]
fn add_space_then_row_keeps_the_space_and_drops_the_seed() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut body_rects = [Rect::NOTHING; 2];
        for body_rect in &mut body_rects {
            run_passes(&ctx, 1, |ui| {
                *body_rect = ui
                    .vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.horizontal(|ui| {
                            ui.allocate_exact_size(WIDGET, Sense::hover());
                        });
                    })
                    .response
                    .rect;
            });
        }
        // The row starts one `add_space` below the top: y span 8..68 (the 40-space
        // stays measured); x span is the row's reading-edge span.
        let expected = if ltr {
            Rect::from_min_max(pos2(8.0, 8.0), pos2(116.0, 68.0))
        } else {
            Rect::from_min_max(pos2(524.0, 8.0), pos2(632.0, 68.0))
        };
        assert_rect(
            ltr,
            "the body must keep the 40-space and measure the row",
            body_rects[0],
            expected,
        );
        assert!(
            (body_rects[0].height() - 60.0).abs() <= TOL,
            "{}: the body height must be 60 (the 40-space + the 20-row), got {:?}",
            if ltr { "LTR" } else { "RTL" },
            body_rects[0]
        );
        assert_rect(
            ltr,
            "the steady frame must match the sizing pass",
            body_rects[1],
            body_rects[0],
        );
    }
}

/// A `Grid` with one row of two widgets inside `vertical_centered` measures
/// the grid cells span, not the union of the cells with the offered-rect center seed
/// (312, both directions, while the seed unions).
#[test]
fn grid_in_centered_body_measures_its_cells() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut body_rects = [Rect::NOTHING; 2];
        for body_rect in &mut body_rects {
            run_passes(&ctx, 1, |ui| {
                *body_rect = ui
                    .vertical_centered(|ui| {
                        egui::Grid::new("centered_seed_grid")
                            .show(ui, |ui| {
                                ui.allocate_exact_size(WIDGET, Sense::hover());
                                ui.allocate_exact_size(WIDGET, Sense::hover());
                                ui.end_row();
                            })
                            .response
                            .rect
                    })
                    .response
                    .rect;
            });
        }
        // Two 108-wide cells one item spacing apart, starting at the reading edge:
        // `[8..232]` LTR, mirrored `[408..632]` RTL.
        let expected = if ltr {
            Rect::from_min_max(pos2(8.0, 8.0), pos2(232.0, 28.0))
        } else {
            Rect::from_min_max(pos2(408.0, 8.0), pos2(632.0, 28.0))
        };
        assert_rect(
            ltr,
            "the centered body must measure the grid cells",
            body_rects[0],
            expected,
        );
        assert_rect(
            ltr,
            "the steady frame must match the sizing pass",
            body_rects[1],
            body_rects[0],
        );
    }
}

/// End-to-end `Window`: the `vertical_centered` body inside the window
/// measures 108 in the sizing pass and the steady frames alike (the sizing pass
/// measured tight while the steady frames measured half the window's inner width), and
/// the window's steady outer width equals its sizing-pass width (no first-frame jump),
/// in both directions.
#[test]
fn window_with_centered_body_does_not_jump_between_sizing_and_steady() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        // Every invocation of the window contents (the sizing pass runs the contents
        // before the visible pass) records its body width.
        let body_widths = core::cell::RefCell::new(Vec::new());
        let mut window_widths = Vec::new();
        for _ in 0..3 {
            let mut window_width = 0.0;
            run_passes(&ctx, 1, |ui| {
                let response = Window::new("centered_seed_window").show(ui.ctx(), |ui| {
                    let body_width = ui
                        .vertical_centered(|ui| {
                            ui.horizontal(|ui| {
                                ui.allocate_exact_size(WIDGET, Sense::hover());
                            });
                        })
                        .response
                        .rect
                        .width();
                    body_widths.borrow_mut().push(body_width);
                });
                window_width = response
                    .expect("the window must be open")
                    .response
                    .rect
                    .width();
            });
            window_widths.push(window_width);
        }
        let body_widths = body_widths.into_inner();
        assert!(
            !body_widths.is_empty(),
            "{}: the window body must have been measured",
            if ltr { "LTR" } else { "RTL" }
        );
        for (pass, &width) in body_widths.iter().enumerate() {
            assert!(
                (width - 108.0).abs() <= TOL,
                "{} pass {pass}: the centered body in the window must measure the row \
                 (108), got {width}",
                if ltr { "LTR" } else { "RTL" }
            );
        }
        let (sizing_width, steady_width) =
            (window_widths[0], window_widths[window_widths.len() - 1]);
        assert!(
            (steady_width - sizing_width).abs() <= TOL,
            "{}: the window must not jump between the sizing pass ({sizing_width}) and \
             the steady frames ({steady_width})",
            if ltr { "LTR" } else { "RTL" }
        );
    }
}

/// An empty `vertical_centered` child in a fresh `Min` parent reports a zero-size point
/// at the offered center, and the parent must measure that point instead of the union
/// with its own seed edge (`[8..320]`): the child point is the first measured rect on
/// each axis, so it replaces the parent's seed contribution.
#[test]
fn empty_centered_child_point_replaces_the_parent_seed() {
    for ltr in [true, false] {
        let ctx = if ltr { ltr_ctx() } else { rtl_ctx() };
        let mut parent = Rect::NOTHING;
        run_passes(&ctx, 1, |ui| {
            parent = ui
                .vertical(|ui| {
                    ui.vertical_centered(|_ui| {});
                })
                .response
                .rect;
        });
        assert_rect(
            ltr,
            "the parent must measure the empty child's seed point",
            parent,
            Rect::from_min_size(pos2(320.0, 8.0), Vec2::ZERO),
        );
    }
}
