//! Snapshot tests for [`egui_extras::Table`].
//!
//! The goal is to pin down the layout and the painting of the table:
//! column sizing, row heights, stripes, selection, hover, clipping, and virtualization.
//! When you change `table.rs`, `layout.rs` or `sizing.rs`, these snapshots tell you
//! exactly what moved.

use egui::{Align, Layout, Rect, Theme, Ui, Vec2};
use egui_extras::{Column, TableBuilder};
use egui_kittest::{Harness, SnapshotResults};

/// A long text that wraps or gets clipped, depending on the column.
const LONG_TEXT: &str = "The quick brown fox jumps over the lazy dog";

const THEMES: [Theme; 2] = [Theme::Dark, Theme::Light];

fn harness<'a>(
    size: [f32; 2],
    theme: Theme,
    add_contents: impl FnMut(&mut Ui) + 'a,
) -> Harness<'a> {
    let mut harness = Harness::builder()
        .with_size(Vec2::from(size))
        .with_theme(theme)
        .build_ui(add_contents);
    harness.run();
    harness
}

fn theme_name(theme: Theme) -> &'static str {
    match theme {
        Theme::Dark => "dark",
        Theme::Light => "light",
    }
}

/// Snapshot the same ui in both themes, as `table/{name}_{theme}`.
fn snapshot_both_themes(name: &str, size: [f32; 2], add_contents: fn(&mut Ui)) {
    let mut results = SnapshotResults::new();
    for theme in THEMES {
        let mut harness = harness(size, theme, add_contents);
        results.add(harness.try_snapshot(format!("table/{name}_{}", theme_name(theme))));
    }
}

/// Snapshot the ui in the dark theme only, as `table/{name}`.
fn snapshot(name: &str, size: [f32; 2], add_contents: impl FnMut(&mut Ui)) {
    let mut harness = harness(size, Theme::Dark, add_contents);
    harness.snapshot(format!("table/{name}"));
}

/// A cell that paints its own background, so that the column widths
/// are visible in the snapshot.
fn filled_cell(ui: &mut Ui, text: &str) {
    ui.painter().rect_filled(
        ui.max_rect(),
        egui::CornerRadius::ZERO,
        ui.visuals().widgets.inactive.bg_fill,
    );
    ui.label(text);
}

// ----------------------------------------------------------------------------
// Column sizing

/// All the [`Column`] kinds side by side.
///
/// Each cell paints its own background, so a change in the sizing code
/// shows up as a moved column border.
#[test]
fn column_kinds() {
    snapshot("column_kinds", [520.0, 160.0], |ui| {
        TableBuilder::new(ui)
            .resizable(true)
            .column(Column::exact(50.0))
            .column(Column::initial(70.0))
            .column(Column::auto())
            .column(Column::auto().at_least(120.0))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                for name in ["exact", "initial", "auto", "at_least", "remainder"] {
                    header.col(|ui| {
                        filled_cell(ui, name);
                    });
                }
            })
            .body(|mut body| {
                for row_index in 0..3 {
                    body.row(18.0, |mut row| {
                        for col in 0..5 {
                            row.col(|ui| {
                                filled_cell(ui, &format!("r{row_index}c{col}"));
                            });
                        }
                    });
                }
            });
    });
}

/// Two [`Column::remainder`] share the slack equally.
#[test]
fn two_remainders_share_the_slack() {
    snapshot("two_remainders_share_the_slack", [400.0, 80.0], |ui| {
        TableBuilder::new(ui)
            .column(Column::exact(60.0))
            .column(Column::remainder())
            .column(Column::remainder())
            .body(|mut body| {
                body.row(18.0, |mut row| {
                    for name in ["exact", "remainder", "remainder"] {
                        row.col(|ui| {
                            filled_cell(ui, name);
                        });
                    }
                });
            });
    });
}

/// `at_most` caps a column even when the content is wider.
#[test]
fn at_most_caps_the_column() {
    snapshot("at_most_caps_the_column", [400.0, 80.0], |ui| {
        TableBuilder::new(ui)
            .column(Column::auto().at_most(80.0).clip(true))
            .column(Column::remainder())
            .body(|mut body| {
                body.row(18.0, |mut row| {
                    row.col(|ui| {
                        filled_cell(ui, LONG_TEXT);
                    });
                    row.col(|ui| {
                        filled_cell(ui, "after");
                    });
                });
            });
    });
}

