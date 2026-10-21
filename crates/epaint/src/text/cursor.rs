//! Different types of text cursors, i.e. ways to point into a [`super::Galley`].

use super::index::CharIndex;

/// Character cursor.
///
/// The default cursor is zero.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CCursor {
    /// Character offset (NOT byte offset!).
    pub index: CharIndex,

    /// Does the cursor stick to the char after it (downstream), rather than the one before it?
    ///
    /// This decides where a cursor is drawn when those two chars are apart on screen:
    /// * At a wrapped row break (NOT paragraph break): the start of the next row,
    ///   or the end of this one. The next row is *almost* always what you want, *except*
    ///   for when explicitly clicking the end of a row or pressing the end key.
    /// * Where the text direction changes: the edge of the run after the cursor,
    ///   or of the run before it. A click sticks the cursor to the char that was clicked,
    ///   and moving the cursor sticks it to the char it moved past.
    pub prefer_next_row: bool,
}

impl CCursor {
    /// A cursor at the given character offset.
    ///
    /// NOTE: `index` is a *character* offset (i.e. counting [`char`]s), NOT a byte offset!
    /// For instance, the cursor after `"ö"` is at `CCursor::new(1)`, even though `"ö".len() == 2`.
    ///
    /// To put the cursor after the last character of a string, use [`Self::end_of_str`].
    #[inline]
    pub fn new(index: impl Into<CharIndex>) -> Self {
        Self {
            index: index.into(),
            prefer_next_row: false,
        }
    }

    /// A cursor placed after the last character of `text`,
    /// i.e. at `text.chars().count()`.
    #[inline]
    pub fn end_of_str(text: &str) -> Self {
        Self::new(text.chars().count())
    }
}

/// Two `CCursor`s are considered equal if they refer to the same character boundary,
/// even if one prefers the start of the next row.
impl PartialEq for CCursor {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl core::ops::Add<usize> for CCursor {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            index: self.index.saturating_add(rhs),
            prefer_next_row: self.prefer_next_row,
        }
    }
}

impl core::ops::Add<CharIndex> for CCursor {
    type Output = Self;

    fn add(self, rhs: CharIndex) -> Self::Output {
        Self {
            index: self.index + rhs,
            prefer_next_row: self.prefer_next_row,
        }
    }
}

impl core::ops::Sub<usize> for CCursor {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            index: self.index.saturating_sub(rhs),
            prefer_next_row: self.prefer_next_row,
        }
    }
}

impl core::ops::Sub<CharIndex> for CCursor {
    type Output = Self;

    fn sub(self, rhs: CharIndex) -> Self::Output {
        Self {
            index: self.index - rhs,
            prefer_next_row: self.prefer_next_row,
        }
    }
}

impl core::ops::AddAssign<usize> for CCursor {
    fn add_assign(&mut self, rhs: usize) {
        self.index = self.index.saturating_add(rhs);
    }
}

impl core::ops::SubAssign<usize> for CCursor {
    fn sub_assign(&mut self, rhs: usize) {
        self.index = self.index.saturating_sub(rhs);
    }
}

/// Row/column cursor.
///
/// This refers to rows and columns in layout terms--text wrapping creates multiple rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct LayoutCursor {
    /// 0 is first row, and so on.
    /// Note that a single paragraph can span multiple rows.
    /// (a paragraph is text separated by `\n`).
    pub row: usize,

    /// Character based (NOT bytes).
    /// It is fine if this points to something beyond the end of the current row.
    /// When moving up/down it may again be within the next row.
    pub column: CharIndex,
}
