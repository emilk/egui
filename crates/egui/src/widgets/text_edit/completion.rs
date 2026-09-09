use core::ops::Range;

use emath::RectAlign;

use crate::{
    Atom, Atoms, Button, Event, EventFilter, Id, InputState, IntoAtoms, Key, Popup, PopupKind,
    ScrollArea, TextEdit, Ui, WidgetText,
    text::{CCursor, CCursorRange, CharIndex},
    vec2,
};

use super::{TextEditOutput, TextEditState};

/// One item in a [`CompletionPopup`].
#[derive(Clone)]
pub struct Suggestion {
    /// The text that replaces the current word when this suggestion is accepted.
    pub insert: String,

    /// What to show in the popup. Defaults to [`Self::insert`].
    pub content: Atoms<'static>,
}

impl Suggestion {
    /// A suggestion that inserts `insert`, showing it in the popup as-is.
    pub fn new(insert: impl Into<String>) -> Self {
        let insert = insert.into();
        Self {
            content: insert.clone().into_atoms(),
            insert,
        }
    }

    /// What to show in the popup instead of [`Self::insert`].
    #[inline]
    pub fn content(mut self, content: impl IntoAtoms<'static>) -> Self {
        self.content = content.into_atoms();
        self
    }

    /// Add weak text to the far right of the suggestion.
    #[inline]
    pub fn description(mut self, description: impl Into<WidgetText>) -> Self {
        self.content.push_right(Atom::grow());
        self.content.push_right(description.into().weak());
        self
    }
}

/// What the [`CompletionPopup`] asks for suggestions for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionQuery<'a> {
    /// The full text of the [`TextEdit`].
    pub text: &'a str,

    /// The word that ends at the cursor (see [`CompletionPopup::word_boundary`]).
    pub word: &'a str,

    /// Where in [`Self::text`] the word is, as char indices.
    pub word_range: Range<CharIndex>,
}

