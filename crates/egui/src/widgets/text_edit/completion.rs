use emath::RectAlign;

use crate::{
    Button, EventFilter, Id, Key, Modifiers, Popup, PopupKind, ScrollArea, TextEdit, Ui,
    WidgetText,
    text::{CCursor, CCursorRange, CharIndex},
    vec2,
};

use super::{TextBuffer, TextEditOutput, TextEditState};

/// One item in a [`CompletionPopup`].
#[derive(Clone)]
pub struct Suggestion {
    /// The text that replaces the current word when this suggestion is accepted.
    pub insert: String,

    /// What to show in the popup. Defaults to [`Self::insert`].
    pub label: WidgetText,

    /// Optional extra text shown weakly to the right of the label.
    pub description: Option<WidgetText>,
}

impl Suggestion {
    /// A suggestion that inserts `insert`, showing it in the popup as-is.
    pub fn new(insert: impl Into<String>) -> Self {
        let insert = insert.into();
        Self {
            label: WidgetText::from(insert.as_str()),
            insert,
            description: None,
        }
    }

    /// What to show in the popup instead of [`Self::insert`].
    #[inline]
    pub fn label(mut self, label: impl Into<WidgetText>) -> Self {
        self.label = label.into();
        self
    }

    /// Extra text shown weakly to the right of the label.
    #[inline]
    pub fn description(mut self, description: impl Into<WidgetText>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// The result of [`CompletionPopup::show`].
pub struct CompletionOutput {
    /// The output of the wrapped [`TextEdit`].
    pub text_edit: TextEditOutput,

    /// The suggestion accepted this frame (with Enter, Tab, or a click), if any.
    pub accepted: Option<Suggestion>,

    /// Is the popup visible?
    pub is_open: bool,
}

/// State stored between frames.
#[derive(Clone, Default)]
struct CompletionState {
    /// Index of the selected suggestion.
    selected: usize,

    /// Was the popup open at the end of the last frame?
    was_open: bool,

    /// The word for which the user dismissed the popup with Escape.
    /// The popup stays hidden until the word changes.
    dismissed_word: Option<String>,
}

impl CompletionState {
    fn load(ui: &Ui, id: Id) -> Self {
        ui.data(|data| data.get_temp(id)).unwrap_or_default()
    }

    fn store(self, ui: &Ui, id: Id) {
        ui.data_mut(|data| data.insert_temp(id, self));
    }
}

/// A code completion popup over a [`TextEdit`].
///
/// Shows a list of suggestions for the word under the cursor.
/// Use the arrow keys to select one, and Enter or Tab to accept it.
/// Escape closes the popup until the word changes.
/// Clicking a suggestion also accepts it.
///
/// Keyboard focus stays in the [`TextEdit`] the whole time.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut text = String::new();
/// let commands = ["/help", "/clear", "/quit"];
/// let output = egui::CompletionPopup::new(egui::Id::new("prompt")).show(
///     ui,
///     &mut text,
///     |text| egui::TextEdit::singleline(text).hint_text("Type / for commands"),
///     |word| {
///         commands
///             .iter()
///             .filter(|command| word.starts_with('/') && command.starts_with(word))
///             .map(|command| egui::Suggestion::new(format!("{command} ")))
///             .collect()
///     },
/// );
/// if let Some(accepted) = output.accepted {
///     println!("Accepted {}", accepted.insert);
/// }
/// # });
/// ```
#[must_use = "You should call .show()"]
pub struct CompletionPopup {
    id: Id,
    is_word_boundary: fn(char) -> bool,
    align: RectAlign,
    max_height: f32,
    accept_keys: Vec<Key>,
}

impl CompletionPopup {
    /// The `id` is given to the [`TextEdit`], and is also used to store the popup state.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            is_word_boundary: char::is_whitespace,
            align: RectAlign::TOP_START,
            max_height: 200.0,
            accept_keys: vec![Key::Enter, Key::Tab],
        }
    }

