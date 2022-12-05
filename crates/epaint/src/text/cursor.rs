//! Different types of text cursors, i.e. ways to point into a [`super::Galley`].

/// Character cursor
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
    fn eq(&self, other: &CCursor) -> bool {
        self.index == other.index
    }
}

impl std::ops::Add<usize> for CCursor {
    type Output = CCursor;

    fn add(self, rhs: usize) -> Self::Output {
        CCursor {
            index: self.index.saturating_add(rhs),
            prefer_next_row: self.prefer_next_row,
        }
    }
}

impl std::ops::Sub<usize> for CCursor {
    type Output = CCursor;

    fn sub(self, rhs: usize) -> Self::Output {
        CCursor {
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

/// Row Cursor
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RCursor {
    /// 0 is first row, and so on.
    /// Note that a single paragraph can span multiple rows.
    /// (a paragraph is text separated by `\n`).
    pub row: usize,

    /// Character based (NOT bytes).
    /// It is fine if this points to something beyond the end of the current row.
    /// When moving up/down it may again be within the next row.
    pub column: usize,
}

/// Paragraph Cursor
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PCursor {
    /// 0 is first paragraph, and so on.
    /// Note that a single paragraph can span multiple rows.
    /// (a paragraph is text separated by `\n`).
    pub paragraph: usize,

    /// Character based (NOT bytes).
    /// It is fine if this points to something beyond the end of the current paragraph.
    /// When moving up/down it may again be within the next paragraph.
    pub offset: usize,

    /// If this cursors sits right at the border of a wrapped row break (NOT paragraph break)
    /// do we prefer the next row?
    /// This is *almost* always what you want, *except* for when
    /// explicitly clicking the end of a row or pressing the end key.
    pub prefer_next_row: bool,
}

/// Two `PCursor`s are considered equal if they refer to the same character boundary,
/// even if one prefers the start of the next row.
impl PartialEq for PCursor {
    fn eq(&self, other: &PCursor) -> bool {
        self.paragraph == other.paragraph && self.offset == other.offset
    }
}

/// All different types of cursors together.
/// They all point to the same place, but in their own different ways.
/// pcursor/rcursor can also point to after the end of the paragraph/row.
/// Does not implement `PartialEq` because you must think which cursor should be equivalent.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Cursor {
    pub ccursor: CCursor,
    pub rcursor: RCursor,
    pub pcursor: PCursor,
}