impl CompletionQuery<'_> {
    /// Is the word at the very start of the text?
    pub fn is_at_start(&self) -> bool {
        self.word_range.start == CharIndex(0)
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

    /// The word under the cursor when the selection was last used.
    ///
    /// When the user types, the word and thus the suggestions change,
    /// and the old selection index would point at an unrelated suggestion.
    /// So when the word differs from this, the selection resets to the first suggestion.
    selected_word: String,

    /// Is the popup open? Updated at the end of each frame.
    open: bool,

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

/// Keys pressed while the popup was open, consumed before the [`TextEdit`] sees them.
#[derive(Default)]
struct PopupKeys {
    /// How many times `ArrowDown` was pressed.
    down: usize,

    /// How many times `ArrowUp` was pressed.
    up: usize,

    accept: bool,
    dismiss: bool,
}

impl PopupKeys {
    fn moved_selection(&self) -> bool {
        self.down != self.up
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
/// The popup starts intercepting keys the frame after it opens,
/// so an accept key pressed in the same frame as the popup appears goes to the [`TextEdit`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut text = String::new();
/// let commands = ["/help", "/clear", "/quit"];
/// let output = egui::CompletionPopup::new(ui.make_persistent_id("prompt")).show(
///     ui,
///     egui::TextEdit::singleline(&mut text).hint_text("Type / for commands"),
///     |query| {
///         if !query.is_at_start() {
///             return vec![];
///         }
///         commands
///             .iter()
///             .filter(|command| command.starts_with(query.word))
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
pub struct CompletionPopup<'a> {
    id: Id,
    is_word_boundary: Box<dyn Fn(char) -> bool + 'a>,
    align: RectAlign,
    max_height: f32,
    accept_keys: Vec<Key>,
}

impl<'a> CompletionPopup<'a> {
    /// The `id` is given to the [`TextEdit`], and is also used to store the popup state.
    ///
    /// It must be unique, so if you show several completion popups (e.g. one per tab),
    /// use e.g. [`Ui::make_persistent_id`] rather than a constant [`Id`].
    pub fn new(id: Id) -> Self {
        Self {
            id,
            is_word_boundary: Box::new(char::is_whitespace),
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
    pub fn word_boundary(mut self, is_word_boundary: impl Fn(char) -> bool + 'a) -> Self {
        self.is_word_boundary = Box::new(is_word_boundary);
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
    /// The [`TextEdit`] is given the [`Id`] of this popup.
    /// Its [`TextEdit::event_filter`] is respected, except that Tab and Escape
    /// are also captured while the popup is open.
    ///
    /// `suggest` is called once per frame, after the [`TextEdit`] has handled input.
    pub fn show(
        self,
        ui: &mut Ui,
        text_edit: TextEdit<'_>,
        suggest: impl FnOnce(&CompletionQuery<'_>) -> Vec<Suggestion>,
    ) -> CompletionOutput {
        let Self {
            id,
            is_word_boundary,
            align,
            max_height,
            accept_keys,
        } = self;

        let mut state = CompletionState::load(ui, id);

        // Consume the popup keys before the `TextEdit` sees them.
        // We act on them after the `TextEdit` has run, when we know the current text and cursor.
        let mut keys = PopupKeys::default();
        if state.open {
            ui.input_mut(|input| {
                keys.down = consume_unmodified_key(input, Key::ArrowDown);
                keys.up = consume_unmodified_key(input, Key::ArrowUp);
                for &key in &accept_keys {
                    if 0 < consume_unmodified_key(input, key) {
                        keys.accept = true;
                    }
                }
                keys.dismiss = 0 < consume_unmodified_key(input, Key::Escape);
            });
        }

        let event_filter = text_edit.get_event_filter();
        let text_edit = text_edit.id(id).event_filter(EventFilter {
            // Keep focus on Tab and Escape while the popup is open, so they can act on the popup:
            tab: event_filter.tab || state.open,
            escape: event_filter.escape || state.open,
            ..event_filter
        });
        let (output, text) = text_edit.show_returning_text(ui);
        let response = output.response.response.clone();

        let cursor = output.cursor_range.map_or_else(
            || text.as_str().chars().count(),
            |range| range.primary.index.0,
        );
        let word_range = word_range_before_cursor(text.as_str(), cursor, &*is_word_boundary);
        let word = text.char_range(word_range.clone()).to_owned();

        if keys.dismiss {
            state.dismissed_word = Some(word.clone());
        }

        let mut suggestions = if state.dismissed_word.as_deref() == Some(word.as_str()) {
            vec![]
        } else {
            suggest(&CompletionQuery {
                text: text.as_str(),
                word: &word,
                word_range: word_range.clone(),
            })
        };

        if state.selected_word != word {
            state.selected = 0;
            state.selected_word = word.clone();
        }
        if !suggestions.is_empty() {
            let len = suggestions.len();
            state.selected = (state.selected + keys.down + (len - keys.up % len)) % len;
        }

        let mut accepted_index = (keys.accept && !suggestions.is_empty()).then_some(state.selected);

        // Don't show the popup in the frame we accept, to avoid a one-frame flash of stale suggestions:
        let is_open = response.has_focus() && !suggestions.is_empty() && accepted_index.is_none();

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
                        let pointer_moved = ui.input(|input| input.pointer.is_moving());
                        let mut clicked = None;
                        for (i, suggestion) in suggestions.iter().enumerate() {
                            let is_selected = i == state.selected;
                            let response = ui.add(
                                Button::selectable(is_selected, suggestion.content.clone())
                                    .min_size(vec2(ui.available_width(), 0.0)),
                            );
                            if is_selected && keys.moved_selection() {
                                response.scroll_to_me(None);
                            }
                            if response.hovered() && pointer_moved {
                                state.selected = i;
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

        if clicked.is_some() {
            accepted_index = clicked;
            // Clicking the popup took focus from the text field, so give it back:
            ui.memory_mut(|mem| mem.request_focus(id));
        }

        let accepted = accepted_index
            .filter(|&i| i < suggestions.len())
            .map(|i| suggestions.swap_remove(i));
        if let Some(accepted) = &accepted {
            text.delete_char_range(word_range.clone());
            let mut ccursor = CCursor::new(word_range.start);
            text.insert_text_at(&mut ccursor, &accepted.insert, usize::MAX);

            let mut text_edit_state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
            text_edit_state
                .cursor
                .set_char_range(Some(CCursorRange::one(ccursor)));
            text_edit_state.store(ui.ctx(), id);

            state.dismissed_word = None;
            state.selected = 0;
        }

        state.open = is_open;
        state.store(ui, id);

        CompletionOutput {
            text_edit: output,
            accepted,
            is_open,
        }
    }
}

/// Consume presses of `key` with no modifiers held, returning how many there were.
///
/// Unlike [`InputState::consume_key`], this leaves e.g. Shift+Enter and Shift+ArrowDown alone,
/// so they still reach the [`TextEdit`].
fn consume_unmodified_key(input: &mut InputState, key: Key) -> usize {
    let mut count = 0;
    input.events.retain(|event| {
        let is_match = matches!(
            event,
            Event::Key {
                key: event_key,
                modifiers,
                pressed: true,
                ..
            } if *event_key == key && modifiers.is_none()
        );
        count += usize::from(is_match);
        !is_match
    });
    count
}

/// The char range of the word that ends at the cursor (a char index).
fn word_range_before_cursor(
    text: &str,
    cursor: usize,
    is_word_boundary: &dyn Fn(char) -> bool,
) -> Range<CharIndex> {
    let start = text
        .chars()
        .take(cursor)
        .enumerate()
        .filter(|&(_, c)| is_word_boundary(c))
        .last()
        .map_or(0, |(i, _)| i + 1);
    CharIndex(start)..CharIndex(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_range_before_cursor_works() {
        let range = |text, cursor| {
            let range = word_range_before_cursor(text, cursor, &char::is_whitespace);
            range.start.0..range.end.0
        };
        assert_eq!(range("", 0), 0..0);
        assert_eq!(range("/gr", 3), 0..3);
        assert_eq!(range("hello /gr", 9), 6..9);
        assert_eq!(range("hello /gr", 6), 6..6);
        assert_eq!(range("hello /gr", 8), 6..8);
        assert_eq!(range("héllo wörld", 11), 6..11);

        let range = word_range_before_cursor("a.b", 3, &|c| c == '.');
        assert_eq!(range.start.0..range.end.0, 2..3);
    }
}
