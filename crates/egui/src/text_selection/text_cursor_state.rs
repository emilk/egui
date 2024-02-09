//! Text cursor changes/interaction, without modifying the text.

use epaint::text::{cursor::*, Galley};

use crate::*;

use super::{CCursorRange, CursorRange};

/// The state of a text cursor selection.
///
/// Used for [`crate::TextEdit`] and [`crate::Label`].
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextCursorState {
    cursor_range: Option<CursorRange>,

    /// This is what is easiest to work with when editing text,
    /// so users are more likely to read/write this.
    ccursor_range: Option<CCursorRange>,
}

impl From<CursorRange> for TextCursorState {
    fn from(cursor_range: CursorRange) -> Self {
        Self {
            cursor_range: Some(cursor_range),
            ccursor_range: Some(CCursorRange {
                primary: cursor_range.primary.ccursor,
                secondary: cursor_range.secondary.ccursor,
            }),
        }
    }
}

impl From<CCursorRange> for TextCursorState {
    fn from(ccursor_range: CCursorRange) -> Self {
        Self {
            cursor_range: None,
            ccursor_range: Some(ccursor_range),
        }
    }
}

impl TextCursorState {
    pub fn is_empty(&self) -> bool {
        self.cursor_range.is_none() && self.ccursor_range.is_none()
    }

    /// The the currently selected range of characters.
    pub fn char_range(&self) -> Option<CCursorRange> {
        self.ccursor_range.or_else(|| {
            self.cursor_range
                .map(|cursor_range| cursor_range.as_ccursor_range())
        })
    }

    pub fn range(&self, galley: &Galley) -> Option<CursorRange> {
        self.cursor_range
            .map(|cursor_range| {
                // We only use the PCursor (paragraph number, and character offset within that paragraph).
                // This is so that if we resize the [`TextEdit`] region, and text wrapping changes,
                // we keep the same byte character offset from the beginning of the text,
                // even though the number of rows changes
                // (each paragraph can be several rows, due to word wrapping).
                // The column (character offset) should be able to extend beyond the last word so that we can
                // go down and still end up on the same column when we return.
                CursorRange {
                    primary: galley.from_pcursor(cursor_range.primary.pcursor),
                    secondary: galley.from_pcursor(cursor_range.secondary.pcursor),
                }
            })
            .or_else(|| {
                self.ccursor_range.map(|ccursor_range| CursorRange {
                    primary: galley.from_ccursor(ccursor_range.primary),
                    secondary: galley.from_ccursor(ccursor_range.secondary),
                })
            })
    }

    /// Sets the currently selected range of characters.
    pub fn set_char_range(&mut self, ccursor_range: Option<CCursorRange>) {
        self.cursor_range = None;
        self.ccursor_range = ccursor_range;
    }