/// A `clip` column truncates its text.
#[test]
fn clip_truncates() {
    snapshot("clip_truncates", [260.0, 80.0], |ui| {
        TableBuilder::new(ui)
            .column(Column::remainder().clip(true))
            .column(Column::exact(40.0))
            .body(|mut body| {
                body.row(18.0, |mut row| {
                    row.col(|ui| {
                        ui.label(LONG_TEXT);
                    });
                    row.col(|ui| {
                        ui.label("end");
                    });
                });
            });
    });
}

/// A non-`clip` column grows the table past the available width.
#[test]
fn no_clip_overflows() {
    snapshot("no_clip_overflows", [260.0, 80.0], |ui| {
        TableBuilder::new(ui)
            .column(Column::auto())
            .column(Column::exact(40.0))
            .body(|mut body| {
                body.row(18.0, |mut row| {
                    row.col(|ui| {
                        ui.label(LONG_TEXT);
                    });
                    row.col(|ui| {
                        ui.label("end");
                    });
                });
            });
    });
}

// ----------------------------------------------------------------------------
// Row heights

/// The height given to [`egui_extras::TableBody::row`] is a minimum:
/// a taller cell pushes the following rows down.
///
/// The stripes should cover the whole grown row.
#[test]
fn tall_cell_grows_the_row() {
    snapshot_both_themes("tall_cell_grows_the_row", [300.0, 200.0], |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::exact(60.0))
            .column(Column::remainder())
            .body(|mut body| {
                for row_index in 0..4 {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.label(format!("row {row_index}"));
                        });
                        row.col(|ui| {
                            if row_index == 2 {
                                // Three labels in a column: taller than the 18 px we asked for.
                                ui.vertical(|ui| {
                                    ui.label("tall");
                                    ui.label("tall");
                                    ui.label("tall");
                                });
                            } else {
                                ui.label("short");
                            }
                        });
                    });
                }
            });
    });
}

/// A clipping column keeps its fixed height, so a tall cell overlaps the next row.
#[test]
fn tall_cell_in_clipped_column() {
    snapshot("tall_cell_in_clipped_column", [300.0, 200.0], |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::exact(60.0))
            .column(Column::remainder().clip(true))
            .body(|mut body| {
                for row_index in 0..4 {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.label(format!("row {row_index}"));
                        });
                        row.col(|ui| {
                            if row_index == 2 {
                                ui.vertical(|ui| {
                                    ui.label("tall");
                                    ui.label("tall");
                                    ui.label("tall");
                                });
                            } else {
                                ui.label("short");
                            }
                        });
                    });
                }
            });
    });
}

/// Wrapping text in a narrow column: the row must be as tall as the wrapped text.
#[test]
fn wrapping_cell_grows_the_row() {
    snapshot("wrapping_cell_grows_the_row", [240.0, 200.0], |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::exact(50.0))
            // `exact` implies `clip`, but we want the text to wrap:
            .column(Column::exact(120.0).clip(false))
            .body(|mut body| {
                for row_index in 0..2 {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.label(format!("row {row_index}"));
                        });
                        row.col(|ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                            ui.label(LONG_TEXT);
                        });
                    });
                }
            });
    });
}

// ----------------------------------------------------------------------------
// Row decorations

#[test]
fn striped() {
    snapshot_both_themes("striped", [300.0, 160.0], |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::exact(60.0))
            .column(Column::remainder())
            .body(|mut body| {
                for row_index in 0..6 {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.label(format!("row {row_index}"));
                        });
                        row.col(|ui| {
                            ui.label("cell");
                        });
                    });
                }
            });
    });
}

#[test]
fn selected_and_overline() {
    snapshot_both_themes("selected_and_overline", [300.0, 160.0], |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::exact(60.0))
            .column(Column::remainder())
            .body(|mut body| {
                for row_index in 0..6 {
                    body.row(18.0, |mut row| {
                        row.set_selected(row_index == 2 || row_index == 3);
                        row.set_overline(row_index == 4);
                        row.col(|ui| {
                            ui.label(format!("row {row_index}"));
                        });
                        row.col(|ui| {
                            ui.label("cell");
                        });
                    });
                }
            });
    });
}

