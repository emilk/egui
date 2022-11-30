#[derive(PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum DemoType {
    Manual,
    ManyHomogenous,
    ManyHeterogenous,
}

/// Shows off a table with dynamic layout
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TableDemo {
    demo: DemoType,
    striped: bool,
    resizable: bool,
    num_rows: usize,
    row_to_scroll_to: i32,
    vertical_scroll_offset: Option<f32>,
}

impl Default for TableDemo {
    fn default() -> Self {
        Self {
            demo: DemoType::Manual,
            striped: true,
            resizable: true,
            num_rows: 10_000,
            row_to_scroll_to: 0,
            vertical_scroll_offset: None,
        }
    }
}

impl super::Demo for TableDemo {
    fn name(&self) -> &'static str {
        "☰ Table Demo"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                use super::View as _;
                self.ui(ui);
            });
    }
}

fn scroll_offset_for_row(ui: &egui::Ui, row: i32) -> f32 {
    let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
    let row_item_spacing = ui.spacing().item_spacing.y;
    row as f32 * (text_height + row_item_spacing)
}

impl super::View for TableDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.striped, "Striped");
                ui.checkbox(&mut self.resizable, "Resizable columns");
            });

            ui.label("Table type:");
            ui.radio_value(&mut self.demo, DemoType::Manual, "Few, manual rows");
            ui.radio_value(
                &mut self.demo,
                DemoType::ManyHomogenous,
                "Thousands of rows of same height",
            );
            ui.radio_value(
                &mut self.demo,
                DemoType::ManyHeterogenous,
                "Thousands of rows of differing heights",
            );

            if self.demo != DemoType::Manual {
                ui.add(
                    egui::Slider::new(&mut self.num_rows, 0..=100_000)
                        .logarithmic(true)
                        .text("Num rows"),
                );
            }

            if self.demo == DemoType::ManyHomogenous {
                let slider_response = ui.add(
                    egui::Slider::new(&mut self.row_to_scroll_to, 0..=self.num_rows as i32)
                        .logarithmic(true)
                        .text("Row to scroll to"),
                );
                if slider_response.changed() {
                    self.vertical_scroll_offset
                        .replace(scroll_offset_for_row(ui, self.row_to_scroll_to));
                }
            }
        });

        ui.separator();

        // Leave room for the source code link after the table demo:
        use egui_extras::{Size, StripBuilder};
        StripBuilder::new(ui)
            .size(Size::remainder()) // for the table
            .size(Size::exact(10.0)) // for the source code link
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    self.table_ui(ui);
                });
                strip.cell(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(crate::egui_github_link_file!());
                    });
                });
            });
    }
}

impl TableDemo {
    fn table_ui(&mut self, ui: &mut egui::Ui) {
        use egui_extras::{Column, TableBuilder};

        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

        let mut table = TableBuilder::new(ui)
            .striped(self.striped)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(
                Column::initial(100.0)
                    .at_least(40.0)
                    .resizable(true)
                    .clip(true),
            )
            .column(Column::remainder());

        if let Some(y_scroll) = self.vertical_scroll_offset.take() {
            table = table.vertical_scroll_offset(y_scroll);
        }

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Row");
                });
                header.col(|ui| {
                    ui.strong("Long text");
                });
                header.col(|ui| {
                    ui.strong("Content");
                });
            })
            .body(|mut body| match self.demo {
                DemoType::Manual => {
                    for row_index in 0..20 {
                        let is_thick = thick_row(row_index);
                        let row_height = if is_thick { 30.0 } else { 18.0 };
                        body.row(row_height, |mut row| {
                            row.col(|ui| {
                                ui.label(row_index.to_string());
                            });
                            row.col(|ui| {
                                ui.label(long_text(row_index));
                            });
                            row.col(|ui| {
                                ui.style_mut().wrap = Some(false);
                                if is_thick {
                                    ui.heading("Extra thick row");
                                } else {
                                    ui.label("Normal row");
                                }
                            });
                        });
                    }
                }
                DemoType::ManyHomogenous => {
                    body.rows(text_height, self.num_rows, |row_index, mut row| {
                        row.col(|ui| {
                            ui.label(row_index.to_string());
                        });
                        row.col(|ui| {
                            ui.label(long_text(row_index));
                        });
                        row.col(|ui| {
                            ui.add(
                                egui::Label::new("Thousands of rows of even height").wrap(false),
                            );
                        });
                    });
                }
                DemoType::ManyHeterogenous => {
                    fn row_thickness(row_index: usize) -> f32 {
                        if thick_row(row_index) {
                            30.0
                        } else {
                            18.0
                        }
                    }
                    body.heterogeneous_rows(
                        (0..self.num_rows).into_iter().map(row_thickness),
                        |row_index, mut row| {
                            row.col(|ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.label(row_index.to_string());
                                });
                            });
                            row.col(|ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.label(long_text(row_index));
                                });
                            });
                            row.col(|ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.style_mut().wrap = Some(false);
                                    if thick_row(row_index) {
                                        ui.heading("Extra thick row");
                                    } else {
                                        ui.label("Normal row");
                                    }
                                });
                            });
                        },
                    );
                }
            });
    }
}

fn long_text(row_index: usize) -> String {
    format!("Row {row_index} has some long text that you may want to clip, or it will overflow")
}

fn thick_row(row_index: usize) -> bool {
    row_index % 6 == 0
}
