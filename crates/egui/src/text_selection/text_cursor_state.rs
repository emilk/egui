//! Text cursor changes/interaction, without modifying the text.

use epaint::text::{Galley, cursor::CCursor};
use unicode_segmentation::UnicodeSegmentation as _;

use crate::{NumExt as _, Rect, Response, Ui, epaint};

use super::CCursorRange;

/// The state of a text cursor selection.
///
/// Used for [`crate::TextEdit`] and [`crate::Label`].
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextCursorState {
    ccursor_range: Option<CCursorRange>,
}

impl From<CCursorRange> for TextCursorState {
    fn from(ccursor_range: CCursorRange) -> Self {
        Self {
            ccursor_range: Some(ccursor_range),
        }
    }
}

impl TextCursorState {
    pub fn is_empty(&self) -> bool {
        self.ccursor_range.is_none()
    }

    /// The currently selected range of characters.
    pub fn char_range(&self) -> Option<CCursorRange> {
        self.ccursor_range
    }

    /// The currently selected range of characters, clamped within the character
    /// range of the given [`Galley`].
    pub fn range(&self, galley: &Galley) -> Option<CCursorRange> {
        self.ccursor_range.map(|mut range| {
            range.primary = galley.clamp_cursor(&range.primary);
            range.secondary = galley.clamp_cursor(&range.secondary);
            range
        })
    }

    /// Sets the currently selected range of characters.
    pub fn set_char_range(&mut self, ccursor_range: Option<CCursorRange>) {
        self.ccursor_range = ccursor_range;
    }
}

