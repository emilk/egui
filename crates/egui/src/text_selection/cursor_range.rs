use epaint::{Galley, text::CharIndex, text::cursor::CCursor};

use crate::{Event, Id, Key, Modifiers, os::OperatingSystem};

use super::text_cursor_state::slice_char_range;

/// A selected text range (could be a range of length zero).
///
/// The selection is based on character count (NOT byte count!).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CCursorRange {
    /// When selecting with a mouse, this is where the mouse was released.
    /// When moving with e.g. shift+arrows, this is what moves.
    /// Note that the two ends can come in any order, and also be equal (no selection).
    pub primary: CCursor,

    /// When selecting with a mouse, this is where the mouse was first pressed.
    /// This part of the cursor does not move when shift is down.
    pub secondary: CCursor,

    /// Saved horizontal position of the cursor.
    pub h_pos: Option<f32>,
}

impl CCursorRange {
    /// The empty range.
    #[inline]
    pub fn one(ccursor: CCursor) -> Self {
        Self {
            primary: ccursor,
            secondary: ccursor,
            h_pos: None,
        }
    }

    #[inline]
    pub fn two(min: impl Into<CCursor>, max: impl Into<CCursor>) -> Self {
        Self {
            primary: max.into(),
            secondary: min.into(),
            h_pos: None,
        }
    }

    /// Select all the text in a galley
    pub fn select_all(galley: &Galley) -> Self {
        Self::two(galley.begin(), galley.end())
    }

    /// Select all of `text`, i.e. from the start to after its last character.
    #[inline]
    pub fn select_all_str(text: &str) -> Self {
        Self::two(CCursor::default(), CCursor::end_of_str(text))
    }

    /// The range of selected character indices.
    pub fn as_sorted_char_range(&self) -> core::ops::Range<CharIndex> {
        let [start, end] = self.sorted_cursors();
        core::ops::Range {
            start: start.index,
            end: end.index,
        }
    }

    /// True if the selected range contains no characters.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.primary == self.secondary
    }

    /// Is `self` a super-set of the other range?
    pub fn contains(&self, other: Self) -> bool {
        let [self_min, self_max] = self.sorted_cursors();
        let [other_min, other_max] = other.sorted_cursors();
        self_min.index <= other_min.index && other_max.index <= self_max.index
    }

    /// If there is a selection, None is returned.
    /// If the two ends are the same, that is returned.
    pub fn single(&self) -> Option<CCursor> {
        if self.is_empty() {
            Some(self.primary)
        } else {
            None
        }
    }

    #[inline]
    pub fn is_sorted(&self) -> bool {
        let p = self.primary;
        let s = self.secondary;
        (p.index, p.prefer_next_row) <= (s.index, s.prefer_next_row)
    }

    /// returns the two ends ordered
    #[inline]
    pub fn sorted_cursors(&self) -> [CCursor; 2] {
        if self.is_sorted() {
            [self.primary, self.secondary]
        } else {
            [self.secondary, self.primary]
        }
    }

    /// The two ends, the one on the left on screen first.
    ///
    /// On one row that is the one further left; across rows, the one that comes first
    /// in a left-to-right paragraph and last in a right-to-left one.
    fn sorted_on_screen(&self, galley: &Galley) -> [CCursor; 2] {
        let [min, max] = self.sorted_cursors();
        let min_row = galley.layout_from_cursor(min).row;
        let max_row = galley.layout_from_cursor(max).row;
        let swap = if min_row == max_row {
            galley.pos_from_cursor(max).min.x < galley.pos_from_cursor(min).min.x
        } else {
            galley
                .rows
                .get(galley.layout_from_cursor(self.primary).row)
                .is_some_and(|row| row.rtl_paragraph)
        };
        if swap { [max, min] } else { [min, max] }
    }

    pub fn slice_str<'s>(&self, text: &'s str) -> &'s str {
        let [min, max] = self.sorted_cursors();
        slice_char_range(text, min.index..max.index)
    }

    /// Check for key presses that are moving the cursor.
    ///
    /// Returns `true` if we did mutate `self`.
    pub fn on_key_press(
        &mut self,
        os: OperatingSystem,
        galley: &Galley,
        modifiers: &Modifiers,
        key: Key,
    ) -> bool {
        match key {
            Key::A if modifiers.command => {
                *self = Self::select_all(galley);
                true
            }

            Key::ArrowLeft | Key::ArrowRight if modifiers.is_none() && !self.is_empty() => {
                let [left, right] = self.sorted_on_screen(galley);
                *self = Self::one(if key == Key::ArrowLeft { left } else { right });
                true
            }

            Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Home
            | Key::End => {
                move_single_cursor(
                    os,
                    &mut self.primary,
                    &mut self.h_pos,
                    galley,
                    key,
                    modifiers,
                );
                if !modifiers.shift {
                    self.secondary = self.primary;
                }
                true
            }

            Key::P | Key::N | Key::B | Key::F | Key::A | Key::E
                if os == OperatingSystem::Mac && modifiers.ctrl && !modifiers.shift =>
            {
                move_single_cursor(
                    os,
                    &mut self.primary,
                    &mut self.h_pos,
                    galley,
                    key,
                    modifiers,
                );
                self.secondary = self.primary;
                true
            }

            _ => false,
        }
    }

    /// Check for events that modify the cursor range.
    ///
    /// Returns `true` if such an event was found and handled.
    pub fn on_event(
        &mut self,
        os: OperatingSystem,
        event: &Event,
        galley: &Galley,
        _widget_id: Id,
    ) -> bool {
        match event {
            Event::Key {
                modifiers,
                key,
                pressed: true,
                ..
            } => self.on_key_press(os, galley, modifiers, *key),

            Event::AccessKitActionRequest(accesskit::ActionRequest {
                action: accesskit::Action::SetTextSelection,
                target_node,
                target_tree,
                data: Some(accesskit::ActionData::SetTextSelection(selection)),
            }) => {
                if _widget_id.accesskit_id() == *target_node
                    && *target_tree == accesskit::TreeId::ROOT
                {
                    let primary =
                        ccursor_from_accesskit_text_position(_widget_id, galley, &selection.focus);
                    let secondary =
                        ccursor_from_accesskit_text_position(_widget_id, galley, &selection.anchor);
                    if let (Some(primary), Some(secondary)) = (primary, secondary) {
                        *self = Self {
                            primary,
                            secondary,
                            h_pos: None,
                        };
                        return true;
                    }
                }
                false
            }

            _ => false,
        }
    }
}

