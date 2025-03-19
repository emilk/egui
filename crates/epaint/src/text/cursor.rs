//! Different types of text cursors, i.e. ways to point into a [`super::Galley`].

use std::ops::Range;

use ecolor::Color32;

/// Determines whether a cursor is attached to the preceding or following character.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Affinity {
    /// The cursor is attached to the following character in the text (e.g. if it's positioned in the middle of a line
    /// wrap, it'll be on the bottom line).
    #[default]
    Downstream,
    /// The cursor is attached to the preceding character in the text.
    Upstream,
}

impl PartialOrd for Affinity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Affinity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Downstream, Self::Downstream) | (Self::Upstream, Self::Upstream) => {
                std::cmp::Ordering::Equal
            }
            (Self::Downstream, Self::Upstream) => std::cmp::Ordering::Greater,
            (Self::Upstream, Self::Downstream) => std::cmp::Ordering::Less,
        }
    }
}

impl From<parley::Affinity> for Affinity {
    #[inline]
    fn from(value: parley::Affinity) -> Self {
        match value {
            parley::Affinity::Downstream => Self::Downstream,
            parley::Affinity::Upstream => Self::Upstream,
        }
    }
}

impl From<Affinity> for parley::Affinity {
    #[inline]
    fn from(value: Affinity) -> Self {
        match value {
            Affinity::Downstream => Self::Downstream,
            Affinity::Upstream => Self::Upstream,
        }
    }
}

/// Byte-index-based text cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ByteCursor {
    pub index: usize,
    pub affinity: Affinity,
}

impl ByteCursor {
    pub const START: Self = Self {
        index: 0,
        affinity: Affinity::Downstream,
    };

    pub const END: Self = Self {
        index: usize::MAX,
        affinity: Affinity::Downstream,
    };

    #[inline]
    pub(crate) fn as_parley(&self, layout: &parley::Layout<Color32>) -> parley::Cursor {
        parley::Cursor::from_byte_index(layout, self.index, self.affinity.into())
    }
}

impl From<parley::Cursor> for ByteCursor {
    #[inline]
    fn from(value: parley::Cursor) -> Self {
        Self {
            index: value.index(),
            affinity: value.affinity().into(),
        }
    }
}

/// Range between two cursors, with some extra text-edit state. Requires text layout to be done before it can be
/// constructed from two [`ByteCursor`]s.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Selection(pub(super) parley::Selection);

impl Selection {
    /// When selecting with a mouse, this is where the mouse was first pressed.
    /// This part of the cursor does not move when shift is down.
    #[inline]
    pub fn anchor(&self) -> ByteCursor {
        self.0.anchor().into()
    }

    /// When selecting with a mouse, this is where the mouse was released.
    /// When moving with e.g. shift+arrows, this is what moves.
    /// Note that the two ends can come in any order, and also be equal (no selection).
    #[inline]
    pub fn focus(&self) -> ByteCursor {
        self.0.focus().into()
    }

    #[deprecated = "use `focus` instead"]
    pub fn primary(&self) -> ByteCursor {
        self.focus()
    }

    #[deprecated = "use `anchor` instead"]
    pub fn secondary(&self) -> ByteCursor {
        self.anchor()
    }

    /// Does this selection contain any characters, or is it empty (both ends are the same)?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_collapsed()
    }

    #[inline]
    pub fn sorted_cursors(&self) -> [ByteCursor; 2] {
        if self.anchor() < self.focus() {
            [self.anchor(), self.focus()]
        } else {
            [self.focus(), self.anchor()]
        }
    }

    #[inline]
    pub fn byte_range(&self) -> Range<usize> {
        let [min, max] = self.sorted_cursors();
        min.index..max.index
    }

    pub fn contains(&self, other: &Self) -> bool {
        let [my_min, my_max] = self.sorted_cursors();
        let [other_min, other_max] = other.sorted_cursors();
        other_min >= my_min && other_max <= my_max
    }

    #[inline]
    pub fn slice_str<'s>(&self, text: &'s str) -> &'s str {
        &text[self.byte_range()]
    }

    /// Collapses this selection into an empty one around its [`Self::focus()`].
    #[inline]
    pub fn collapse(&self) -> Self {
        self.0.collapse().into()
    }
}

impl From<parley::Selection> for Selection {
    #[inline]
    fn from(value: parley::Selection) -> Self {
        Self(value)
    }
}

impl From<Selection> for parley::Selection {
    #[inline]
    fn from(value: Selection) -> Self {
        value.0
    }
}
