use egui::TextStyle;

#[derive(PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum DemoType {
    Manual,
    ManyHomogeneous,
    ManyHeterogenous,
}

/// Shows off a table with dynamic layout
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TableDemo {
    demo: DemoType,
    striped: bool,
    resizable: bool,
    clickable: bool,
    num_rows: usize,
    scroll_to_row_slider: usize,
    scroll_to_row: Option<usize>,
    selection: std::collections::HashSet<usize>,
    checked: bool,
}

impl Default for TableDemo {
    fn default() -> Self {
        Self {
            demo: DemoType::Manual,
            striped: true,
            resizable: true,
            clickable: true,
            num_rows: 10_000,
            scroll_to_row_slider: 0,
            scroll_to_row: None,
            selection: Default::default(),
            checked: false,
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
                ui.checkbox(&mut self.clickable, "Clickable rows");
            });

            ui.label("Table type:");
            ui.radio_value(&mut self.demo, DemoType::Manual, "Few, manual rows");
            ui.radio_value(
                &mut self.demo,
                DemoType::ManyHomogeneous,
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
        let body_text_size = TextStyle::Body.resolve(ui.style()).size;
        use egui_extras::{Size, StripBuilder};
        StripBuilder::new(ui)
            .size(Size::remainder().at_least(100.0)) // for the table
            .size(Size::exact(body_text_size)) // for the source code link
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

        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        let mut table = TableBuilder::new(ui)
            .striped(self.striped)
            .resizable(self.resizable)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::initial(100.0).range(40.0..=300.0))
            .column(Column::initial(100.0).at_least(40.0).clip(true))
            .column(Column::remainder())
            .min_scrolled_height(0.0);

        if self.clickable {
            table = table.sense(egui::Sense::click());
        }

        if let Some(row_index) = self.scroll_to_row.take() {
            table = table.scroll_to_row(row_index, None);
        }

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Row");
                });
                header.col(|ui| {
                    ui.strong("Interaction");
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
                            row.set_selected(self.selection.contains(&row_index));

                            row.col(|ui| {
                                ui.label(row_index.to_string());
                            });
                            row.col(|ui| {
                                ui.checkbox(&mut self.checked, "Click me");
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

                            self.toggle_row_selection(row_index, &row.response());
                        });
                    }
                }
                DemoType::ManyHomogeneous => {
                    body.rows(text_height, self.num_rows, |mut row| {
                        let row_index = row.index();
                        row.set_selected(self.selection.contains(&row_index));

                        row.col(|ui| {
                            ui.label(row_index.to_string());
                        });
                        row.col(|ui| {
                            ui.checkbox(&mut self.checked, "Click me");
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

                        self.toggle_row_selection(row_index, &row.response());
                    });
                }
                DemoType::ManyHeterogenous => {
                    let row_height = |i: usize| if thick_row(i) { 30.0 } else { 18.0 };
                    body.heterogeneous_rows((0..self.num_rows).map(row_height), |mut row| {
                        let row_index = row.index();
                        row.set_selected(self.selection.contains(&row_index));

                        row.col(|ui| {
                            ui.label(row_index.to_string());
                        });
                        row.col(|ui| {
                            ui.checkbox(&mut self.checked, "Click me");
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

                        self.toggle_row_selection(row_index, &row.response());
                    });
                }
            });
    }

    fn toggle_row_selection(&mut self, row_index: usize, row_response: &egui::Response) {
        if row_response.clicked() {
            if self.selection.contains(&row_index) {
                self.selection.remove(&row_index);
            } else {
                self.selection.insert(row_index);
            }
        }
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
