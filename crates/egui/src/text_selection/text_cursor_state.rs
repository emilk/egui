//! Text cursor changes/interaction, without modifying the text.

use epaint::text::{ByteIndex, ByteRangeExt as _, CharIndex, Galley, cursor::CCursor};
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
    if ccursor.index == CharIndex::ZERO {
        CCursorRange::two(ccursor, ccursor_next_line(text, ccursor))
    } else {
        let it = text.chars();
        let mut it = it.skip(ccursor.index.0 - 1);
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
                } else if is_linebreak(char_after_cursor) {
                    let min = ccursor_previous_line(text, ccursor);
                    let max = ccursor_next_line(text, ccursor);
                    CCursorRange::two(min, max)
                } else {
                    let max = ccursor_next_line(text, ccursor);
                    CCursorRange::two(ccursor, max)
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
    let num_chars = CharIndex(text.chars().count());
    let reversed: String = text.graphemes(true).rev().collect();
    let boundary = next_word_boundary_char_index(&reversed, num_chars - ccursor.index);
    CCursor {
        index: num_chars - boundary.min(num_chars),
        prefer_next_row: true,
    }
}

fn ccursor_previous_line(text: &str, ccursor: CCursor) -> CCursor {
    let num_chars = CharIndex(text.chars().count());
    let boundary = next_line_boundary_char_index(text.chars().rev(), num_chars - ccursor.index);
    CCursor {
        index: num_chars - boundary,
        prefer_next_row: true,
    }
}

fn next_word_boundary_char_index(text: &str, cursor_ci: CharIndex) -> CharIndex {
    let mut current_char_idx = CharIndex::ZERO;

    for (_word_byte_index, word) in text.split_word_bound_indices() {
        let word_ci = current_char_idx;

        // We consider `.` a word boundary.
        // At least that's how Mac works when navigating something like `www.example.com`.
        let mut word_char_count = 0;
        for chr in word.chars() {
            let dot_ci = word_ci + word_char_count;
            if chr == '.' && cursor_ci < dot_ci {
                return dot_ci;
            }
            word_char_count += 1;
        }

        // Splitting considers contiguous whitespace as one word, such words must be skipped,
        // this handles cases for example ' abc' (a space and a word), the cursor is at the beginning
        // (before space) - this jumps at the end of 'abc' (this is consistent with text editors
        // or browsers)
        if cursor_ci < word_ci && !all_word_chars(word) {
            return word_ci;
        }

        current_char_idx += word_char_count;
    }

    current_char_idx
}

fn all_word_chars(text: &str) -> bool {
    text.chars().all(is_word_char)
}

fn next_line_boundary_char_index(
    it: impl Iterator<Item = char>,
    mut index: CharIndex,
) -> CharIndex {
    let mut it = it.skip(index.0);
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
    let text_before = (ByteIndex::ZERO..byte_idx).slice(text);

    if let Some(last_newline_byte) = text_before.rfind('\n') {
        let char_idx = char_index_from_byte_index(text, ByteIndex(last_newline_byte + 1));
        CCursor::new(char_idx)
    } else {
        CCursor::new(0)
    }
}

pub fn byte_index_from_char_index(s: &str, char_index: CharIndex) -> ByteIndex {
    for (ci, (bi, _)) in s.char_indices().enumerate() {
        if ci == char_index.0 {
            return ByteIndex(bi);
        }
    }
    ByteIndex(s.len())
}

pub fn char_index_from_byte_index(input: &str, byte_index: ByteIndex) -> CharIndex {
    for (ci, (bi, _)) in input.char_indices().enumerate() {
        if bi == byte_index.0 {
            return CharIndex(ci);
        }
    }

    // `byte_index` is at or past the end of the string (or not on a char boundary):
    // return the total number of characters.
    CharIndex(input.chars().count())
}