impl TextCursorState {
    /// Handle clicking and/or dragging text.
    ///
    /// Returns `true` if there was interaction.
    pub fn pointer_interaction(
        &mut self,
        ui: &Ui,
        response: &Response,
        cursor_at_pointer: CCursor,
        galley: &Galley,
        is_being_dragged: bool,
    ) -> bool {
        let text = galley.text();

        if response.double_clicked() {
            // Select word:
            let ccursor_range = select_word_at(text, cursor_at_pointer);
            self.set_char_range(Some(ccursor_range));
            true
        } else if response.triple_clicked() {
            // Select line:
            let ccursor_range = select_line_at(text, cursor_at_pointer);
            self.set_char_range(Some(ccursor_range));
            true
        } else if response.sense.senses_drag() {
            if response.hovered() && ui.input(|i| i.pointer.any_pressed()) {
                // The start of a drag (or a click).
                if ui.input(|i| i.modifiers.shift) {
                    if let Some(mut cursor_range) = self.range(galley) {
                        cursor_range.primary = cursor_at_pointer;
                        self.set_char_range(Some(cursor_range));
                    } else {
                        self.set_char_range(Some(CCursorRange::one(cursor_at_pointer)));
                    }
                } else {
                    self.set_char_range(Some(CCursorRange::one(cursor_at_pointer)));
                }
                true
            } else if is_being_dragged {
                // Drag to select text:
                if let Some(mut cursor_range) = self.range(galley) {
                    cursor_range.primary = cursor_at_pointer;
                    self.set_char_range(Some(cursor_range));
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

fn select_word_at(text: &str, ccursor: CCursor) -> CCursorRange {
    if text.is_empty() {
        return CCursorRange::one(ccursor);
    }

    let line_start = find_line_start(text, ccursor);
    let line_end = ccursor_next_line(text, line_start);

    let line_range = line_start.index..line_end.index;
    let current_line_text = slice_char_range(text, line_range.clone());

    let relative_idx = ccursor.index - line_start.index;
    let relative_ccursor = CCursor::new(relative_idx);

    let min = ccursor_previous_word(current_line_text, relative_ccursor);
    let max = ccursor_next_word(current_line_text, relative_ccursor);

    CCursorRange::two(
        CCursor::new(line_start.index + min.index),
        CCursor::new(line_start.index + max.index),
    )
}

fn select_line_at(text: &str, ccursor: CCursor) -> CCursorRange {
    if ccursor.index == 0 {
        CCursorRange::two(ccursor, ccursor_next_line(text, ccursor))
    } else {
        let it = text.chars();
        let mut it = it.skip(ccursor.index - 1);
        if let Some(char_before_cursor) = it.next() {
            if let Some(char_after_cursor) = it.next() {
                if (!is_linebreak(char_before_cursor)) && (!is_linebreak(char_after_cursor)) {
                    let min = ccursor_previous_line(text, ccursor + 1);
                    let max = ccursor_next_line(text, min);
                    CCursorRange::two(min, max)
                } else if !is_linebreak(char_before_cursor) {
                    let min = ccursor_previous_line(text, ccursor);
                    let max = ccursor_next_line(text, min);
                    CCursorRange::two(min, max)
                } else if !is_linebreak(char_after_cursor) {
                    let max = ccursor_next_line(text, ccursor);
                    CCursorRange::two(ccursor, max)
                } else {
                    let min = ccursor_previous_line(text, ccursor);
                    let max = ccursor_next_line(text, ccursor);
                    CCursorRange::two(min, max)
                }
            } else {
                let min = ccursor_previous_line(text, ccursor);
                CCursorRange::two(min, ccursor)
            }
        } else {
            let max = ccursor_next_line(text, ccursor);
            CCursorRange::two(ccursor, max)
        }
    }
}

pub fn ccursor_next_word(text: &str, ccursor: CCursor) -> CCursor {
    CCursor {
        index: next_word_boundary_char_index(text, ccursor.index),
        prefer_next_row: false,
    }
}

fn ccursor_next_line(text: &str, ccursor: CCursor) -> CCursor {
    CCursor {
        index: next_line_boundary_char_index(text.chars(), ccursor.index),
        prefer_next_row: false,
    }
}

pub fn ccursor_previous_word(text: &str, ccursor: CCursor) -> CCursor {
    if ccursor.index == 0 {
        return ccursor;
    }
    let byte_idx = byte_index_from_char_index(text, ccursor.index);
    let text_before = &text[..byte_idx];

    if let Some((byte_offset, _word)) = text_before.split_word_bound_indices().next_back() {
        return CCursor::new(char_index_from_byte_index(text, byte_offset));
    }
    CCursor::new(0)
}

fn ccursor_previous_line(text: &str, ccursor: CCursor) -> CCursor {
    let num_chars = text.chars().count();
    CCursor {
        index: num_chars
            - next_line_boundary_char_index(text.chars().rev(), num_chars - ccursor.index),
        prefer_next_row: true,
    }
}

fn next_word_boundary_char_index(text: &str, cursor_ci: usize) -> usize {
    let mut current_char_idx = 0;
    let mut last_byte_idx = 0;

    for (word_byte_index, word) in text.split_word_bound_indices() {
        current_char_idx += text[last_byte_idx..word_byte_index].chars().count();
        last_byte_idx = word_byte_index;

        let word_ci = current_char_idx;

        // We consider `.` a word boundary.
        // At least that's how Mac works when navigating something like `www.example.com`.
        for (dot_ci_offset, chr) in word.chars().enumerate() {
            let dot_ci = word_ci + dot_ci_offset;
            if chr == '.' && cursor_ci < dot_ci {
                return dot_ci;
            }
        }

        // Splitting considers contiguous whitespace as one word, such words must be skipped,
        // this handles cases for example ' abc' (a space and a word), the cursor is at the beginning
        // (before space) - this jumps at the end of 'abc' (this is consistent with text editors
        // or browsers)
        if cursor_ci < word_ci && !all_word_chars(word) {
            return word_ci;
        }
    }

    current_char_idx + text[last_byte_idx..].chars().count()
}

fn all_word_chars(text: &str) -> bool {
    text.chars().all(is_word_char)
}

fn next_line_boundary_char_index(it: impl Iterator<Item = char>, mut index: usize) -> usize {
    let mut it = it.skip(index);
    if let Some(_first) = it.next() {
        index += 1;

        if let Some(second) = it.next() {
            index += 1;
            for next in it {
                if is_linebreak(next) != is_linebreak(second) {
                    break;
                }
                index += 1;
            }
        }
    }
    index
}

pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_linebreak(c: char) -> bool {
    c == '\r' || c == '\n'
}

/// Accepts and returns character offset (NOT byte offset!).
pub fn find_line_start(text: &str, current_index: CCursor) -> CCursor {
    let byte_idx = byte_index_from_char_index(text, current_index.index);
    let text_before = &text[..byte_idx];

    if let Some(last_newline_byte) = text_before.rfind('\n') {
        let char_idx = char_index_from_byte_index(text, last_newline_byte + 1);
        CCursor::new(char_idx)
    } else {
        CCursor::new(0)
    }
}

pub fn byte_index_from_char_index(s: &str, char_index: usize) -> usize {
    for (ci, (bi, _)) in s.char_indices().enumerate() {
        if ci == char_index {
            return bi;
        }
    }
    s.len()
}

pub fn char_index_from_byte_index(input: &str, byte_index: usize) -> usize {
    for (ci, (bi, _)) in input.char_indices().enumerate() {
        if bi == byte_index {
            return ci;
        }
    }

    input.char_indices().last().map_or(0, |(i, _)| i + 1)
}

pub fn slice_char_range(s: &str, char_range: std::ops::Range<usize>) -> &str {
    assert!(
        char_range.start <= char_range.end,
        "Invalid range, start must be less than end, but start = {}, end = {}",
        char_range.start,
        char_range.end
    );
    let start_byte = byte_index_from_char_index(s, char_range.start);
    let end_byte = byte_index_from_char_index(s, char_range.end);
    &s[start_byte..end_byte]
}

/// The thin rectangle of one end of the selection, e.g. the primary cursor, in local galley coordinates.
pub fn cursor_rect(galley: &Galley, cursor: &CCursor, row_height: f32) -> Rect {
    let mut cursor_pos = galley.pos_from_cursor(*cursor);

    // Handle completely empty galleys
    cursor_pos.max.y = cursor_pos.max.y.at_least(cursor_pos.min.y + row_height);

    cursor_pos = cursor_pos.expand(1.5); // slightly above/below row

    cursor_pos
}

#[cfg(test)]
mod test {
    use crate::text_selection::text_cursor_state::next_word_boundary_char_index;

    #[test]
    fn test_next_word_boundary_char_index() {
        // ASCII only
        let text = "abc d3f g_h i-j";
        assert_eq!(next_word_boundary_char_index(text, 1), 3);
        assert_eq!(next_word_boundary_char_index(text, 3), 7);
        assert_eq!(next_word_boundary_char_index(text, 9), 11);
        assert_eq!(next_word_boundary_char_index(text, 12), 13);
        assert_eq!(next_word_boundary_char_index(text, 13), 15);
        assert_eq!(next_word_boundary_char_index(text, 15), 15);

        assert_eq!(next_word_boundary_char_index("", 0), 0);
        assert_eq!(next_word_boundary_char_index("", 1), 0);

        // ASCII only
        let text = "abc.def.ghi";
        assert_eq!(next_word_boundary_char_index(text, 1), 3);
        assert_eq!(next_word_boundary_char_index(text, 3), 7);
        assert_eq!(next_word_boundary_char_index(text, 7), 11);

        // Unicode graphemes, some of which consist of multiple Unicode characters,
        // !!! Unicode character is not always what is tranditionally considered a character,
        // the values below are correct despite not seeming that way on the first look,
        // handling of and around emojis is kind of weird and is not consistent across
        // text editors and browsers
        let text = "❤️👍 skvělá knihovna 👍❤️";
        assert_eq!(next_word_boundary_char_index(text, 0), 2);
        assert_eq!(next_word_boundary_char_index(text, 2), 3); // this does not skip the space between thumbs-up and 'skvělá'
        assert_eq!(next_word_boundary_char_index(text, 6), 10);
        assert_eq!(next_word_boundary_char_index(text, 9), 10);
        assert_eq!(next_word_boundary_char_index(text, 12), 19);
        assert_eq!(next_word_boundary_char_index(text, 15), 19);
        assert_eq!(next_word_boundary_char_index(text, 19), 20);
        assert_eq!(next_word_boundary_char_index(text, 20), 21);
    }
}