/// Hovering a row of a `sense`:ing table paints the whole row.
#[test]
fn hovered_row() {
    let mut results = SnapshotResults::new();
    for theme in THEMES {
        let mut harness = harness([300.0, 160.0], theme, |ui| {
            TableBuilder::new(ui)
                .sense(egui::Sense::click())
                .column(Column::exact(60.0))
                .column(Column::remainder())
                .body(|mut body| {
                    for row_index in 0..6 {
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                ui.label(format!("row {row_index}"));
                            });
                            row.col(|ui| {
                                ui.label("cell");
                            });
                        });
                    }
                });
        });

        // The third row:
        harness.hover_at(egui::pos2(150.0, 8.0 + 2.5 * 21.0));
        harness.run();
        results.add(harness.try_snapshot(format!("table/hovered_row_{}", theme_name(theme))));
    }
}

// ----------------------------------------------------------------------------
// Header, virtualization and scrolling

/// A sticky header stays put while the body scrolls under it.
#[test]
fn header_stays_while_body_scrolls() {
    snapshot("header_stays_while_body_scrolls", [300.0, 140.0], |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .vertical_scroll_offset(100.0)
            .column(Column::exact(60.0))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("index");
                });
                header.col(|ui| {
                    ui.strong("value");
                });
            })
            .body(|body| {
                body.rows(18.0, 100, |mut row| {
                    let row_index = row.index();
                    row.col(|ui| {
                        ui.label(row_index.to_string());
                    });
                    row.col(|ui| {
                        ui.label(format!("value {row_index}"));
                    });
                });
            });
    });
}

/// `scroll_to_row` on a virtualized body.
#[test]
fn scroll_to_row() {
    snapshot("scroll_to_row", [300.0, 140.0], |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .animate_scrolling(false)
            .scroll_to_row(50, Some(Align::Center))
            .column(Column::exact(60.0))
            .column(Column::remainder())
            .body(|body| {
                body.rows(18.0, 100, |mut row| {
                    let row_index = row.index();
                    row.col(|ui| {
                        ui.label(row_index.to_string());
                    });
                    row.col(|ui| {
                        ui.label(format!("value {row_index}"));
                    });
                });
            });
    });
}

#[test]
fn heterogeneous_rows() {
    snapshot("heterogeneous_rows", [300.0, 200.0], |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::exact(60.0))
            .column(Column::remainder())
            .body(|body| {
                body.heterogeneous_rows([18.0, 40.0, 18.0, 60.0, 18.0].into_iter(), |mut row| {
                    let row_index = row.index();
                    row.col(|ui| {
                        ui.label(row_index.to_string());
                    });
                    row.col(|ui| {
                        ui.label("cell");
                    });
                });
            });
    });
}

/// Dragging the separator between two resizable columns moves it.
#[test]
fn resize_column_by_dragging() {
    let mut results = SnapshotResults::new();
    let mut harness = harness([300.0, 100.0], Theme::Dark, |ui| {
        TableBuilder::new(ui)
            .resizable(true)
            .column(Column::initial(100.0))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    filled_cell(ui, "first");
                });
                header.col(|ui| {
                    filled_cell(ui, "second");
                });
            })
            .body(|mut body| {
                body.row(18.0, |mut row| {
                    row.col(|ui| {
                        filled_cell(ui, "a");
                    });
                    row.col(|ui| {
                        filled_cell(ui, "b");
                    });
                });
            });
    });
    results.add(harness.try_snapshot("table/resize_column_before_drag"));

    // The separator sits just right of the first column:
    let separator = egui::pos2(104.0, 20.0);
    harness.drag_at(separator);
    harness.run();
    harness.hover_at(separator + Vec2::new(60.0, 0.0));
    harness.run();
    harness.drop_at(separator + Vec2::new(60.0, 0.0));
    harness.run();

    results.extend_harness(&mut harness);
    results.add(harness.try_snapshot("table/resize_column_after_drag"));
}

