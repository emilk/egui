//! The granularity of a drag-selection (character/word/line).

use epaint::text::cursor::CCursor;

use super::CCursorRange;
use super::text_cursor_state::{range_bounds, select_line_at, select_word_at};

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

#[cfg(test)]
mod test {
    use super::SelectionMode;

    #[test]
    fn test_from_click_count() {
        assert_eq!(SelectionMode::from_click_count(0), SelectionMode::Char);
        assert_eq!(SelectionMode::from_click_count(1), SelectionMode::Char);
        assert_eq!(SelectionMode::from_click_count(2), SelectionMode::Word);
        assert_eq!(SelectionMode::from_click_count(3), SelectionMode::Line);
        assert_eq!(SelectionMode::from_click_count(4), SelectionMode::Line);
    }
}
