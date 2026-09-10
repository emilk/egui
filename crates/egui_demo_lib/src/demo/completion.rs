use core::ops::RangeInclusive;

use egui::{CompletionPopup, CompletionQuery, Suggestion, TextEdit, has_emoji_presentation};

/// The Unicode blocks we look for emoji in.
const EMOJI_RANGES: [RangeInclusive<u32>; 2] = [0x2600..=0x27BF, 0x1F000..=0x1FAFF];

/// How many suggestions to show at most.
const MAX_SUGGESTIONS: usize = 100;

/// An emoji and its name, e.g. `:grinning_face:`.
struct Emoji {
    chr: char,
    name: String,
}

/// Showcase [`egui::CompletionPopup`], completing emoji names like `:smile:`.
#[derive(Default)]
pub struct CompletionDemo {
    text: String,

    /// All emoji with names, collected on first use.
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
        let emojis = emojis.get_or_insert_with(all_emojis);

        CompletionPopup::new(ui.make_persistent_id("emoji_completion"))
            // A word is `:` plus a name, so an emoji (or any other symbol) ends the word before it:
            .word_boundary(|c| !(c.is_alphanumeric() || c == '_' || c == ':'))
            .show(
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

    let (mut starts_with, contains): (Vec<&Emoji>, Vec<&Emoji>) = emojis
        .iter()
        .filter(|emoji| emoji.name.contains(filter))
        .partition(|emoji| emoji.name.starts_with(filter));
    starts_with.extend(contains);

    starts_with
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|Emoji { chr, name }| {
            Suggestion::new(chr.to_string()).content(format!("{chr}  :{name}:"))
        })
        .collect()
}

/// All named emoji in [`EMOJI_RANGES`].
///
/// This does not look at the installed fonts, so it also works on web,
/// where the browser draws the emoji.
fn all_emojis() -> Vec<Emoji> {
    EMOJI_RANGES
        .into_iter()
        .flatten()
        .filter_map(char::from_u32)
        .filter(|chr| has_emoji_presentation(&chr.to_string()))
        .filter_map(|chr| {
            let name = unicode_names2::name(chr)?
                .to_string()
                .to_lowercase()
                .replace(' ', "_");
            Some(Emoji { chr, name })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Key, accesskit::Role};
    use egui_kittest::{Harness, kittest::Queryable as _};

    #[test]
    fn popup_reopens_right_after_an_emoji() {
        let mut harness = Harness::new_ui_state(
            |ui, demo: &mut CompletionDemo| {
                use crate::View as _;
                demo.ui(ui);
            },
            CompletionDemo::default(),
        );
        harness.run();
        harness.get_by_role(Role::MultilineTextInput).focus();
        harness.run();

        for _ in 0..2 {
            harness
                .get_by_role(Role::MultilineTextInput)
                .type_text(":grinning_face");
            harness.run();
            harness.key_press(Key::Enter);
            harness.run();
        }
        assert_eq!(harness.state().text, "😀😀");
    }

    #[test]
    fn a_lone_colon_suggests_something() {
        let emojis = all_emojis();
        assert!(1000 < emojis.len(), "found only {} emoji", emojis.len());

        let query = CompletionQuery {
            text: ":",
            word: ":",
            word_range: egui::text::CharIndex(0)..egui::text::CharIndex(1),
        };
        assert_eq!(suggest_emojis(&emojis, &query).len(), MAX_SUGGESTIONS);
    }
}
