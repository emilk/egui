use std::{borrow::Cow, ops::Range};

use epaint::{
    text::{
        cursor::{CCursor, PCursor},
        TAB_SIZE,
    },
    Galley,
};

use crate::text_selection::{
    text_cursor_state::{
        byte_index_from_char_index, ccursor_next_word, ccursor_previous_word, find_line_start,
        slice_char_range,
    },
    CursorRange,
};

/// Trait constraining what types [`crate::TextEdit`] may use as
/// an underlying buffer.
///
/// Most likely you will use a [`String`] which implements [`TextBuffer`].
pub trait TextBuffer {
    /// Can this text be edited?
    fn is_mutable(&self) -> bool;

    /// Returns this buffer as a `str`.
    fn as_str(&self) -> &str;

    /// Inserts text `text` into this buffer at character index `char_index`.
    ///
    /// # Notes
    /// `char_index` is a *character index*, not a byte index.
    ///
    /// # Return
    /// Returns how many *characters* were successfully inserted
    fn insert_text(&mut self, text: &str, char_index: usize) -> usize;

    /// Deletes a range of text `char_range` from this buffer.
    ///
    /// # Notes
    /// `char_range` is a *character range*, not a byte range.
    fn delete_char_range(&mut self, char_range: Range<usize>);

    /// Reads the given character range.
    fn char_range(&self, char_range: Range<usize>) -> &str {
        slice_char_range(self.as_str(), char_range)
    }

    fn byte_index_from_char_index(&self, char_index: usize) -> usize {
        byte_index_from_char_index(self.as_str(), char_index)
    }

    /// Clears all characters in this buffer
    fn clear(&mut self) {
        self.delete_char_range(0..self.as_str().len());
    }

    /// Replaces all contents of this string with `text`
    fn replace_with(&mut self, text: &str) {
        self.clear();
        self.insert_text(text, 0);
    }

    /// Clears all characters in this buffer and returns a string of the contents.
    fn take(&mut self) -> String {
        let s = self.as_str().to_owned();
        self.clear();
        s
    }

    fn insert_text_at(&mut self, ccursor: &mut CCursor, text_to_insert: &str, char_limit: usize) {
        if char_limit < usize::MAX {
            let mut new_string = text_to_insert;
            // Avoid subtract with overflow panic
            let cutoff = char_limit.saturating_sub(self.as_str().chars().count());

            new_string = match new_string.char_indices().nth(cutoff) {
                None => new_string,
                Some((idx, _)) => &new_string[..idx],
            };

            ccursor.index += self.insert_text(new_string, ccursor.index);
        } else {
            ccursor.index += self.insert_text(text_to_insert, ccursor.index);
        }
    }

    fn decrease_indentation(&mut self, ccursor: &mut CCursor) {
        let line_start = find_line_start(self.as_str(), *ccursor);

        let remove_len = if self.as_str().chars().nth(line_start.index) == Some('\t') {
            Some(1)
        } else if self
            .as_str()
            .chars()
            .skip(line_start.index)
            .take(TAB_SIZE)
            .all(|c| c == ' ')
        {
            Some(TAB_SIZE)
        } else {
            None
        };

        if let Some(len) = remove_len {
            self.delete_char_range(line_start.index..(line_start.index + len));
            if *ccursor != line_start {
                *ccursor -= len;
            }
        }
    }

    fn delete_selected(&mut self, cursor_range: &CursorRange) -> CCursor {
        let [min, max] = cursor_range.sorted_cursors();
        self.delete_selected_ccursor_range([min.ccursor, max.ccursor])
    }

    fn delete_selected_ccursor_range(&mut self, [min, max]: [CCursor; 2]) -> CCursor {
        self.delete_char_range(min.index..max.index);
        CCursor {
            index: min.index,
            prefer_next_row: true,
        }
    }

