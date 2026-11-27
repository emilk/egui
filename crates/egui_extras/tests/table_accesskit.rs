//! A `Table` in the accessibility tree: a table of rows, with column headers and cells.

use egui::{Align, accesskit::Role};
use egui_extras::{Column, TableBuilder};
use egui_kittest::{
    Harness,
    kittest::{AccessKitNode, NodeT as _, Queryable as _},
};

const COLUMNS: [&str; 3] = ["Name", "CPU", "Memory"];

/// A table with a header and `total_rows` virtualized rows, of which `selected` is selected.
fn process_table(ui: &mut egui::Ui, total_rows: usize, selected: usize, scroll_to: Option<usize>) {
    let mut table = TableBuilder::new(ui).columns(Column::auto(), COLUMNS.len());
    if let Some(row) = scroll_to {
        table = table.scroll_to_row(row, Some(Align::TOP));
    }
    table
        .header(20.0, |mut header| {
            for name in COLUMNS {
                header.col(|ui| {
                    ui.label(name);
                });
            }
        })
        .body(|body| {
            body.rows(18.0, total_rows, |mut row| {
                let index = row.index();
                row.set_selected(index == selected);
                for name in COLUMNS {
                    row.col(|ui| {
                        ui.label(format!("{name} {index}"));
                    });
                }
            });
        });
}

/// The row and cell that hold the label with this text.
fn row_and_cell_of<'t>(
    harness: &'t Harness<'_>,
    text: &'t str,
) -> (AccessKitNode<'t>, AccessKitNode<'t>) {
    let label = harness.get_by_label(text).accesskit_node();
    let cell = label.parent().expect("label has a cell");
    let row = cell.parent().expect("cell has a row");
    (row, cell)
}

#[test]
fn table_has_header_row_and_body_rows() {
    let harness = Harness::builder()
        .with_size(egui::vec2(400.0, 200.0))
        .build_ui(|ui| process_table(ui, 300, 1, None));

    let table = harness.get_by_role(Role::Table).accesskit_node();
    assert_eq!(table.data().column_count(), Some(3));
    // The header row plus every body row, including the ones not shown.
    assert_eq!(table.data().row_count(), Some(301));

    let rows: Vec<_> = table.children().collect();
    assert!(rows.iter().all(|row| row.role() == Role::Row));
    assert!(rows.len() < 301, "only the visible rows are in the tree");

    let (header_row, name_header) = row_and_cell_of(&harness, "Name");
    assert_eq!(header_row.id(), rows[0].id());
    assert_eq!(header_row.data().row_index(), Some(0));
    assert_eq!(name_header.role(), Role::ColumnHeader);
    let header_columns: Vec<_> = header_row
        .children()
        .map(|cell| (cell.role(), cell.data().column_index()))
        .collect();
    assert_eq!(
        header_columns,
        [
            (Role::ColumnHeader, Some(0)),
            (Role::ColumnHeader, Some(1)),
            (Role::ColumnHeader, Some(2)),
        ]
    );

    let (row, cell) = row_and_cell_of(&harness, "Memory 0");
    assert_eq!(row.id(), rows[1].id());
    assert_eq!(row.data().row_index(), Some(1));
    assert_eq!(cell.role(), Role::Cell);
    assert_eq!(cell.data().column_index(), Some(2));
}

#[test]
fn selected_row_is_selected() {
    let harness = Harness::builder()
        .with_size(egui::vec2(400.0, 200.0))
        .build_ui(|ui| process_table(ui, 300, 1, None));

    let (selected, _) = row_and_cell_of(&harness, "Name 1");
    assert_eq!(selected.is_selected(), Some(true));

    let (other, _) = row_and_cell_of(&harness, "Name 2");
    assert_ne!(other.is_selected(), Some(true));
}

#[test]
fn scrolled_rows_keep_their_index() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 200.0))
        .build_ui(|ui| process_table(ui, 300, 1, Some(150)));
    harness.run();

    assert!(
        harness.query_by_label("Name 0").is_none(),
        "the first rows are scrolled out of view"
    );
    let (row, _) = row_and_cell_of(&harness, "Name 150");
    assert_eq!(row.data().row_index(), Some(151));

    let table = harness.get_by_role(Role::Table).accesskit_node();
    assert_eq!(table.data().row_count(), Some(301));
}

#[test]
fn table_without_header_counts_rows_from_zero() {
    let harness = Harness::new_ui(|ui| {
        TableBuilder::new(ui)
            .column(Column::auto())
            .body(|mut body| {
                for index in 0..3 {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.label(format!("Row {index}"));
                        });
                    });
                }
            });
    });

    let table = harness.get_by_role(Role::Table).accesskit_node();
    assert_eq!(table.data().row_count(), Some(3));
    assert_eq!(table.data().column_count(), Some(1));

    let (row, cell) = row_and_cell_of(&harness, "Row 0");
    assert_eq!(row.data().row_index(), Some(0));
    assert_eq!(cell.role(), Role::Cell);
    assert_eq!(cell.data().column_index(), Some(0));
}
