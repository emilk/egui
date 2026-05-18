//! Text cursor changes/interaction, without modifying the text.

use epaint::text::{Galley, cursor::CCursor};
use unicode_segmentation::UnicodeSegmentation as _;

use crate::{NumExt as _, Rect, Response, Ui, epaint};

use super::CCursorRange;

/// How a drag-selection extends the selection.
///
/// A plain click-and-drag selects character-by-character. Double-click-and-drag
/// selects word-by-word, and triple-click-and-drag selects line-by-line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) enum SelectionMode {
    /// Select character-by-character (plain click-and-drag).
    #[default]
    Char,

    /// Select word-by-word (double-click-and-drag).
    Word,

    /// Select line-by-line (triple-click-and-drag).
    Line,
}

impl SelectionMode {
    /// Derive the selection mode from a press click-count (1 => Char, 2 => Word, >=3 => Line).
    pub(crate) fn from_click_count(count: u32) -> Self {
        match count {
            2 => Self::Word,
            3.. => Self::Line,
            _ => Self::Char,
        }
    }

    /// Returns the unit (word/line/char) range containing `ccursor` in `text`.
    pub(crate) fn unit_at(self, text: &str, ccursor: CCursor) -> CCursorRange {
        match self {
            Self::Char => CCursorRange::one(ccursor),
            Self::Word => select_word_at(text, ccursor),
            Self::Line => select_line_at(text, ccursor),
        }
    }

    /// Returns the `(min, max)` char range of the unit containing `ccursor`.
    pub(crate) fn unit_bounds_at(self, text: &str, ccursor: CCursor) -> (usize, usize) {
        range_bounds(&self.unit_at(text, ccursor))
    }
}

/// The state of a text cursor selection.
///
/// Used for [`crate::TextEdit`] and [`crate::Label`].
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextCursorState {
    ccursor_range: Option<CCursorRange>,

    /// The granularity of the current drag-selection.
    ///
    /// Ephemeral drag state; not persisted.
    #[cfg_attr(feature = "serde", serde(skip))]
    selection_mode: SelectionMode,

    /// The unit (word/line) range the current drag started in.
    ///
    /// Used in [`SelectionMode::Word`] / [`SelectionMode::Line`] so that dragging
    /// back and forth across the anchor re-derives the selection correctly.
    /// Stored as `(min, max)` character indices.
    ///
    /// Ephemeral drag state; not persisted.
    #[cfg_attr(feature = "serde", serde(skip))]
    drag_anchor_unit: Option<(usize, usize)>,

    /// Stationary-pointer cache for the word/line unit lookup.
    ///
    /// `extend_word_line_drag` runs every frame while dragging and does an O(n)
    /// word/line scan. This caches the pointer's char index and the resulting
    /// range from the previous frame, so the scan is skipped while the pointer
    /// stays on the same character. Cleared when a new drag begins.
    ///
    /// Ephemeral drag state; not persisted.
    #[cfg_attr(feature = "serde", serde(skip))]
    last_drag_pointer: Option<(usize, CCursorRange)>,
}

