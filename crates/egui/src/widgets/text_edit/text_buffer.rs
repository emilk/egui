use std::{borrow::Cow, ops::Range};

use epaint::text::{
    cursor::{Affinity, ByteCursor},
    TAB_SIZE,
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

    /// Inserts text `text` into this buffer at byte index `byte_index`.
    ///
    /// # Return
    /// Returns how many bytes were successfully inserted
    fn insert_text(&mut self, text: &str, byte_index: usize) -> usize {
        let end_idx = self.replace_range(byte_index..byte_index, text);
        end_idx - byte_index
    }

    /// Reads the given byte range.
    fn byte_range(&self, byte_range: Range<usize>) -> &str {
        &self.as_str()[byte_range]
    }

    /// Replaces a given byte range with `replacement_text`.
    ///
    /// # Return
    /// Returns the end index of the replacement text in the buffer
    fn replace_range(&mut self, range: Range<usize>, replacement_text: &str) -> usize;

    fn delete_byte_range(&mut self, range: Range<usize>) {
        self.replace_range(range, "");
    }

    /// Replaces the range of text specified in the given [`Selection`] with the
    /// string passed in. Returns the new cursor position.
    fn replace_selection(
        &mut self,
        selection: Range<usize>,
        replacement_text: &str,
        char_limit: usize,
    ) -> ByteCursor {
        let mut new_string = replacement_text;
        if char_limit < usize::MAX && {
            // Optimization: one Unicode character can take up to 4 bytes. This
            // gives us an upper bound for the new length of the string. If
            // char_limit exceeds that upper bound, we don't need to count
            // characters (potentially expensive).
            let byte_count_after_removal = self.as_str().len() - (selection.end - selection.start);
            char_limit < (byte_count_after_removal + replacement_text.len()) * 4
        } {
            let current_char_count = self.as_str().chars().count();
            let removed_char_count = self.as_str()[selection.clone()].chars().count();
            // Avoid subtract with overflow panic
            let cutoff = char_limit.saturating_sub(current_char_count - removed_char_count);

            new_string = match new_string.char_indices().nth(cutoff) {
                None => new_string,
                Some((idx, _)) => &new_string[..idx],
            };
        }

        let new_index = self.replace_range(selection, new_string);
        ByteCursor {
            index: new_index,
            affinity: Affinity::Downstream,
        }
    }

    /// Clears all characters in this buffer
    fn clear(&mut self) {
        self.delete_byte_range(0..self.as_str().len());
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

    fn decrease_indentation(&mut self, index: &mut usize) {
        let line_start = find_line_start(self.as_str(), *index);

        let remove_len = if self.as_str().as_bytes().get(line_start) == Some(&b'\t') {
            Some(1)
        } else if self.as_str().as_bytes()[line_start..line_start + TAB_SIZE]
            .iter()
            .all(|c| c == &b' ')
        {
            Some(TAB_SIZE)
        } else {
            None
        };

        if let Some(len) = remove_len {
            self.delete_byte_range(line_start..(line_start + len));
            if *index != line_start {
                *index -= len;
            }
        }
    }

    /// Returns a unique identifier for the implementing type.
    ///
    /// This is useful for downcasting from this trait to the implementing type.
    /// Here is an example usage:
    /// ```
    /// use egui::TextBuffer;
    /// use std::any::TypeId;
    ///
    /// struct ExampleBuffer {}
    ///
    /// impl TextBuffer for ExampleBuffer {
    ///     fn is_mutable(&self) -> bool { unimplemented!() }
    ///     fn as_str(&self) -> &str { unimplemented!() }
    ///     fn insert_text(&mut self, text: &str, char_index: usize) -> usize { unimplemented!() }
    ///     fn delete_char_range(&mut self, char_range: std::ops::Range<usize>) { unimplemented!() }
    ///
    ///     // Implement it like the following:
    ///     fn type_id(&self) -> TypeId {
    ///         TypeId::of::<Self>()
    ///     }
    /// }
    ///
    /// // Example downcast:
    /// pub fn downcast_example(buffer: &dyn TextBuffer) -> Option<&ExampleBuffer> {
    ///     if buffer.type_id() == TypeId::of::<ExampleBuffer>() {
    ///         unsafe { Some(&*(buffer as *const dyn TextBuffer as *const ExampleBuffer)) }
    ///     } else {
    ///         None
    ///     }
    /// }
    /// ```
    fn type_id(&self) -> std::any::TypeId;
}

impl TextBuffer for String {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.as_ref()
    }

    fn replace_range(&mut self, range: Range<usize>, replacement_text: &str) -> usize {
        if range.end - range.start == 0 {
            self.insert_str(range.start, replacement_text);
        } else {
            self.replace_range(range.clone(), replacement_text);
        }
        range.start + replacement_text.len()
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn replace_with(&mut self, text: &str) {
        text.clone_into(self);
    }

    fn take(&mut self) -> String {
        std::mem::take(self)
    }

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
}

impl TextBuffer for Cow<'_, str> {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.as_ref()
    }

    fn replace_range(&mut self, range: Range<usize>, replacement_text: &str) -> usize {
        <String as TextBuffer>::replace_range(self.to_mut(), range, replacement_text)
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

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Cow<'_, str>>()
    }
}

/// Immutable view of a `&str`!
impl TextBuffer for &str {
    fn is_mutable(&self) -> bool {
        false
    }

    fn as_str(&self) -> &str {
        self
    }

    fn replace_range(&mut self, range: Range<usize>, _replacement_text: &str) -> usize {
        // Since we don't modify anything, return the start of the range
        range.start
    }

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<&str>()
    }
}

/// Accepts and returns byte offset
fn find_line_start(text: &str, current_index: usize) -> usize {
    text[..current_index]
        .rfind('\n')
        .map_or(0, |line_ending| line_ending + 1)
}