    pub fn set_range(&mut self, cursor_range: Option<CursorRange>) {
        self.cursor_range = cursor_range;
        self.ccursor_range = None;
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
        cursor_at_pointer: Cursor,
        galley: &Galley,
        is_being_dragged: bool,
    ) -> bool {
        let text = galley.text();

        if response.double_clicked() {
            // Select word:
            let ccursor_range = select_word_at(text, cursor_at_pointer.ccursor);
            self.set_range(Some(CursorRange {
                primary: galley.from_ccursor(ccursor_range.primary),
                secondary: galley.from_ccursor(ccursor_range.secondary),
            }));
            true
        } else if response.triple_clicked() {
            // Select line:
            let ccursor_range = select_line_at(text, cursor_at_pointer.ccursor);
            self.set_range(Some(CursorRange {
                primary: galley.from_ccursor(ccursor_range.primary),
                secondary: galley.from_ccursor(ccursor_range.secondary),
            }));
            true
        } else if response.sense.drag {
            if response.hovered() && ui.input(|i| i.pointer.any_pressed()) {
                // The start of a drag (or a click).
                if ui.input(|i| i.modifiers.shift) {
                    if let Some(mut cursor_range) = self.range(galley) {
                        cursor_range.primary = cursor_at_pointer;
                        self.set_range(Some(cursor_range));
                    } else {
                        self.set_range(Some(CursorRange::one(cursor_at_pointer)));
                    }
                } else {
                    self.set_range(Some(CursorRange::one(cursor_at_pointer)));
                }
                true
            } else if is_being_dragged {
                // Drag to select text:
                if let Some(mut cursor_range) = self.range(galley) {
                    cursor_range.primary = cursor_at_pointer;
                    self.set_range(Some(cursor_range));
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
    if ccursor.index == 0 {
        CCursorRange::two(ccursor, ccursor_next_word(text, ccursor))
    } else {
        let it = text.chars();
        let mut it = it.skip(ccursor.index - 1);
        if let Some(char_before_cursor) = it.next() {
            if let Some(char_after_cursor) = it.next() {
                if is_word_char(char_before_cursor) && is_word_char(char_after_cursor) {
                    let min = ccursor_previous_word(text, ccursor + 1);
                    let max = ccursor_next_word(text, min);
                    CCursorRange::two(min, max)
                } else if is_word_char(char_before_cursor) {
                    let min = ccursor_previous_word(text, ccursor);
                    let max = ccursor_next_word(text, min);
                    CCursorRange::two(min, max)
                } else if is_word_char(char_after_cursor) {
                    let max = ccursor_next_word(text, ccursor);
                    CCursorRange::two(ccursor, max)
                } else {
                    let min = ccursor_previous_word(text, ccursor);
                    let max = ccursor_next_word(text, ccursor);
                    CCursorRange::two(min, max)
                }
            } else {
                let min = ccursor_previous_word(text, ccursor);
                CCursorRange::two(min, ccursor)
            }
        } else {
            let max = ccursor_next_word(text, ccursor);
            CCursorRange::two(ccursor, max)
        }
    }
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
        index: next_word_boundary_char_index(text.chars(), ccursor.index),
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
    let num_chars = text.chars().count();
    CCursor {
        index: num_chars
            - next_word_boundary_char_index(text.chars().rev(), num_chars - ccursor.index),
        prefer_next_row: true,
    }
}

fn ccursor_previous_line(text: &str, ccursor: CCursor) -> CCursor {
    let num_chars = text.chars().count();
    CCursor {
        index: num_chars
            - next_line_boundary_char_index(text.chars().rev(), num_chars - ccursor.index),
        prefer_next_row: true,
    }
}

fn next_word_boundary_char_index(it: impl Iterator<Item = char>, mut index: usize) -> usize {
    let mut it = it.skip(index);
    if let Some(_first) = it.next() {
        index += 1;

        if let Some(second) = it.next() {
            index += 1;
            for next in it {
                if is_word_char(next) != is_word_char(second) {
                    break;
                }
                index += 1;
            }
        }
    }
    index
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
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_linebreak(c: char) -> bool {
    c == '\r' || c == '\n'
}

/// Accepts and returns character offset (NOT byte offset!).
pub fn find_line_start(text: &str, current_index: CCursor) -> CCursor {
    // We know that new lines, '\n', are a single byte char, but we have to
    // work with char offsets because before the new line there may be any
    // number of multi byte chars.
    // We need to know the char index to be able to correctly set the cursor
    // later.
    let chars_count = text.chars().count();

    let position = text
        .chars()
        .rev()
        .skip(chars_count - current_index.index)
        .position(|x| x == '\n');

    match position {
        Some(pos) => CCursor::new(current_index.index - pos),
        None => CCursor::new(0),
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

pub fn slice_char_range(s: &str, char_range: std::ops::Range<usize>) -> &str {
    assert!(char_range.start <= char_range.end);
    let start_byte = byte_index_from_char_index(s, char_range.start);
    let end_byte = byte_index_from_char_index(s, char_range.end);
    &s[start_byte..end_byte]
}

/// The thin rectangle of one end of the selection, e.g. the primary cursor.
pub fn cursor_rect(galley_pos: Pos2, galley: &Galley, cursor: &Cursor, row_height: f32) -> Rect {
    let mut cursor_pos = galley
        .pos_from_cursor(cursor)
        .translate(galley_pos.to_vec2());
    cursor_pos.max.y = cursor_pos.max.y.at_least(cursor_pos.min.y + row_height);
    // Handle completely empty galleys
    cursor_pos = cursor_pos.expand(1.5);
    // slightly above/below row
    cursor_pos
}