// ----------------------------------------------------------------------------

fn ccursor_from_accesskit_text_position(
    id: Id,
    galley: &Galley,
    position: &accesskit::TextPosition,
) -> Option<CCursor> {
    use super::accesskit_text::MAX_CHARS_PER_TEXT_RUN;

    let mut total_length = 0usize;
    for (i, row) in galley.rows.iter().enumerate() {
        let row_chars = row.glyphs.len() + (row.ends_with_newline as usize);
        let num_chunks = if row_chars == 0 {
            1
        } else {
            row_chars.div_ceil(MAX_CHARS_PER_TEXT_RUN)
        };

        for chunk_idx in 0..num_chunks {
            let run_id = id.with(i).with(chunk_idx);
            if run_id.accesskit_id() == position.node {
                let column = chunk_idx * MAX_CHARS_PER_TEXT_RUN + position.character_index;
                return Some(CCursor {
                    index: CharIndex(total_length + column),
                    prefer_next_row: !(column == row.glyphs.len()
                        && !row.ends_with_newline
                        && (i + 1) < galley.rows.len()),
                });
            }
        }

        total_length += row_chars;
    }
    None
}

// ----------------------------------------------------------------------------

/// Move a text cursor based on keyboard
fn move_single_cursor(
    os: OperatingSystem,
    cursor: &mut CCursor,
    h_pos: &mut Option<f32>,
    galley: &Galley,
    key: Key,
    modifiers: &Modifiers,
) {
    let (new_cursor, new_h_pos) =
        if os == OperatingSystem::Mac && modifiers.ctrl && !modifiers.shift {
            match key {
                Key::A => (galley.cursor_begin_of_row(cursor), None),
                Key::E => (galley.cursor_end_of_row(cursor), None),
                Key::P => galley.cursor_up_one_row(cursor, *h_pos),
                Key::N => galley.cursor_down_one_row(cursor, *h_pos),
                // Backward and forward in logical order, as in Cocoa's `moveBackward:` and `moveForward:`:
                Key::B => (galley.cursor_left_one_character(cursor), None),
                Key::F => (galley.cursor_right_one_character(cursor), None),
                _ => return,
            }
        } else {
            match key {
                // Left and right move on screen, also in right-to-left text:
                Key::ArrowLeft => {
                    if modifiers.alt || modifiers.ctrl {
                        // alt on mac, ctrl on windows
                        (galley.cursor_word_left_visually(cursor), None)
                    } else if modifiers.mac_cmd {
                        (galley.cursor_left_end_of_row(cursor), None)
                    } else {
                        (galley.cursor_left_visually(cursor), None)
                    }
                }
                Key::ArrowRight => {
                    if modifiers.alt || modifiers.ctrl {
                        // alt on mac, ctrl on windows
                        (galley.cursor_word_right_visually(cursor), None)
                    } else if modifiers.mac_cmd {
                        (galley.cursor_right_end_of_row(cursor), None)
                    } else {
                        (galley.cursor_right_visually(cursor), None)
                    }
                }
                Key::ArrowUp => {
                    if modifiers.command {
                        // mac and windows behavior
                        (galley.begin(), None)
                    } else {
                        galley.cursor_up_one_row(cursor, *h_pos)
                    }
                }
                Key::ArrowDown => {
                    if modifiers.command {
                        // mac and windows behavior
                        (galley.end(), None)
                    } else {
                        galley.cursor_down_one_row(cursor, *h_pos)
                    }
                }

                Key::Home => {
                    if modifiers.ctrl {
                        // windows behavior
                        (galley.begin(), None)
                    } else {
                        (galley.cursor_begin_of_row(cursor), None)
                    }
                }
                Key::End => {
                    if modifiers.ctrl {
                        // windows behavior
                        (galley.end(), None)
                    } else {
                        (galley.cursor_end_of_row(cursor), None)
                    }
                }

                _ => unreachable!(),
            }
        };

    *cursor = new_cursor;
    *h_pos = new_h_pos;
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use epaint::{
        Color32, FontId,
        text::{FontDefinitions, Fonts, TextOptions},
    };

    use super::*;

    fn galley(text: &str) -> Arc<Galley> {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::default());
        fonts.with_pixels_per_point(1.0).layout_no_wrap(
            text.to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
        )
    }

    fn press(range: &mut CCursorRange, galley: &Galley, modifiers: Modifiers, key: Key) {
        assert!(range.on_key_press(OperatingSystem::Windows, galley, &modifiers, key));
    }

    /// Shift+arrows move the cursor on screen, but select a logical range.
    #[test]
    fn shift_arrows_select_a_logical_range() {
        let galley = galley("abc אבג def");
        let mut range = CCursorRange::one(galley.begin());
        for _ in 0..5 {
            press(&mut range, &galley, Modifiers::SHIFT, Key::ArrowRight);
        }
        // Past "abc " and the left end of the Hebrew word, "ג":
        assert_eq!(range.as_sorted_char_range(), CharIndex(0)..CharIndex(6));
        press(&mut range, &galley, Modifiers::SHIFT, Key::ArrowRight);
        assert_eq!(range.as_sorted_char_range(), CharIndex(0)..CharIndex(5));

        press(&mut range, &galley, Modifiers::CTRL, Key::ArrowRight);
        assert_eq!(range.single().map(|c| c.index), Some(CharIndex(4)));
    }

    /// Left and Right put the cursor at the end of the selection on that side of the screen.
    #[test]
    fn arrows_collapse_a_selection_to_its_end_on_screen() {
        let galley = galley("abc אבג def");
        // The Hebrew word, from the right of "א" to the left of "ג":
        let before_alef = CCursor {
            prefer_next_row: true,
            ..CCursor::new(4)
        };
        let after_gimel = CCursor::new(7);
        let hebrew = CCursorRange::two(before_alef, after_gimel);
        assert_eq!(hebrew.as_sorted_char_range(), CharIndex(4)..CharIndex(7));

        let mut range = hebrew;
        press(&mut range, &galley, Modifiers::NONE, Key::ArrowLeft);
        assert_eq!(range.single().map(|c| c.index), Some(CharIndex(7)));
        let mut range = hebrew;
        press(&mut range, &galley, Modifiers::NONE, Key::ArrowRight);
        assert_eq!(range.single().map(|c| c.index), Some(CharIndex(4)));
    }

    #[test]
    fn home_and_end_in_a_rtl_paragraph() {
        // On screen: "abc גבא", which starts on the right.
        let galley = galley("אבג abc");
        let mut range = CCursorRange::one(CCursor::new(5));
        press(&mut range, &galley, Modifiers::NONE, Key::Home);
        assert_eq!(range.primary.index, CharIndex(0));
        press(&mut range, &galley, Modifiers::NONE, Key::End);
        assert_eq!(range.primary.index, CharIndex(4));
        press(&mut range, &galley, Modifiers::NONE, Key::ArrowRight);
        assert_eq!(range.primary.index, CharIndex(5));

        // Ctrl+F on a Mac moves forward in logical order:
        let mut range = CCursorRange::one(CCursor::new(2));
        let modifiers = Modifiers::CTRL;
        assert!(range.on_key_press(OperatingSystem::Mac, &galley, &modifiers, Key::F));
        assert_eq!(range.primary.index, CharIndex(3));
    }
}