pub fn slice_char_range(s: &str, char_range: std::ops::Range<CharIndex>) -> &str {
    assert!(
        char_range.start <= char_range.end,
        "Invalid range, start must be less than end, but start = {}, end = {}",
        char_range.start,
        char_range.end
    );
    let start_byte = byte_index_from_char_index(s, char_range.start);
    let end_byte = byte_index_from_char_index(s, char_range.end);
    (start_byte..end_byte).slice(s)
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
    use super::*;

    #[test]
    fn test_next_word_boundary_char_index() {
        // ASCII only
        let text = "abc d3f g_h i-j";
        assert_eq!(next_word_boundary_char_index(text, CharIndex(1)).0, 3);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(3)).0, 7);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(9)).0, 11);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(12)).0, 13);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(13)).0, 15);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(15)).0, 15);

        assert_eq!(next_word_boundary_char_index("", CharIndex(0)).0, 0);
        assert_eq!(next_word_boundary_char_index("", CharIndex(1)).0, 0);

        // ASCII only
        let text = "abc.def.ghi";
        assert_eq!(next_word_boundary_char_index(text, CharIndex(1)).0, 3);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(3)).0, 7);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(7)).0, 11);

        // Unicode graphemes, some of which consist of multiple Unicode characters,
        // !!! Unicode character is not always what is tranditionally considered a character,
        // the values below are correct despite not seeming that way on the first look,
        // handling of and around emojis is kind of weird and is not consistent across
        // text editors and browsers
        let text = "❤️👍 skvělá knihovna 👍❤️";
        assert_eq!(next_word_boundary_char_index(text, CharIndex(0)).0, 2);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(2)).0, 3); // this does not skip the space between thumbs-up and 'skvělá'
        assert_eq!(next_word_boundary_char_index(text, CharIndex(6)).0, 10);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(9)).0, 10);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(12)).0, 19);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(15)).0, 19);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(19)).0, 20);
        assert_eq!(next_word_boundary_char_index(text, CharIndex(20)).0, 21);
    }

    #[test]
    fn test_previous_word() {
        let text = "abc def ghi";
        assert_eq!(ccursor_previous_word(text, CCursor::new(7)).index.0, 4);
        assert_eq!(ccursor_previous_word(text, CCursor::new(5)).index.0, 4);
        assert_eq!(ccursor_previous_word(text, CCursor::new(4)).index.0, 0);
        assert_eq!(ccursor_previous_word(text, CCursor::new(0)).index.0, 0);
    }

    #[test]
    fn test_next_word() {
        let text = "abc def ghi";
        assert_eq!(ccursor_next_word(text, CCursor::new(0)).index.0, 3);
        assert_eq!(ccursor_next_word(text, CCursor::new(3)).index.0, 7);
        assert_eq!(ccursor_next_word(text, CCursor::new(7)).index.0, 11);
        assert_eq!(ccursor_next_word(text, CCursor::new(11)).index.0, 11);
    }

    #[test]
    fn test_index_conversion_roundtrip() {
        // "é" is 2 bytes, "👍" is 4 bytes.
        let text = "aé👍b";
        let char_count = text.chars().count(); // 4
        assert_eq!(char_count, 4);

        // char -> byte, including the end index
        assert_eq!(byte_index_from_char_index(text, CharIndex(0)).0, 0);
        assert_eq!(byte_index_from_char_index(text, CharIndex(1)).0, 1);
        assert_eq!(byte_index_from_char_index(text, CharIndex(2)).0, 3);
        assert_eq!(byte_index_from_char_index(text, CharIndex(3)).0, 7);
        assert_eq!(byte_index_from_char_index(text, CharIndex(4)).0, 8);
        // Past the end clamps to the byte length:
        assert_eq!(
            byte_index_from_char_index(text, CharIndex(99)).0,
            text.len()
        );

        // byte -> char, including the end index
        assert_eq!(char_index_from_byte_index(text, ByteIndex(0)).0, 0);
        assert_eq!(char_index_from_byte_index(text, ByteIndex(1)).0, 1);
        assert_eq!(char_index_from_byte_index(text, ByteIndex(3)).0, 2);
        assert_eq!(char_index_from_byte_index(text, ByteIndex(7)).0, 3);
        // The end byte index must map to the character count, not to some byte offset:
        assert_eq!(char_index_from_byte_index(text, ByteIndex(text.len())).0, 4);
        // Past the end clamps to the character count:
        assert_eq!(char_index_from_byte_index(text, ByteIndex(99)).0, 4);

        // Empty string:
        assert_eq!(byte_index_from_char_index("", CharIndex(0)).0, 0);
        assert_eq!(char_index_from_byte_index("", ByteIndex(0)).0, 0);
    }

    #[test]
    fn test_select_word_at() {
        // CCursorRange::two(min, max) sets primary=max, secondary=min
        let text = "hello world";
        let range = select_word_at(text, CCursor::new(2));
        let (lo, hi) = (
            range.primary.index.min(range.secondary.index),
            range.primary.index.max(range.secondary.index),
        );
        assert_eq!(lo.0, 0);
        assert_eq!(hi.0, 5);

        let range = select_word_at(text, CCursor::new(8));
        let (lo, hi) = (
            range.primary.index.min(range.secondary.index),
            range.primary.index.max(range.secondary.index),
        );
        assert_eq!(lo.0, 6);
        assert_eq!(hi.0, 11);
    }

    #[test]
    fn test_word_boundary_large_text_performance() {
        // Before the O(n²) → O(n) fix, this would take minutes on large text.
        let large_text = "word ".repeat(200_000); // ~1MB
        let len = large_text.chars().count();

        let start = std::time::Instant::now();

        let next = ccursor_next_word(&large_text, CCursor::new(len - 10));
        assert!(next.index.0 <= len);

        let prev = ccursor_previous_word(&large_text, CCursor::new(len - 10));
        assert!(prev.index.0 < len);

        let range = select_word_at(&large_text, CCursor::new(len - 3));
        let lo = range.primary.index.min(range.secondary.index);
        let hi = range.primary.index.max(range.secondary.index);
        assert!(lo < hi, "Expected a non-empty word selection");

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_secs() < 5,
            "Word boundary operations on 1MB text took {elapsed:?}, expected < 5s"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_previous_word_graphemes() {
        let cases = [
            ("", 0, 0),
            ("hello", 0, 0),
            ("hello", "hello".chars().count(), 0),
            ("hello world", 6, 0),
            ("hello world", 8, 6),
            ("hello world", "hello world".chars().count(), 6),
            ("hello world   ", "hello world   ".chars().count(), 6),
            ("hello   world", "hello   world".chars().count(), 8),
            ("   ", "   ".chars().count(), 0),
            ("hello, world", "hello, world".chars().count(), 7),
            ("www.example.com", "www.example.com".chars().count(), 12),
            ("안녕! 😊 세상", 8, 6),
            ("❤️👍 skvělá knihovna 👍❤️", 18, 11),
            (
                "a e\u{301} b",
                "a e\u{301} b".chars().count(),
                "a e\u{301} ".chars().count(),
            ),
            (
                "hi 🙂 world",
                "hi 🙂 world".chars().count(),
                "hi 🙂 ".chars().count(),
            ),
            (
                "hi 👨‍👩‍👧‍👦 world",
                "hi 👨‍👩‍👧‍👦 world".chars().count(),
                "hi 👨‍👩‍👧‍👦 ".chars().count(),
            ),
        ];

        for (text, cursor, expected) in cases {
            let result = ccursor_previous_word(text, CCursor::new(cursor));
            assert_eq!(
                result.index.0, expected,
                "text={text:?}, cursor={cursor}, got={}, expected={expected}",
                result.index.0
            );
        }
    }
}
