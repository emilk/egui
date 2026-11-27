use egui::{Color32, CornerRadius, Margin, Stroke, vec2};
use egui_extras::{Column, TableBuilder};
use egui_kittest::Harness;

/// A table that uses every field of [`egui::style::TableStyle`].
#[test]
fn styled_table() {
    let mut harness = Harness::builder()
        .with_size(vec2(180.0, 160.0))
        .with_pixels_per_point(2.0)
        .build_ui(|ui| {
            let table = &mut ui.visuals_mut().table;
            table.striped = true;
            table.header_fill = Color32::from_rgb(40, 40, 80);
            table.selected_row_fill = Color32::from_rgb(0, 80, 0);
            table.outer_stroke = Stroke::new(1.0, Color32::RED);
            table.corner_radius = CornerRadius::same(4);
            table.header_stroke = Stroke::new(1.0, Color32::YELLOW);
            table.row_stroke = Stroke::new(1.0, Color32::GRAY);
            table.column_stroke = Stroke::new(1.0, Color32::LIGHT_BLUE);
            table.cell_margin = Margin::symmetric(6, 2);
            table_ui(ui);
        });
    harness.snapshot("table_style");
}

fn table_ui(ui: &mut egui::Ui) {
    TableBuilder::new(ui)
        .columns(Column::auto(), 3)
        .vscroll(false)
        .header(20.0, |mut header| {
            for name in ["Name", "Kind", "Size"] {
                header.col(|ui| {
                    ui.strong(name);
                });
            }
        })
        .body(|body| {
            body.rows(20.0, 4, |mut row| {
                let index = row.index();
                row.set_selected(index == 2);
                for text in [
                    format!("row {index}"),
                    "file".to_owned(),
                    "42 kB".to_owned(),
                ] {
                    row.col(|ui| {
                        ui.label(text);
                    });
                }
            });
        });
}

/// The outer cell margins only take up space on the axes where `edge_margin` is on.
#[test]
fn table_edge_margin() {
    let size = |edge_margin: egui::Vec2b| {
        let ctx = egui::Context::default();
        let mut size = egui::Vec2::ZERO;
        let output = ctx.run_ui(Default::default(), |ui| {
            let top_left = ui.cursor().min;
            TableBuilder::new(ui)
                .edge_margin(edge_margin)
                .column(Column::exact(100.0))
                .vscroll(false)
                .body(|mut body| {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label("x");
                        });
                    });
                });
            size = ui.min_rect().max - top_left;
        });
        output.drop_without_applying_deltas();
        size
    };

    let margin = egui::style::TableStyle::default().cell_margin.sum();
    let cells = vec2(100.0, 20.0);
    assert_eq!(size(egui::Vec2b::TRUE), cells + margin);
    assert_eq!(size(egui::Vec2b::FALSE), cells);
    assert_eq!(
        size(egui::Vec2b::new(false, true)),
        cells + vec2(0.0, margin.y)
    );
}

/// The width of the lines between cells takes up space.
#[test]
fn line_width_takes_up_space() {
    let size = |line_width: f32| {
        let ctx = egui::Context::default();
        let mut size = egui::Vec2::ZERO;
        let output = ctx.run_ui(Default::default(), |ui| {
            let table = &mut ui.visuals_mut().table;
            table.header_stroke = Stroke::new(line_width, Color32::RED);
            table.row_stroke = Stroke::new(line_width, Color32::RED);
            table.column_stroke = Stroke::new(line_width, Color32::RED);
            let top_left = ui.cursor().min;
            TableBuilder::new(ui)
                .columns(Column::exact(100.0), 2)
                .vscroll(false)
                .header(20.0, |mut header| {
                    header.col(|_| {});
                    header.col(|_| {});
                })
                .body(|body| {
                    body.rows(20.0, 3, |mut row| {
                        row.col(|_| {});
                        row.col(|_| {});
                    });
                });
            size = ui.min_rect().max - top_left;
        });
        output.drop_without_applying_deltas();
        size
    };

    // One column line, one header line, and two row lines:
    assert_eq!(size(3.0) - size(0.0), vec2(3.0, 9.0));
}
