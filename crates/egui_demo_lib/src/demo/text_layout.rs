/// Showcase text layout
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextLayoutDemo {
    break_anywhere: bool,
    max_rows: usize,
    overflow_character: Option<char>,
    extra_letter_spacing_pixels: i32,
    line_height_pixels: u32,
}

impl Default for TextLayoutDemo {
    fn default() -> Self {
        Self {
            max_rows: 3,
            break_anywhere: true,
            overflow_character: Some('…'),
            extra_letter_spacing_pixels: 0,
            line_height_pixels: 0,
        }
    }
}

impl super::Demo for TextLayoutDemo {
    fn name(&self) -> &'static str {
        "🖹 Text Layout"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(true)
            .show(ctx, |ui| {
                use super::View as _;
                self.ui(ui);
            });
    }
}

impl super::View for TextLayoutDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            break_anywhere,
            max_rows,
            overflow_character,
            extra_letter_spacing_pixels,
            line_height_pixels,
        } = self;

        use egui::text::LayoutJob;

        let pixels_per_point = ui.ctx().pixels_per_point();
        let points_per_pixel = 1.0 / pixels_per_point;

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file_line!());
        });

        ui.add_space(12.0);

        egui::Grid::new("TextLayoutDemo")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Max rows:");
                ui.add(egui::DragValue::new(max_rows));
                ui.end_row();

                ui.label("Line-break:");
                ui.horizontal(|ui| {
                    ui.radio_value(break_anywhere, false, "word boundaries");
                    ui.radio_value(break_anywhere, true, "anywhere");
                });
                ui.end_row();

                ui.label("Overflow character:");
                ui.horizontal(|ui| {
                    ui.selectable_value(overflow_character, None, "None");
                    ui.selectable_value(overflow_character, Some('…'), "…");
                    ui.selectable_value(overflow_character, Some('—'), "—");
                    ui.selectable_value(overflow_character, Some('-'), "  -  ");
                });
                ui.end_row();

                ui.label("Extra letter spacing:");
                ui.add(egui::DragValue::new(extra_letter_spacing_pixels).suffix(" pixels"));
                ui.end_row();

                ui.label("Line height:");
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(*line_height_pixels == 0, "Default")
                        .clicked()
                    {
                        *line_height_pixels = 0;
                    }
                    if ui
                        .selectable_label(*line_height_pixels != 0, "Custom")
                        .clicked()
                    {
                        *line_height_pixels = (pixels_per_point * 20.0).round() as _;
                    }
                    if *line_height_pixels != 0 {
                        ui.add(egui::DragValue::new(line_height_pixels).suffix(" pixels"));
                    }
                });
                ui.end_row();
            });

        ui.add_space(12.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let extra_letter_spacing = points_per_pixel * *extra_letter_spacing_pixels as f32;
            let line_height =
                (*line_height_pixels != 0).then_some(points_per_pixel * *line_height_pixels as f32);

            let mut job = LayoutJob::single_section(
                crate::LOREM_IPSUM_LONG.to_owned(),
                egui::TextFormat {
                    extra_letter_spacing,
                    line_height,
                    ..Default::default()
                },
            );
            job.wrap = egui::text::TextWrapping {
                max_rows: *max_rows,
                break_anywhere: *break_anywhere,
                overflow_character: *overflow_character,
                ..Default::default()
            };

            // NOTE: `Label` overrides some of the wrapping settings, e.g. wrap width
            ui.label(job);
        });
    }
}