    fn delete_previous_char(&mut self, ccursor: CCursor) -> CCursor {
        if ccursor.index > 0 {
            let max_ccursor = ccursor;
            let min_ccursor = max_ccursor - 1;
            self.delete_selected_ccursor_range([min_ccursor, max_ccursor])
        } else {
            ccursor
        }
    }

    fn delete_next_char(&mut self, ccursor: CCursor) -> CCursor {
        self.delete_selected_ccursor_range([ccursor, ccursor + 1])
    }

    fn delete_previous_word(&mut self, max_ccursor: CCursor) -> CCursor {
        let min_ccursor = ccursor_previous_word(self.as_str(), max_ccursor);
        self.delete_selected_ccursor_range([min_ccursor, max_ccursor])
    }

    fn delete_next_word(&mut self, min_ccursor: CCursor) -> CCursor {
        let max_ccursor = ccursor_next_word(self.as_str(), min_ccursor);
        self.delete_selected_ccursor_range([min_ccursor, max_ccursor])
    }

    fn delete_paragraph_before_cursor(
        &mut self,
        galley: &Galley,
        cursor_range: &CursorRange,
    ) -> CCursor {
        let [min, max] = cursor_range.sorted_cursors();
        let min = galley.from_pcursor(PCursor {
            paragraph: min.pcursor.paragraph,
            offset: 0,
            prefer_next_row: true,
        });
        if min.ccursor == max.ccursor {
            self.delete_previous_char(min.ccursor)
        } else {
            self.delete_selected(&CursorRange::two(min, max))
        }
    }

    fn delete_paragraph_after_cursor(
        &mut self,
        galley: &Galley,
        cursor_range: &CursorRange,
    ) -> CCursor {
        let [min, max] = cursor_range.sorted_cursors();
        let max = galley.from_pcursor(PCursor {
            paragraph: max.pcursor.paragraph,
            offset: usize::MAX, // end of paragraph
            prefer_next_row: false,
        });
        if min.ccursor == max.ccursor {
            self.delete_next_char(min.ccursor)
        } else {
            self.delete_selected(&CursorRange::two(min, max))
        }
    }
}

impl TextBuffer for String {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.as_ref()
    }

    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        // Get the byte index from the character index
        let byte_idx = byte_index_from_char_index(self.as_str(), char_index);

        // Then insert the string
        self.insert_str(byte_idx, text);

        text.chars().count()
    }

    fn delete_char_range(&mut self, char_range: Range<usize>) {
        assert!(char_range.start <= char_range.end);

        // Get both byte indices
        let byte_start = byte_index_from_char_index(self.as_str(), char_range.start);
        let byte_end = byte_index_from_char_index(self.as_str(), char_range.end);

        // Then drain all characters within this range
        self.drain(byte_start..byte_end);
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn replace_with(&mut self, text: &str) {
        *self = text.to_owned();
    }

    fn take(&mut self) -> String {
        std::mem::take(self)
    }
}

impl<'a> TextBuffer for Cow<'a, str> {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.as_ref()
    }

    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        <String as TextBuffer>::insert_text(self.to_mut(), text, char_index)
    }

    fn delete_char_range(&mut self, char_range: Range<usize>) {
        <String as TextBuffer>::delete_char_range(self.to_mut(), char_range);
    }

    fn clear(&mut self) {
        <String as TextBuffer>::clear(self.to_mut());
    }

    fn replace_with(&mut self, text: &str) {
        *self = Cow::Owned(text.to_owned());
    }

    fn take(&mut self) -> String {
        std::mem::take(self).into_owned()
    }
}

/// Immutable view of a `&str`!
impl<'a> TextBuffer for &'a str {
    fn is_mutable(&self) -> bool {
        false
    }

    fn as_str(&self) -> &str {
        self
    }

    fn insert_text(&mut self, _text: &str, _ch_idx: usize) -> usize {
        0
    }

    fn delete_char_range(&mut self, _ch_range: Range<usize>) {}
}
