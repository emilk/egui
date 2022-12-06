use epaint::text::cursor::*;

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
    pub fn one(cursor: Cursor) -> Self {
        Self {
            primary: cursor,
            secondary: cursor,
        }
    }

    pub fn two(min: Cursor, max: Cursor) -> Self {
        Self {
            primary: max,
            secondary: min,
        }
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
    pub fn is_empty(&self) -> bool {
        self.primary.ccursor == self.secondary.ccursor
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
    pub fn one(ccursor: CCursor) -> Self {
        Self {
            primary: ccursor,
            secondary: ccursor,
        }
    }

    pub fn two(min: CCursor, max: CCursor) -> Self {
        Self {
            primary: max,
            secondary: min,
        }
    }

    pub fn is_sorted(&self) -> bool {
        let p = self.primary;
        let s = self.secondary;
        (p.index, p.prefer_next_row) <= (s.index, s.prefer_next_row)
    }

    /// returns the two ends ordered
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