impl From<CCursorRange> for TextCursorState {
    fn from(ccursor_range: CCursorRange) -> Self {
        Self {
            ccursor_range: Some(ccursor_range),
            ..Default::default()
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

    /// Begin a drag-selection in the given `mode`, anchoring on the unit
    /// (word/line/char) under `cursor_at_pointer`.
    ///
    /// Returns the initial [`CCursorRange`] for the selection.
    fn begin_drag(
        &mut self,
        text: &str,
        cursor_at_pointer: CCursor,
        mode: SelectionMode,
    ) -> CCursorRange {
        self.selection_mode = mode;
        let unit = mode.unit_at(text, cursor_at_pointer);
        self.drag_anchor_unit = Some(range_bounds(&unit));
        // A fresh drag must never reuse a stale cached range.
        self.last_drag_pointer = None;
        unit
    }

    /// Extend the current word/line drag-selection to the pointer position.
    ///
    /// Returns the new [`CCursorRange`], or `None` if not in a word/line drag.
    ///
    /// The O(n) word/line scan is skipped while the pointer's char index is
    /// unchanged from the previous frame (see [`Self::last_drag_pointer`]).
    fn extend_word_line_drag(
        &mut self,
        text: &str,
        cursor_at_pointer: CCursor,
    ) -> Option<CCursorRange> {
        if self.selection_mode == SelectionMode::Char {
            return None;
        }
        let anchor_unit = self.drag_anchor_unit?;

        // Stationary-pointer fast path: same char index as last frame.
        if let Some((cached_index, cached_range)) = self.last_drag_pointer
            && cached_index == cursor_at_pointer.index
        {
            return Some(cached_range);
        }

        let pointer_unit = range_bounds(&self.selection_mode.unit_at(text, cursor_at_pointer));
        let range = combine_units(anchor_unit, pointer_unit);
        self.last_drag_pointer = Some((cursor_at_pointer.index, range));
        Some(range)
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
                    // Shift-click extends the current selection; keep char granularity.
                    self.selection_mode = SelectionMode::Char;
                    self.drag_anchor_unit = None;
                    if let Some(mut cursor_range) = self.range(galley) {
                        cursor_range.primary = cursor_at_pointer;
                        self.set_char_range(Some(cursor_range));
                    } else {
                        self.set_char_range(Some(CCursorRange::one(cursor_at_pointer)));
                    }
                } else {
                    // Use the press click-count to decide the drag granularity:
                    // 2 => word-by-word, >=3 => line-by-line, else char-by-char.
                    let mode = ui
                        .input(|i| i.pointer.press_click_count())
                        .map_or(SelectionMode::Char, SelectionMode::from_click_count);
                    let ccursor_range = self.begin_drag(text, cursor_at_pointer, mode);
                    self.set_char_range(Some(ccursor_range));
                }
                true
            } else if is_being_dragged {
                // Drag to select text:
                if let Some(ccursor_range) = self.extend_word_line_drag(text, cursor_at_pointer) {
                    // Word-/line-by-word drag (double-/triple-click-and-drag).
                    self.set_char_range(Some(ccursor_range));
                } else if let Some(mut cursor_range) = self.range(galley) {
                    // Plain character-by-character drag.
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

/// Extend a drag-selection from an anchored unit to the unit under the pointer.
///
/// `anchor_unit` is the `(min, max)` character range of the word/line the drag
/// started in. `pointer_unit` is the `(min, max)` range of the word/line under
/// the current pointer position. The result is the union of the two units, with
/// `primary` placed at the moving (pointer) end and `secondary` at the anchored
/// end, so the selection extends in the direction the user drags and shrinks
/// back when dragging towards the anchor.
pub(crate) fn combine_units(
    anchor_unit: (usize, usize),
    pointer_unit: (usize, usize),
) -> CCursorRange {
    let (anchor_min, anchor_max) = anchor_unit;
    let (pointer_min, pointer_max) = pointer_unit;

    let union_min = anchor_min.min(pointer_min);
    let union_max = anchor_max.max(pointer_max);

    // If the pointer unit is at or before the anchor, the selection extends to
    // the left, so the moving (primary) end is the union's minimum.
    // Otherwise the selection extends to the right.
    if pointer_min < anchor_min {
        CCursorRange {
            primary: CCursor::new(union_min),
            secondary: CCursor::new(union_max),
            h_pos: None,
        }
    } else {
        CCursorRange {
            primary: CCursor::new(union_max),
            secondary: CCursor::new(union_min),
            h_pos: None,
        }
    }
}

/// Pick which end of an anchor unit the selection's `secondary` should point to.
///
/// During a cross-galley word/line drag, `secondary` must always sit at the
/// *far* end of the anchor unit relative to the moving (primary) end, so the
/// whole anchor word/line stays selected regardless of drag direction.
///
/// `anchor_unit` is the `(min, max)` char range of the anchored word/line.
/// `primary_before_anchor` is `true` when the moving end has been dragged
/// upward/leftward past the anchor (i.e. the selection extends backwards).
///
/// When the primary is before the anchor, the selection extends backwards, so
/// `secondary` must be at the anchor's `max`; otherwise it is at the anchor's
/// `min`.
pub(crate) fn anchor_secondary_index(
    anchor_unit: (usize, usize),
    primary_before_anchor: bool,
) -> usize {
    let (anchor_min, anchor_max) = anchor_unit;
    if primary_before_anchor {
        anchor_max
    } else {
        anchor_min
    }
}

/// Returns the `(min, max)` character indices of a [`CCursorRange`].
fn range_bounds(range: &CCursorRange) -> (usize, usize) {
    (
        range.primary.index.min(range.secondary.index),
        range.primary.index.max(range.secondary.index),
    )
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
    let num_chars = text.chars().count();
    let reversed: String = text.graphemes(true).rev().collect();
    CCursor {
        index: num_chars
            - next_word_boundary_char_index(&reversed, num_chars - ccursor.index).min(num_chars),
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

fn next_word_boundary_char_index(text: &str, cursor_ci: usize) -> usize {
    let mut current_char_idx = 0;

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
    use super::*;

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

    #[test]
    fn test_previous_word() {
        let text = "abc def ghi";
        assert_eq!(ccursor_previous_word(text, CCursor::new(7)).index, 4);
        assert_eq!(ccursor_previous_word(text, CCursor::new(5)).index, 4);
        assert_eq!(ccursor_previous_word(text, CCursor::new(4)).index, 0);
        assert_eq!(ccursor_previous_word(text, CCursor::new(0)).index, 0);
    }

    #[test]
    fn test_next_word() {
        let text = "abc def ghi";
        assert_eq!(ccursor_next_word(text, CCursor::new(0)).index, 3);
        assert_eq!(ccursor_next_word(text, CCursor::new(3)).index, 7);
        assert_eq!(ccursor_next_word(text, CCursor::new(7)).index, 11);
        assert_eq!(ccursor_next_word(text, CCursor::new(11)).index, 11);
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
        assert_eq!(lo, 0);
        assert_eq!(hi, 5);

        let range = select_word_at(text, CCursor::new(8));
        let (lo, hi) = (
            range.primary.index.min(range.secondary.index),
            range.primary.index.max(range.secondary.index),
        );
        assert_eq!(lo, 6);
        assert_eq!(hi, 11);
    }

    /// Helper mirroring `extend_word_line_drag`: given an anchor cursor and a
    /// pointer cursor, return the `(min, max)` of the resulting selection.
    fn drag_select(
        mode: SelectionMode,
        text: &str,
        anchor: usize,
        pointer: usize,
    ) -> (usize, usize) {
        let anchor_unit = mode.unit_bounds_at(text, CCursor::new(anchor));
        let pointer_unit = mode.unit_bounds_at(text, CCursor::new(pointer));
        let range = combine_units(anchor_unit, pointer_unit);
        range_bounds(&range)
    }

    #[test]
    fn test_combine_units_directions() {
        // Anchor on the middle word, drag forward and backward.
        let anchor = (6, 11); // "world" in "hello world again"

        // Pointer in the same unit -> selection is just the anchor unit.
        let same = combine_units(anchor, (6, 11));
        assert_eq!(range_bounds(&same), (6, 11));

        // Pointer unit fully after the anchor -> extends right,
        // primary (moving end) at the maximum.
        let forward = combine_units(anchor, (12, 17));
        assert_eq!(range_bounds(&forward), (6, 17));
        assert_eq!(forward.primary.index, 17);
        assert_eq!(forward.secondary.index, 6);

        // Pointer unit fully before the anchor -> extends left,
        // primary (moving end) at the minimum.
        let backward = combine_units(anchor, (0, 5));
        assert_eq!(range_bounds(&backward), (0, 11));
        assert_eq!(backward.primary.index, 0);
        assert_eq!(backward.secondary.index, 11);
    }

    #[test]
    fn test_word_drag_extension() {
        let text = "hello world again now";
        //          0     6     12    18

        // Double-click on "world", then drag forward into "again":
        // whole words should be selected.
        assert_eq!(drag_select(SelectionMode::Word, text, 8, 14), (6, 17));

        // Keep dragging forward into "now".
        assert_eq!(drag_select(SelectionMode::Word, text, 8, 20), (6, 21));

        // Drag back onto the anchor word -> only the anchor word selected.
        assert_eq!(drag_select(SelectionMode::Word, text, 8, 7), (6, 11));

        // Drag backward, before the anchor, into "hello".
        assert_eq!(drag_select(SelectionMode::Word, text, 8, 2), (0, 11));
    }

    #[test]
    fn test_line_drag_extension() {
        let text = "first line\nsecond line\nthird line";
        //          0          11         23

        // Triple-click on the first line, drag down into the second line:
        // both whole lines selected.
        let sel = drag_select(SelectionMode::Line, text, 3, 15);
        assert_eq!(sel.0, 0);
        assert_eq!(sel.1, 22); // end of "second line" (before the newline)

        // Drag further down into the third line.
        let sel = drag_select(SelectionMode::Line, text, 3, 27);
        assert_eq!(sel.0, 0);
        assert_eq!(sel.1, text.chars().count());

        // Anchor on the last line, drag backward (upward) into the first line.
        let sel = drag_select(SelectionMode::Line, text, 27, 3);
        assert_eq!(sel.0, 0);
        assert_eq!(sel.1, text.chars().count());

        // Drag back onto the anchor line only.
        let sel = drag_select(SelectionMode::Line, text, 15, 13);
        assert_eq!(sel.0, 11);
        assert_eq!(sel.1, 22);
    }

    #[test]
    fn test_char_mode_is_single_cursor() {
        // Char mode units are a single cursor; combining them just yields a range.
        let text = "hello world";
        let anchor = SelectionMode::Char.unit_bounds_at(text, CCursor::new(2));
        let pointer = SelectionMode::Char.unit_bounds_at(text, CCursor::new(8));
        assert_eq!(anchor, (2, 2));
        assert_eq!(pointer, (8, 8));
        let range = combine_units(anchor, pointer);
        assert_eq!(range_bounds(&range), (2, 8));
    }

    #[test]
    fn test_word_boundary_large_text_performance() {
        // Before the O(n²) → O(n) fix, this would take minutes on large text.
        let large_text = "word ".repeat(200_000); // ~1MB
        let len = large_text.chars().count();

        let start = std::time::Instant::now();

        let next = ccursor_next_word(&large_text, CCursor::new(len - 10));
        assert!(next.index <= len);

        let prev = ccursor_previous_word(&large_text, CCursor::new(len - 10));
        assert!(prev.index < len);

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

    #[test]
    fn test_anchor_secondary_index_directions() {
        let anchor = (6, 11); // "world" in "hello world again"

        // Dragging forward/downward: primary is after the anchor, so the
        // secondary sits at the anchor's MIN, keeping the anchor word selected.
        assert_eq!(anchor_secondary_index(anchor, false), 6);

        // Dragging upward/leftward: primary is before the anchor, so the
        // secondary must sit at the anchor's MAX, keeping the anchor selected.
        assert_eq!(anchor_secondary_index(anchor, true), 11);
    }

    #[test]
    fn test_extend_word_line_drag_cache() {
        let text = "hello world again now";
        let mut state = TextCursorState::default();

        // Begin a word drag anchored on "world".
        let initial = state.begin_drag(text, CCursor::new(8), SelectionMode::Word);
        assert_eq!(range_bounds(&initial), (6, 11));
        assert!(state.last_drag_pointer.is_none(), "begin_drag clears cache");

        // First extension into "again": fresh computation, fills the cache.
        let first = state
            .extend_word_line_drag(text, CCursor::new(14))
            .expect("word drag active");
        assert_eq!(range_bounds(&first), (6, 17));
        assert_eq!(state.last_drag_pointer.map(|(i, _)| i), Some(14));

        // Repeated pointer index: cached path returns the same range.
        let cached = state
            .extend_word_line_drag(text, CCursor::new(14))
            .expect("word drag active");
        assert_eq!(range_bounds(&cached), range_bounds(&first));

        // A different index recomputes a fresh (different) range.
        let moved = state
            .extend_word_line_drag(text, CCursor::new(20))
            .expect("word drag active");
        assert_eq!(range_bounds(&moved), (6, 21));
        assert_ne!(range_bounds(&moved), range_bounds(&first));
        assert_eq!(state.last_drag_pointer.map(|(i, _)| i), Some(20));

        // The cached range must match a fresh computation for that same index.
        let fresh_for_14 = {
            let anchor_unit = state.drag_anchor_unit.unwrap();
            let pointer_unit = range_bounds(&SelectionMode::Word.unit_at(text, CCursor::new(14)));
            combine_units(anchor_unit, pointer_unit)
        };
        let cached_again = state
            .extend_word_line_drag(text, CCursor::new(14))
            .expect("word drag active");
        assert_eq!(range_bounds(&cached_again), range_bounds(&fresh_for_14));

        // begin_drag clears the cache so a new drag never reuses a stale range.
        state.begin_drag(text, CCursor::new(8), SelectionMode::Word);
        assert!(state.last_drag_pointer.is_none());
    }

    #[test]
    fn test_from_click_count() {
        assert_eq!(SelectionMode::from_click_count(0), SelectionMode::Char);
        assert_eq!(SelectionMode::from_click_count(1), SelectionMode::Char);
        assert_eq!(SelectionMode::from_click_count(2), SelectionMode::Word);
        assert_eq!(SelectionMode::from_click_count(3), SelectionMode::Line);
        assert_eq!(SelectionMode::from_click_count(4), SelectionMode::Line);
    }

    #[test]
    fn test_shift_click_resets_to_char_mode() {
        // After a word/line drag, a subsequent shift-click must be character
        // granular. `TextCursorState::pointer_interaction` resets
        // `selection_mode`/`drag_anchor_unit` on a shift-click; this verifies
        // that beginning a fresh `Char` drag (the same reset) clears the
        // word/line drag state so a stale word/line mode cannot leak through.
        let text = "hello world again";
        let mut state = TextCursorState::default();

        // Simulate a word drag leaving word state behind.
        state.begin_drag(text, CCursor::new(8), SelectionMode::Word);
        state
            .extend_word_line_drag(text, CCursor::new(14))
            .expect("word drag active");
        assert_eq!(state.selection_mode, SelectionMode::Word);
        assert!(state.drag_anchor_unit.is_some());
        assert!(state.last_drag_pointer.is_some());

        // The shift-click path sets `selection_mode = Char` and clears the
        // anchor; mirror that here and confirm the word state is gone.
        state.selection_mode = SelectionMode::Char;
        state.drag_anchor_unit = None;
        assert_eq!(state.selection_mode, SelectionMode::Char);
        // A char drag never extends word/line-wise.
        assert!(
            state.extend_word_line_drag(text, CCursor::new(2)).is_none(),
            "char mode must not perform a word/line extension"
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
                result.index, expected,
                "text={text:?}, cursor={cursor}, got={}, expected={expected}",
                result.index
            );
        }
    }
}
