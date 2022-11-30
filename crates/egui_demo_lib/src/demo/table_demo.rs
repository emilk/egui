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
    scroll_to_row_slider: usize,
    scroll_to_row: Option<usize>,
}

impl Default for TableDemo {
    fn default() -> Self {
        Self {
            demo: DemoType::Manual,
            striped: true,
            resizable: true,
            num_rows: 10_000,
            scroll_to_row_slider: 0,
            scroll_to_row: None,
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

const NUM_MANUAL_ROWS: usize = 20;

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

            {
                let max_rows = if self.demo == DemoType::Manual {
                    NUM_MANUAL_ROWS
                } else {
                    self.num_rows
                };

                let slider_response = ui.add(
                    egui::Slider::new(&mut self.scroll_to_row_slider, 0..=max_rows)
                        .logarithmic(true)
                        .text("Row to scroll to"),
                );
                if slider_response.changed() {
                    self.scroll_to_row = Some(self.scroll_to_row_slider);
                }
            }
        });

        ui.separator();

        // Leave room for the source code link after the table demo:
        use egui_extras::{Size, StripBuilder};
        StripBuilder::new(ui)
            .size(Size::remainder().at_least(100.0)) // for the table
            .size(Size::exact(10.0)) // for the source code link
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        self.table_ui(ui);
                    });
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
            .column(Column::initial(100.0).range(40.0..=300.0).resizable(true))
            .column(
                Column::initial(100.0)
                    .at_least(40.0)
                    .resizable(true)
                    .clip(true),
            )
            .column(Column::remainder())
            .min_scrolled_height(0.0);

        if let Some(row_nr) = self.scroll_to_row.take() {
            table = table.scroll_to_row(row_nr, None);
        }

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Row");
                });
                header.col(|ui| {
                    ui.strong("Expanding content");
                });
                header.col(|ui| {
                    ui.strong("Clipped text");
                });
                header.col(|ui| {
                    ui.strong("Content");
                });
            })
            .body(|mut body| match self.demo {
                DemoType::Manual => {
                    for row_index in 0..NUM_MANUAL_ROWS {
                        let is_thick = thick_row(row_index);
                        let row_height = if is_thick { 30.0 } else { 18.0 };
                        body.row(row_height, |mut row| {
                            row.col(|ui| {
                                ui.label(row_index.to_string());
                            });
                            row.col(|ui| {
                                expanding_content(ui);
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
                            expanding_content(ui);
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
                                ui.label(row_index.to_string());
                            });
                            row.col(|ui| {
                                expanding_content(ui);
                            });
                            row.col(|ui| {
                                ui.label(long_text(row_index));
                            });
                            row.col(|ui| {
                                ui.style_mut().wrap = Some(false);
                                if thick_row(row_index) {
                                    ui.heading("Extra thick row");
                                } else {
                                    ui.label("Normal row");
                                }
                            });
                        },
                    );
                }
            });
    }
}

fn expanding_content(ui: &mut egui::Ui) {
    let width = ui.available_width().clamp(20.0, 200.0);
    let height = ui.available_height();
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        (1.0, ui.visuals().text_color()),
    );
}

fn long_text(row_index: usize) -> String {
    format!("Row {row_index} has some long text that you may want to clip, or it will take up too much horizontal space!")
}

fn thick_row(row_index: usize) -> bool {
    row_index % 6 == 0
}
