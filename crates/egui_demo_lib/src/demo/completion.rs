use egui::{CompletionPopup, CompletionQuery, FontFamily, Suggestion, TextEdit};

/// An emoji and its name, e.g. `:grinning_face:`.
struct Emoji {
    chr: char,
    name: String,
}

/// Showcase [`egui::CompletionPopup`], completing emoji names like `:smile:`.
#[derive(Default)]
pub struct CompletionDemo {
    text: String,

    /// All emoji in the current fonts, looked up on first use.
    emojis: Option<Vec<Emoji>>,
}

impl crate::Demo for CompletionDemo {
    fn name(&self) -> &'static str {
        "😀 Completion"
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

impl crate::View for CompletionDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("A ");
            ui.code("CompletionPopup");
            ui.label(" over a ");
            ui.code("TextEdit");
            ui.label(". Type a colon and the name of an emoji, e.g. ");
            ui.code(":smile");
            ui.label(". The arrow keys select, Enter or Tab accepts, and Escape closes the popup.");
        });

        let Self { text, emojis } = self;
        let emojis = emojis.get_or_insert_with(|| available_emojis(ui));

        CompletionPopup::new(ui.make_persistent_id("emoji_completion")).show(
            ui,
            TextEdit::multiline(text)
                .hint_text("Type :smile: …")
                .desired_width(f32::INFINITY),
            |query| suggest_emojis(emojis, query),
        );
    }
}

/// Suggest emoji for a word like `:smi`, best matches first.
fn suggest_emojis(emojis: &[Emoji], query: &CompletionQuery<'_>) -> Vec<Suggestion> {
    let Some(filter) = query.word.strip_prefix(':') else {
        return vec![];
    };
    if filter.is_empty() {
        return vec![];
    }

    let (mut starts_with, contains): (Vec<&Emoji>, Vec<&Emoji>) = emojis
        .iter()
        .filter(|emoji| emoji.name.contains(filter))
        .partition(|emoji| emoji.name.starts_with(filter));
    starts_with.extend(contains);

    starts_with
        .into_iter()
        .map(|Emoji { chr, name }| {
            Suggestion::new(chr.to_string()).content(format!("{chr}  :{name}:"))
        })
        .collect()
}

/// All named characters in the emoji fonts of the proportional family.
fn available_emojis(ui: &egui::Ui) -> Vec<Emoji> {
    ui.fonts_mut(|fonts| {
        fonts
            .characters(&FontFamily::Proportional)
            .iter()
            .filter(|(_, fonts)| {
                fonts
                    .iter()
                    .any(|font| font.to_lowercase().contains("emoji"))
            })
            .filter_map(|(chr, _)| {
                let name = unicode_names2::name(*chr)?
                    .to_string()
                    .to_lowercase()
                    .replace(' ', "_");
                Some(Emoji { chr: *chr, name })
            })
            .collect()
    })
}
