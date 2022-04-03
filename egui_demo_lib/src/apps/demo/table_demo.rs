use egui::TextStyle;
use egui_extras::{Size, StripBuilder, TableBuilder, TableRow};

/// Shows off a table with dynamic layout
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default)]
pub struct TableDemo {
    heterogeneous_rows: bool,
    resizable: bool,
    num_rows: usize,
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

impl super::View for TableDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        // Leave room for the source code link after the table demo:
        StripBuilder::new(ui)
            .size(Size::exact(50.0)) // for the settings
            .size(Size::remainder()) // for the table
            .size(Size::exact(10.0)) // for the source code link
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    StripBuilder::new(ui)
                        .size(Size::exact(150.0))
                        .size(Size::remainder())
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.checkbox(&mut self.heterogeneous_rows, "Heterogeneous rows");
                                ui.checkbox(&mut self.resizable, "Resizable columns");
                            });
                            strip.cell(|ui| {
                                ui.add(
                                    egui::Slider::new(&mut self.num_rows, 0..=300_000)
                                        .text("Num rows"),
                                );
                            });
                        });
                });
                strip.cell(|ui| {
                    self.table_ui(ui);
                });
                strip.cell(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(crate::__egui_github_link_file!());
                    });
                });
            });
    }
}

impl TableDemo {
    fn table_ui(&mut self, ui: &mut egui::Ui) {
        let text_height = TextStyle::Body.resolve(ui.style()).size;

        TableBuilder::new(ui)
            .striped(true)
            .column(Size::initial(60.0).at_least(40.0))
            .column(Size::initial(60.0).at_least(40.0))
            .column(Size::remainder().at_least(60.0))
            .resizable(self.resizable)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.centered_and_justified(|ui| {
                        ui.heading("Row");
                    });
                });
                header.col(|ui| {
                    ui.centered_and_justified(|ui| {
                        ui.heading("Clock");
                    });
                });
                header.col(|ui| {
                    ui.centered_and_justified(|ui| {
                        ui.heading("Content");
                    });
                });
            })
            .body(|mut body| {
                if !self.heterogeneous_rows {
                    body.rows(text_height, self.num_rows, |row_index, mut row| {
                        row.col(|ui| {
                            ui.label(row_index.to_string());
                        });
                        row.col(|ui| {
                            ui.label(clock_emoji(row_index));
                        });
                        row.col(|ui| {
                            ui.add(
                                egui::Label::new("Thousands of rows of even height").wrap(false),
                            );
                        });
                    });
                } else {
                    let rows = DemoRows::new(self.num_rows);
                    body.heterogeneous_rows(rows, DemoRows::populate_row);
                }
            });
    }
}

struct DemoRows {
    row_count: usize,
    current_row: usize,
}

impl DemoRows {
    fn new(row_count: usize) -> Self {
        Self {
            row_count,
            current_row: 0,
        }
    }

    fn populate_row(index: usize, mut row: TableRow<'_, '_>) {
        let thick = index % 6 == 0;
        row.col(|ui| {
            ui.centered_and_justified(|ui| {
                ui.label(index.to_string());
            });
        });
        row.col(|ui| {
            ui.centered_and_justified(|ui| {
                ui.label(clock_emoji(index));
            });
        });
        row.col(|ui| {
            ui.centered_and_justified(|ui| {
                ui.style_mut().wrap = Some(false);
                if thick {
                    ui.heading("Extra thick row");
                } else {
                    ui.label("Normal row");
                }
            });
        });
    }
}

impl Iterator for DemoRows {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row < self.row_count {
            let thick = self.current_row % 6 == 0;
            self.current_row += 1;
            Some(if thick { 30.0 } else { 18.0 })
        } else {
            None
        }
    }
}

fn clock_emoji(row_index: usize) -> String {
    char::from_u32(0x1f550 + row_index as u32 % 24)
        .unwrap()
        .to_string()
}
