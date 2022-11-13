use std::ops::Range;

/// Trait constraining what types [`crate::TextEdit`] may use as
/// an underlying buffer.
///
/// Most likely you will use a [`String`] which implements [`TextBuffer`].
pub trait TextBuffer {
    /// Can this text be edited?
    fn is_mutable(&self) -> bool;

    /// Returns this buffer as a `str`.
    fn as_str(&self) -> &str;

    /// Reads the given character range.
    fn char_range(&self, char_range: Range<usize>) -> &str {
        assert!(char_range.start <= char_range.end);
        let start_byte = self.byte_index_from_char_index(char_range.start);
        let end_byte = self.byte_index_from_char_index(char_range.end);
        &self.as_str()[start_byte..end_byte]
    }

    fn byte_index_from_char_index(&self, char_index: usize) -> usize {
        byte_index_from_char_index(self.as_str(), char_index)
    }

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

    /// Clears all characters in this buffer
    fn clear(&mut self) {
        self.delete_char_range(0..self.as_str().len());
    }

    /// Replaces all contents of this string with `text`
    fn replace(&mut self, text: &str) {
        self.clear();
        self.insert_text(text, 0);
    }

    /// Clears all characters in this buffer and returns a string of the contents.
    fn take(&mut self) -> String {
        let s = self.as_str().to_owned();
        self.clear();
        s
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
        let byte_idx = self.byte_index_from_char_index(char_index);

        // Then insert the string
        self.insert_str(byte_idx, text);

        text.chars().count()
    }

    fn delete_char_range(&mut self, char_range: Range<usize>) {
        assert!(char_range.start <= char_range.end);

        // Get both byte indices
        let byte_start = self.byte_index_from_char_index(char_range.start);
        let byte_end = self.byte_index_from_char_index(char_range.end);

        // Then drain all characters within this range
        self.drain(byte_start..byte_end);
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn replace(&mut self, text: &str) {
        *self = text.to_owned();
    }

    fn take(&mut self) -> String {
        std::mem::take(self)
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

fn byte_index_from_char_index(s: &str, char_index: usize) -> usize {
    for (ci, (bi, _)) in s.char_indices().enumerate() {
        if ci == char_index {
            return bi;
        }
    }
    s.len()
}