// ----------------------------------------------------------------------------
// Misc

/// `cell_layout` controls the alignment inside every cell.
#[test]
fn cell_layouts() {
    snapshot("cell_layouts", [420.0, 160.0], |ui| {
        for (i, layout) in [
            Layout::left_to_right(Align::Center),
            Layout::right_to_left(Align::Center),
            Layout::top_down(Align::Center),
        ]
        .into_iter()
        .enumerate()
        {
            TableBuilder::new(ui)
                .id_salt(i)
                .striped(true)
                .vscroll(false)
                .cell_layout(layout)
                .column(Column::exact(100.0))
                .column(Column::exact(100.0))
                .body(|mut body| {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            ui.label("left");
                        });
                        row.col(|ui| {
                            ui.label("right");
                        });
                    });
                });
        }
    });
}

/// A table with columns but no rows should not paint anything but the resize lines.
#[test]
fn empty_table() {
    snapshot("empty_table", [300.0, 80.0], |ui| {
        TableBuilder::new(ui)
            .resizable(true)
            .column(Column::exact(60.0))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("index");
                });
                header.col(|ui| {
                    ui.strong("value");
                });
            })
            .body(|_| {});
    });
}

// ----------------------------------------------------------------------------
// Geometry assertions
//
// Snapshots show _that_ something moved; these say _where_ it should be.

/// Run a table and collect the rect of every cell, in the order they were added.
fn cell_rects(add_table: impl FnMut(&mut Ui, &mut Vec<Rect>)) -> Vec<Rect> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 200.0))
        .build_ui_state(add_table, Vec::new());
    harness.run();
    harness.into_state()
}

/// The height passed to `TableBody::row` is a minimum:
/// a taller cell pushes the following row down past it.
#[test]
fn tall_cell_pushes_the_next_row_down() {
    let rects = cell_rects(|ui, rects| {
        rects.clear();
        TableBuilder::new(ui)
            .column(Column::exact(60.0))
            .column(Column::remainder())
            .body(|mut body| {
                body.row(18.0, |mut row| {
                    rects.push(row.col(|ui| _ = ui.allocate_space(Vec2::new(30.0, 5.0))).0);
                    rects.push(row.col(|ui| _ = ui.allocate_space(Vec2::new(30.0, 60.0))).0);
                });
                body.row(18.0, |mut row| {
                    rects.push(row.col(|ui| _ = ui.allocate_space(Vec2::new(30.0, 5.0))).0);
                    rects.push(row.col(|ui| _ = ui.allocate_space(Vec2::new(30.0, 5.0))).0);
                });
            });
    });

    assert_eq!(rects.len(), 4);
    let tall_cell = rects[1];
    let next_row = rects[2];
    assert!(
        tall_cell.bottom() <= next_row.top(),
        "The next row (top {}) should start below the 60 px cell (bottom {})",
        next_row.top(),
        tall_cell.bottom()
    );
}

/// A clipping column keeps the requested height,
/// so a tall cell in it is clipped instead of growing the row.
#[test]
fn tall_cell_in_clipped_column_does_not_grow_the_row() {
    let rects = cell_rects(|ui, rects| {
        rects.clear();
        TableBuilder::new(ui)
            .column(Column::exact(60.0))
            .column(Column::remainder().clip(true))
            .body(|mut body| {
                body.row(18.0, |mut row| {
                    rects.push(row.col(|ui| _ = ui.allocate_space(Vec2::new(30.0, 5.0))).0);
                    rects.push(row.col(|ui| _ = ui.allocate_space(Vec2::new(30.0, 60.0))).0);
                });
                body.row(18.0, |mut row| {
                    rects.push(row.col(|ui| _ = ui.allocate_space(Vec2::new(30.0, 5.0))).0);
                    rects.push(row.col(|ui| _ = ui.allocate_space(Vec2::new(30.0, 5.0))).0);
                });
            });
    });

    assert_eq!(rects.len(), 4);
    let first_row = rects[0];
    let next_row = rects[2];
    assert!(
        next_row.top() - first_row.top() < 30.0,
        "The clipped row should stay 18 px tall, but the next row is {} px below",
        next_row.top() - first_row.top()
    );
}
