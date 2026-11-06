#[derive(PartialEq)]
pub struct GridTest {
    num_cols: usize,
    num_rows: usize,
    min_col_width: f32,
    max_col_width: f32,
    text_length: usize,
}

impl Default for GridTest {
    fn default() -> Self {
        Self {
            num_cols: 4,
            num_rows: 4,
            min_col_width: 10.0,
            max_col_width: 200.0,
            text_length: 10,
        }
    }
}

impl crate::Demo for GridTest {
    fn name(&self) -> &'static str {
        "Grid Test"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(crate::i18n::t(self.name()))
            .open(open)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for GridTest {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::Slider::new(&mut self.min_col_width, 0.0..=400.0)
                .text(crate::i18n::t("Minimum column width")),
        );
        ui.add(
            egui::Slider::new(&mut self.max_col_width, 0.0..=400.0)
                .text(crate::i18n::t("Maximum column width")),
        );
        ui.add(egui::Slider::new(&mut self.num_cols, 0..=5).text(crate::i18n::t("Columns")));
        ui.add(egui::Slider::new(&mut self.num_rows, 0..=20).text(crate::i18n::t("Rows")));

        ui.separator();

        let words = [
            crate::i18n::t("random"),
            crate::i18n::t("words"),
            crate::i18n::t("in"),
            crate::i18n::t("a"),
            crate::i18n::t("random"),
            crate::i18n::t("order"),
            crate::i18n::t("that"),
            crate::i18n::t("just"),
            crate::i18n::t("keeps"),
            crate::i18n::t("going"),
            crate::i18n::t("with"),
            crate::i18n::t("some"),
            crate::i18n::t("more"),
        ];

        egui::Grid::new("my_grid")
            .striped(true)
            .min_col_width(self.min_col_width)
            .max_col_width(self.max_col_width)
            .show(ui, |ui| {
                for row in 0..self.num_rows {
                    for col in 0..self.num_cols {
                        if col == 0 {
                            // The `{row}` placeholder is filled by `i18n::fmt`, not by `format!`.
                            #[expect(clippy::literal_string_with_formatting_args)]
                            ui.label(crate::i18n::fmt(
                                crate::i18n::t("row {row}"),
                                &[("row", row.to_string().as_str())],
                            ));
                        } else {
                            let word_idx = row * 3 + col * 5;
                            let word_count = (row * 5 + col * 75) % 13;
                            let mut string = String::new();
                            for word in words.iter().cycle().skip(word_idx).take(word_count) {
                                string += word;
                                string += " ";
                            }
                            ui.label(string);
                        }
                    }
                    ui.end_row();
                }
            });

        ui.separator();
        ui.add(
            egui::Slider::new(&mut self.text_length, 1..=40).text(crate::i18n::t("Text length")),
        );
        egui::Grid::new("parent grid").striped(true).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(crate::i18n::t("Vertical nest1"));
                ui.label(crate::i18n::t("Vertical nest2"));
            });
            ui.label(crate::i18n::t("First row, second column"));
            ui.end_row();

            ui.horizontal(|ui| {
                ui.label(crate::i18n::t("Horizontal nest1"));
                ui.label(crate::i18n::t("Horizontal nest2"));
            });
            ui.label(crate::i18n::t("Second row, second column"));
            ui.end_row();

            ui.scope(|ui| {
                ui.label(crate::i18n::t("Scope nest 1"));
                ui.label(crate::i18n::t("Scope nest 2"));
            });
            ui.label(crate::i18n::t("Third row, second column"));
            ui.end_row();

            egui::Grid::new("nested grid").show(ui, |ui| {
                ui.label(crate::i18n::t("Grid nest11"));
                ui.label(crate::i18n::t("Grid nest12"));
                ui.end_row();
                ui.label(crate::i18n::t("Grid nest21"));
                ui.label(crate::i18n::t("Grid nest22"));
                ui.end_row();
            });
            ui.label(crate::i18n::t("Fourth row, second column"));
            ui.end_row();

            let mut dyn_text = String::from("O");
            dyn_text.extend(core::iter::repeat_n('h', self.text_length));
            ui.label(dyn_text);
            ui.label(crate::i18n::t("Fifth row, second column"));
            ui.end_row();
        });

        ui.vertical_centered(|ui| {
            egui::reset_button(ui, self, crate::i18n::t("Reset"));
            ui.add(crate::egui_github_link_file!());
        });
    }
}
