//! Different types of text cursors, i.e. ways to point into a [`super::Galley`].

/// Character cursor.
///
/// The default cursor is zero.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CCursor {
    /// Character offset (NOT byte offset!).
    pub index: usize,

    /// If this cursors sits right at the border of a wrapped row break (NOT paragraph break)
    /// do we prefer the next row?
    /// This is *almost* always what you want, *except* for when
    /// explicitly clicking the end of a row or pressing the end key.
    pub prefer_next_row: bool,
}

impl CCursor {
    #[inline]
    pub fn new(index: usize) -> Self {
        Self {
            index,
            prefer_next_row: false,
        }
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

impl std::ops::Add<usize> for CCursor {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            index: self.index.saturating_add(rhs),
            prefer_next_row: self.prefer_next_row,
        }
    }
}

impl std::ops::Sub<usize> for CCursor {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            index: self.index.saturating_sub(rhs),
            prefer_next_row: self.prefer_next_row,
        }
    }
}

impl std::ops::AddAssign<usize> for CCursor {
    fn add_assign(&mut self, rhs: usize) {
        self.index = self.index.saturating_add(rhs);
    }
}

impl std::ops::SubAssign<usize> for CCursor {
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
    pub column: usize,
}
