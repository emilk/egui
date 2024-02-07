use epaint::{text::cursor::*, Galley};

use crate::{os::OperatingSystem, Event, Id, Key, Modifiers};

use super::text_cursor_state::{ccursor_next_word, ccursor_previous_word, slice_char_range};

/// A selected text range (could be a range of length zero).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CursorRange {
    /// When selecting with a mouse, this is where the mouse was released.
    /// When moving with e.g. shift+arrows, this is what moves.
    /// Note that the two ends can come in any order, and also be equal (no selection).
    pub primary: Cursor,

    /// When selecting with a mouse, this is where the mouse was first pressed.
    /// This part of the cursor does not move when shift is down.
    pub secondary: Cursor,
}

impl CursorRange {
    /// The empty range.
    #[inline]
    pub fn one(cursor: Cursor) -> Self {
        Self {
            primary: cursor,
            secondary: cursor,
        }
    }

    #[inline]
    pub fn two(min: Cursor, max: Cursor) -> Self {
        Self {
            primary: max,
            secondary: min,
        }
    }

    /// Select all the text in a galley
    pub fn select_all(galley: &Galley) -> Self {
        Self::two(galley.begin(), galley.end())
    }

    pub fn as_ccursor_range(&self) -> CCursorRange {
        CCursorRange {
            primary: self.primary.ccursor,
            secondary: self.secondary.ccursor,
        }
    }

    /// The range of selected character indices.
    pub fn as_sorted_char_range(&self) -> std::ops::Range<usize> {
        let [start, end] = self.sorted_cursors();
        std::ops::Range {
            start: start.ccursor.index,
            end: end.ccursor.index,
        }
    }