    /// Which characters separate words? The current word is the text
    /// between the last such character and the cursor.
    ///
    /// Default: [`char::is_whitespace`].
    #[inline]
    pub fn word_boundary(mut self, is_word_boundary: fn(char) -> bool) -> Self {
        self.is_word_boundary = is_word_boundary;
        self
    }

    /// Where to place the popup relative to the [`TextEdit`].
    /// If there is no room, the vertically flipped alignment is used instead.
    ///
    /// Default: [`RectAlign::TOP_START`].
    #[inline]
    pub fn align(mut self, align: RectAlign) -> Self {
        self.align = align;
        self
    }

    /// Maximum height of the popup before it starts to scroll.
    ///
    /// Default: 200.0.
    #[inline]
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }

    /// Which keys accept the selected suggestion?
    ///
    /// Default: Enter and Tab.
    #[inline]
    pub fn accept_keys(mut self, accept_keys: impl Into<Vec<Key>>) -> Self {
        self.accept_keys = accept_keys.into();
        self
    }

    /// Show the [`TextEdit`] and, when `suggest` returns anything for the word under the cursor, the popup.
    ///
    /// `make_text_edit` builds the [`TextEdit`] from `text`, e.g. `|text| TextEdit::singleline(text)`.
    /// The popup needs the buffer separately, to insert the accepted suggestion.
    ///
    /// `suggest` is given the word before the cursor (see [`Self::word_boundary`]).
    /// It is called up to twice per frame: before the [`TextEdit`] handles input, and after.
    pub fn show(
        self,
        ui: &mut Ui,
        text: &mut dyn TextBuffer,
        make_text_edit: impl for<'t> FnOnce(&'t mut dyn TextBuffer) -> TextEdit<'t>,
        mut suggest: impl FnMut(&str) -> Vec<Suggestion>,
    ) -> CompletionOutput {
        let Self {
            id,
            is_word_boundary,
            align,
            max_height,
            accept_keys,
        } = self;

        let mut state = CompletionState::load(ui, id);
        let mut accepted = None;

        // Where is the cursor? We use the state from the last frame, before the `TextEdit` runs.
        let cursor = TextEditState::load(ui.ctx(), id)
            .and_then(|state| state.cursor.char_range())
            .map_or_else(
                || text.as_str().chars().count(),
                |range| range.primary.index.0,
            );
        let word = word_before_cursor(text.as_str(), cursor, is_word_boundary);
        let suggestions = state.filtered(&word, &mut suggest);

        // Handle the popup keys before the `TextEdit` sees them:
        let mut selection_changed = false;
        if state.was_open && !suggestions.is_empty() {
            let mut accept = false;
            ui.input_mut(|input| {
                if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    state.selected = (state.selected + 1) % suggestions.len();
                    selection_changed = true;
                }
                if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    state.selected = (state.selected + suggestions.len() - 1) % suggestions.len();
                    selection_changed = true;
                }
                if accept_keys
                    .iter()
                    .any(|&key| input.consume_key(Modifiers::NONE, key))
                {
                    accept = true;
                }
                if input.consume_key(Modifiers::NONE, Key::Escape) {
                    state.dismissed_word = Some(word.clone());
                }
            });

            if accept && let Some(suggestion) = suggestions.get(state.selected) {
                state.accept(ui, id, text, cursor, &word, suggestion);
                accepted = Some(suggestion.clone());
            }
        }

        let text_edit = make_text_edit(text).id(id).event_filter(EventFilter {
            horizontal_arrows: true,
            vertical_arrows: true,
            // Keep focus on Tab and Escape while the popup is open, so they can act on the popup:
            tab: state.was_open,
            escape: state.was_open,
        });
        let output = text_edit.show(ui);
        let response = output.response.response.clone();

        // Recompute with the up-to-date text and cursor, so the popup reacts to typing without a frame delay:
        let cursor = output.cursor_range.map_or_else(
            || text.as_str().chars().count(),
            |range| range.primary.index.0,
        );
        let word = word_before_cursor(text.as_str(), cursor, is_word_boundary);
        let suggestions = state.filtered(&word, &mut suggest);
        state.selected = state.selected.min(suggestions.len().saturating_sub(1));

        let is_open = response.has_focus() && !suggestions.is_empty();
        state.was_open = is_open;

        let clicked = Popup::from_response(&response)
            .id(id.with("completion_popup"))
            .kind(PopupKind::Popup)
            .open(is_open)
            .align(align)
            .align_alternatives(&[align.flipped_y()])
            .width(response.rect.width())
            .show(|ui| {
                ScrollArea::vertical()
                    .max_height(max_height)
                    .show(ui, |ui| {
                        let mut clicked = None;
                        for (i, suggestion) in suggestions.iter().enumerate() {
                            let is_selected = i == state.selected;
                            let mut button =
                                Button::selectable(is_selected, suggestion.label.clone())
                                    .min_size(vec2(ui.available_width(), 0.0));
                            if let Some(description) = &suggestion.description {
                                button = button.right_text(description.clone().weak());
                            }
                            let response = ui.add(button);
                            if is_selected && selection_changed {
                                response.scroll_to_me(None);
                            }
                            if response.clicked() {
                                clicked = Some(i);
                            }
                        }
                        clicked
                    })
                    .inner
            })
            .and_then(|inner| inner.inner);

        if let Some(i) = clicked
            && let Some(suggestion) = suggestions.get(i)
        {
            state.accept(ui, id, text, cursor, &word, suggestion);
            accepted = Some(suggestion.clone());
            // Clicking the popup took focus from the text field, so give it back:
            ui.memory_mut(|mem| mem.request_focus(id));
        }

        state.store(ui, id);

        CompletionOutput {
            text_edit: output,
            accepted,
            is_open,
        }
    }
}

