pub struct FontBook {
    standard: bool,
    emojis: bool,
    filter: String,
    text_style: egui::TextStyle,
}

impl Default for FontBook {
    fn default() -> Self {
        Self {
            standard: false,
            emojis: true,
            filter: Default::default(),
            text_style: egui::TextStyle::Button,
        }
    }
}

impl FontBook {
    fn characters_ui(&self, ui: &mut egui::Ui, characters: &[(u32, char, &str)]) {
        use egui::{Button, Label};
        for &(_, chr, name) in characters {
            if self.filter.is_empty()
                || name.contains(&self.filter)
                || self.filter == chr.to_string()
            {
                let button = Button::new(chr).text_style(self.text_style).frame(false);

                let tooltip_ui = |ui: &mut egui::Ui| {
                    ui.add(Label::new(chr).text_style(self.text_style));
                    ui.label(format!("{}\nU+{:X}\n\nClick to copy", name, chr as u32));
                };

                if ui.add(button).on_hover_ui(tooltip_ui).clicked() {
                    ui.output().copied_text = chr.to_string();
                }
            }
        }
    }
}

impl super::Demo for FontBook {
    fn name(&self) -> &'static str {
        "🔤 Font Book"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        egui::Window::new(self.name()).open(open).show(ctx, |ui| {
            use super::View;
            self.ui(ui);
        });
    }
}

impl super::View for FontBook {
    fn ui(&mut self, ui: &mut egui::Ui) {
        use super::font_contents_emoji::FULL_EMOJI_LIST;
        use super::font_contents_ubuntu::UBUNTU_FONT_CHARACTERS;

        ui.label(format!(
            "The default egui fonts supports {} standard characters and {} emojis.",
            UBUNTU_FONT_CHARACTERS.len(),
            FULL_EMOJI_LIST.len(),
        ));

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("You can add more characters by installing additional fonts with ");
            ui.add(
                egui::Hyperlink::from_label_and_url(
                    "Context::set_fonts",
                    "https://docs.rs/egui/latest/egui/struct.Context.html#method.set_fonts",
                )
                .text_style(egui::TextStyle::Monospace),
            );
            ui.label(".");
        });

        ui.separator();

        egui::ComboBox::from_label("Text style")
            .selected_text(format!("{:?}", self.text_style))
            .show_ui(ui, |ui| {
                for style in egui::TextStyle::all() {
                    ui.selectable_value(&mut self.text_style, style, format!("{:?}", style));
                }
            });

        ui.horizontal(|ui| {
            ui.label("Show:");
            ui.checkbox(&mut self.standard, "Standard");
            ui.checkbox(&mut self.emojis, "Emojis");
        });

        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter);
            self.filter = self.filter.to_lowercase();
            if ui.button("ｘ").clicked() {
                self.filter.clear();
            }
        });

        ui.separator();

        egui::ScrollArea::auto_sized().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::splat(2.0);

                if self.standard {
                    self.characters_ui(ui, UBUNTU_FONT_CHARACTERS);
                }
                if self.emojis {
                    self.characters_ui(ui, FULL_EMOJI_LIST);
                }
            });
        });
    }
}
