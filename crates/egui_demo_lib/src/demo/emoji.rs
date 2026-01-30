/// Showcase for color emoji support.
pub struct EmojiDemo {
    sample_text: String,
    font_size: f32,
}

impl Default for EmojiDemo {
    fn default() -> Self {
        Self {
            sample_text: "Hello 😀 World! 🦀 Rust is awesome 🎉".to_owned(),
            font_size: 24.0,
        }
    }
}

impl crate::Demo for EmojiDemo {
    fn name(&self) -> &'static str {
        "🎨 Color Emoji"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .default_width(400.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for EmojiDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.add_space(8.0);

        ui.label("egui supports color emoji via the optional egui_noto_emoji crate.");
        ui.label("Emoji render in full color and don't get tinted by text color.");

        ui.add_space(12.0);
        ui.separator();

        // Emoji grid section
        ui.heading("Emoji Grid");
        ui.add_space(4.0);

        const SAMPLE_EMOJI: &[&str] = &[
            "😀", "😃", "😄", "😁", "😅", "😂", "🤣", "😊", "😇", "🙂", "😉", "😌", "😍", "🥰",
            "😘", "😋", "🎉", "🎊", "🎈", "🎁", "✨", "🌟", "⭐", "🔥", "💗", "🧡", "💛", "💚",
            "💙", "💜", "🖤", "🤍", "🦀", "🐧", "🦊", "🐱", "🐶", "🐼", "🦁", "🐯",
        ];

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            for emoji in SAMPLE_EMOJI {
                ui.label(egui::RichText::new(*emoji).size(28.0));
            }
        });

        ui.add_space(12.0);
        ui.separator();

        // Emoji in text section
        ui.heading("Emoji in Text");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Font size:");
            ui.add(egui::Slider::new(&mut self.font_size, 12.0..=48.0).suffix(" px"));
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Sample text:");
            ui.text_edit_singleline(&mut self.sample_text);
        });

        ui.add_space(8.0);

        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            ui.label(egui::RichText::new(&self.sample_text).size(self.font_size));
        });

        ui.add_space(12.0);
        ui.separator();

        // Size comparison
        ui.heading("Size Comparison");
        ui.add_space(4.0);

        for size in [14.0, 18.0, 24.0, 32.0, 48.0] {
            ui.horizontal(|ui| {
                ui.label(format!("{size:>2} px:"));
                ui.label(egui::RichText::new("🦀 Ferris says hello! 🎉").size(size));
            });
        }

        ui.add_space(12.0);
        ui.separator();

        // Ferris showcase
        ui.heading("Custom Glyphs");
        ui.add_space(4.0);

        ui.label("The crab emoji 🦀 is replaced with a custom Ferris sprite!");

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            for size in [24.0, 48.0, 72.0, 96.0] {
                ui.label(egui::RichText::new("🦀").size(size));
            }
        });
    }
}