impl CompletionState {
    fn filtered(
        &self,
        word: &str,
        suggest: &mut impl FnMut(&str) -> Vec<Suggestion>,
    ) -> Vec<Suggestion> {
        if self.dismissed_word.as_deref() == Some(word) {
            vec![]
        } else {
            suggest(word)
        }
    }

    /// Replace `word` (which ends at `cursor`) with the suggestion, and move the cursor after it.
    fn accept(
        &mut self,
        ui: &Ui,
        id: Id,
        text: &mut dyn TextBuffer,
        cursor: usize,
        word: &str,
        suggestion: &Suggestion,
    ) {
        let start = cursor - word.chars().count();
        text.delete_char_range(CharIndex(start)..CharIndex(cursor));
        let mut ccursor = CCursor::new(start);
        text.insert_text_at(&mut ccursor, &suggestion.insert, usize::MAX);

        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(CCursorRange::one(ccursor)));
        state.store(ui.ctx(), id);

        self.dismissed_word = None;
        self.selected = 0;
    }
}

/// The word that ends at the cursor (a char index).
fn word_before_cursor(text: &str, cursor: usize, is_word_boundary: fn(char) -> bool) -> String {
    let before_cursor: Vec<char> = text.chars().take(cursor).collect();
    let start = before_cursor
        .iter()
        .rposition(|&c| is_word_boundary(c))
        .map_or(0, |i| i + 1);
    before_cursor[start..].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_before_cursor_works() {
        assert_eq!(word_before_cursor("", 0, char::is_whitespace), "");
        assert_eq!(word_before_cursor("/gr", 3, char::is_whitespace), "/gr");
        assert_eq!(
            word_before_cursor("hello /gr", 9, char::is_whitespace),
            "/gr"
        );
        assert_eq!(word_before_cursor("hello /gr", 6, char::is_whitespace), "");
        assert_eq!(
            word_before_cursor("hello /gr", 8, char::is_whitespace),
            "/g"
        );
        assert_eq!(word_before_cursor("héllo", 5, char::is_whitespace), "héllo");
        assert_eq!(word_before_cursor("a.b", 3, |c| c == '.'), "b");
    }
}
