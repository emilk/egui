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
            table.header_stroke = Stroke::new(1.0, Color32::YELLOW);
            table.row_stroke = Stroke::new(1.0, Color32::GRAY);
            table.column_stroke = Stroke::new(1.0, Color32::LIGHT_BLUE);
            table.corner_radius = CornerRadius::same(4);
            table.cell_margin = Margin::symmetric(6, 2);

            // Room for the outer stroke, which is outside the table:
            egui::Frame::new().inner_margin(8).show(ui, table_ui);
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
