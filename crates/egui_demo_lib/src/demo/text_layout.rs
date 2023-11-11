/// Showcase text layout
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextLayoutDemo {
    break_anywhere: bool,
    max_rows: usize,
    overflow_character: Option<char>,
    extra_letter_spacing_pixels: i32,
    line_height_pixels: u32,
    lorem_ipsum: bool,
}

impl Default for TextLayoutDemo {
    fn default() -> Self {
        Self {
            max_rows: 6,
            break_anywhere: true,
            overflow_character: Some('…'),
            extra_letter_spacing_pixels: 0,
            line_height_pixels: 0,
            lorem_ipsum: true,
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
            lorem_ipsum,
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

                ui.label("Text:");
                ui.horizontal(|ui| {
                    ui.selectable_value(lorem_ipsum, true, "Lorem Ipsum");
                    ui.selectable_value(lorem_ipsum, false, "La Pasionaria");
                });
            });

        ui.add_space(12.0);

        let text = if *lorem_ipsum {
            crate::LOREM_IPSUM_LONG
        } else {
            TO_BE_OR_NOT_TO_BE
        };

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let extra_letter_spacing = points_per_pixel * *extra_letter_spacing_pixels as f32;
                let line_height = (*line_height_pixels != 0)
                    .then_some(points_per_pixel * *line_height_pixels as f32);

                let mut job = LayoutJob::single_section(
                    text.to_owned(),
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

/// Excerpt from Dolores Ibárruri's farwel speech to the International Brigades:
const TO_BE_OR_NOT_TO_BE: &str = "Mothers! Women!\n
When the years pass by and the wounds of war are stanched; when the memory of the sad and bloody days dissipates in a present of liberty, of peace and of wellbeing; when the rancor have died out and pride in a free country is felt equally by all Spaniards, speak to your children. Tell them of these men of the International Brigades.\n\
\n\
Recount for them how, coming over seas and mountains, crossing frontiers bristling with bayonets, sought by raving dogs thirsting to tear their flesh, these men reached our country as crusaders for freedom, to fight and die for Spain’s liberty and independence threatened by German and Italian fascism. \
They gave up everything — their loves, their countries, home and fortune, fathers, mothers, wives, brothers, sisters and children — and they came and said to us: “We are here. Your cause, Spain’s cause, is ours. It is the cause of all advanced and progressive mankind.”\n\
\n\
- Dolores Ibárruri, 1938";