    /// True if the selected range contains no characters.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.primary.ccursor == self.secondary.ccursor
    }

    /// Is `self` a super-set of the the other range?
    pub fn contains(&self, other: &Self) -> bool {
        let [self_min, self_max] = self.sorted_cursors();
        let [other_min, other_max] = other.sorted_cursors();
        self_min.ccursor.index <= other_min.ccursor.index
            && other_max.ccursor.index <= self_max.ccursor.index
    }

    /// If there is a selection, None is returned.
    /// If the two ends is the same, that is returned.
    pub fn single(&self) -> Option<Cursor> {
        if self.is_empty() {
            Some(self.primary)
        } else {
            None
        }
    }

    pub fn is_sorted(&self) -> bool {
        let p = self.primary.ccursor;
        let s = self.secondary.ccursor;
        (p.index, p.prefer_next_row) <= (s.index, s.prefer_next_row)
    }

    pub fn sorted(self) -> Self {
        if self.is_sorted() {
            self
        } else {
            Self {
                primary: self.secondary,
                secondary: self.primary,
            }
        }
    }

    /// returns the two ends ordered
    pub fn sorted_cursors(&self) -> [Cursor; 2] {
        if self.is_sorted() {
            [self.primary, self.secondary]
        } else {
            [self.secondary, self.primary]
        }
    }

    pub fn slice_str<'s>(&self, text: &'s str) -> &'s str {
        let [min, max] = self.sorted_cursors();
        slice_char_range(text, min.ccursor.index..max.ccursor.index)
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
                if key == Key::ArrowLeft {
                    *self = Self::one(self.sorted_cursors()[0]);
                } else {
                    *self = Self::one(self.sorted_cursors()[1]);
                }
                true
            }

            Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Home
            | Key::End => {
                move_single_cursor(os, &mut self.primary, galley, key, modifiers);
                if !modifiers.shift {
                    self.secondary = self.primary;
                }
                true
            }

            Key::P | Key::N | Key::B | Key::F | Key::A | Key::E
                if os == OperatingSystem::Mac && modifiers.ctrl && !modifiers.shift =>
            {
                move_single_cursor(os, &mut self.primary, galley, key, modifiers);
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

            #[cfg(feature = "accesskit")]
            Event::AccessKitActionRequest(accesskit::ActionRequest {
                action: accesskit::Action::SetTextSelection,
                target,
                data: Some(accesskit::ActionData::SetTextSelection(selection)),
            }) => {
                if _widget_id.accesskit_id() == *target {
                    let primary =
                        ccursor_from_accesskit_text_position(_widget_id, galley, &selection.focus);
                    let secondary =
                        ccursor_from_accesskit_text_position(_widget_id, galley, &selection.anchor);
                    if let (Some(primary), Some(secondary)) = (primary, secondary) {
                        *self = Self {
                            primary: galley.from_ccursor(primary),
                            secondary: galley.from_ccursor(secondary),
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
}

impl CCursorRange {
    /// The empty range.
    #[inline]
    pub fn one(ccursor: CCursor) -> Self {
        Self {
            primary: ccursor,
            secondary: ccursor,
        }
    }

    #[inline]
    pub fn two(min: impl Into<CCursor>, max: impl Into<CCursor>) -> Self {
        Self {
            primary: max.into(),
            secondary: min.into(),
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
    pub fn sorted(&self) -> [CCursor; 2] {
        if self.is_sorted() {
            [self.primary, self.secondary]
        } else {
            [self.secondary, self.primary]
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PCursorRange {
    /// When selecting with a mouse, this is where the mouse was released.
    /// When moving with e.g. shift+arrows, this is what moves.
    /// Note that the two ends can come in any order, and also be equal (no selection).
    pub primary: PCursor,

    /// When selecting with a mouse, this is where the mouse was first pressed.
    /// This part of the cursor does not move when shift is down.
    pub secondary: PCursor,
}

// ----------------------------------------------------------------------------

#[cfg(feature = "accesskit")]
fn ccursor_from_accesskit_text_position(
    id: Id,
    galley: &Galley,
    position: &accesskit::TextPosition,
) -> Option<CCursor> {
    let mut total_length = 0usize;
    for (i, row) in galley.rows.iter().enumerate() {
        let row_id = id.with(i);
        if row_id.accesskit_id() == position.node {
            return Some(CCursor {
                index: total_length + position.character_index,
                prefer_next_row: !(position.character_index == row.glyphs.len()
                    && !row.ends_with_newline
                    && (i + 1) < galley.rows.len()),
            });
        }
        total_length += row.glyphs.len() + (row.ends_with_newline as usize);
    }
    None
}

// ----------------------------------------------------------------------------

/// Move a text cursor based on keyboard
fn move_single_cursor(
    os: OperatingSystem,
    cursor: &mut Cursor,
    galley: &Galley,
    key: Key,
    modifiers: &Modifiers,
) {
    if os == OperatingSystem::Mac && modifiers.ctrl && !modifiers.shift {
        match key {
            Key::A => *cursor = galley.cursor_begin_of_row(cursor),
            Key::E => *cursor = galley.cursor_end_of_row(cursor),
            Key::P => *cursor = galley.cursor_up_one_row(cursor),
            Key::N => *cursor = galley.cursor_down_one_row(cursor),
            Key::B => *cursor = galley.cursor_left_one_character(cursor),
            Key::F => *cursor = galley.cursor_right_one_character(cursor),
            _ => (),
        }
        return;
    }
    match key {
        Key::ArrowLeft => {
            if modifiers.alt || modifiers.ctrl {
                // alt on mac, ctrl on windows
                *cursor = galley.from_ccursor(ccursor_previous_word(galley, cursor.ccursor));
            } else if modifiers.mac_cmd {
                *cursor = galley.cursor_begin_of_row(cursor);
            } else {
                *cursor = galley.cursor_left_one_character(cursor);
            }
        }
        Key::ArrowRight => {
            if modifiers.alt || modifiers.ctrl {
                // alt on mac, ctrl on windows
                *cursor = galley.from_ccursor(ccursor_next_word(galley, cursor.ccursor));
            } else if modifiers.mac_cmd {
                *cursor = galley.cursor_end_of_row(cursor);
            } else {
                *cursor = galley.cursor_right_one_character(cursor);
            }
        }
        Key::ArrowUp => {
            if modifiers.command {
                // mac and windows behavior
                *cursor = galley.begin();
            } else {
                *cursor = galley.cursor_up_one_row(cursor);
            }
        }
        Key::ArrowDown => {
            if modifiers.command {
                // mac and windows behavior
                *cursor = galley.end();
            } else {
                *cursor = galley.cursor_down_one_row(cursor);
            }
        }

        Key::Home => {
            if modifiers.ctrl {
                // windows behavior
                *cursor = galley.begin();
            } else {
                *cursor = galley.cursor_begin_of_row(cursor);
            }
        }
        Key::End => {
            if modifiers.ctrl {
                // windows behavior
                *cursor = galley.end();
            } else {
                *cursor = galley.cursor_end_of_row(cursor);
            }
        }

        _ => unreachable!(),
    }
}
