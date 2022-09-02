//! One- and two-dimensional alignment ([`Align::Center`], [`Align2::LEFT_TOP`] etc).

use crate::*;

/// left/center/right or top/center/bottom alignment for e.g. anchors and layouts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Align {
    /// Left or top.
    Min,

    /// Horizontal or vertical center.
    Center,

    /// Right or bottom.
    Max,
}

impl Align {
    /// Convenience for [`Self::Min`]
    pub const LEFT: Self = Self::Min;

    /// Convenience for [`Self::Max`]
    pub const RIGHT: Self = Self::Max;

    /// Convenience for [`Self::Min`]
    pub const TOP: Self = Self::Min;

    /// Convenience for [`Self::Max`]
    pub const BOTTOM: Self = Self::Max;

    /// Convert `Min => 0.0`, `Center => 0.5` or `Max => 1.0`.
    #[inline(always)]
    pub fn to_factor(self) -> f32 {
        match self {
            Self::Min => 0.0,
            Self::Center => 0.5,
            Self::Max => 1.0,
        }
    }

    /// Convert `Min => -1.0`, `Center => 0.0` or `Max => 1.0`.
    #[inline(always)]
    pub fn to_sign(self) -> f32 {
        match self {
            Self::Min => -1.0,
            Self::Center => 0.0,
            Self::Max => 1.0,
        }
    }

    /// Returns a range of given size within a specified range.
    ///
    /// If the requested `size` is bigger than the size of `range`, then the returned
    /// range will not fit into the available `range`. The extra space will be allocated
    /// from:
    ///
    /// |Align |Side        |
    /// |------|------------|
    /// |Min   |right (end) |
    /// |Center|both        |
    /// |Max   |left (start)|
    ///
    /// # Examples
    /// ```
    /// use std::f32::{INFINITY, NEG_INFINITY};
    /// use emath::Align::*;
    ///
    /// // The size is smaller than a range
    /// assert_eq!(Min   .align_size_within_range(2.0, 10.0..=20.0), 10.0..=12.0);
    /// assert_eq!(Center.align_size_within_range(2.0, 10.0..=20.0), 14.0..=16.0);
    /// assert_eq!(Max   .align_size_within_range(2.0, 10.0..=20.0), 18.0..=20.0);
    ///
    /// // The size is bigger than a range
    /// assert_eq!(Min   .align_size_within_range(20.0, 10.0..=20.0), 10.0..=30.0);
    /// assert_eq!(Center.align_size_within_range(20.0, 10.0..=20.0),  5.0..=25.0);
    /// assert_eq!(Max   .align_size_within_range(20.0, 10.0..=20.0),  0.0..=20.0);
    ///
    /// // The size is infinity, but range is finite - a special case of a previous example
    /// assert_eq!(Min   .align_size_within_range(INFINITY, 10.0..=20.0),         10.0..=INFINITY);
    /// assert_eq!(Center.align_size_within_range(INFINITY, 10.0..=20.0), NEG_INFINITY..=INFINITY);
    /// assert_eq!(Max   .align_size_within_range(INFINITY, 10.0..=20.0), NEG_INFINITY..=20.0);
    /// ```
    ///
    /// The infinity-sized ranges can produce a surprising results, if the size is also infinity,
    /// use such ranges with carefully!
    ///
    /// ```
    /// use std::f32::{INFINITY, NEG_INFINITY};
    /// use emath::Align::*;
    ///
    /// // Allocating a size aligned for infinity bound will lead to empty ranges!
    /// assert_eq!(Min   .align_size_within_range(2.0, 10.0..=INFINITY),     10.0..=12.0);
    /// assert_eq!(Center.align_size_within_range(2.0, 10.0..=INFINITY), INFINITY..=INFINITY);// (!)
    /// assert_eq!(Max   .align_size_within_range(2.0, 10.0..=INFINITY), INFINITY..=INFINITY);// (!)
    ///
    /// assert_eq!(Min   .align_size_within_range(2.0, NEG_INFINITY..=20.0), NEG_INFINITY..=NEG_INFINITY);// (!)
    /// assert_eq!(Center.align_size_within_range(2.0, NEG_INFINITY..=20.0), NEG_INFINITY..=NEG_INFINITY);// (!)
    /// assert_eq!(Max   .align_size_within_range(2.0, NEG_INFINITY..=20.0),         18.0..=20.0);
    ///
    ///
    /// // The infinity size will always return the given range if it has at least one infinity bound
    /// assert_eq!(Min   .align_size_within_range(INFINITY, 10.0..=INFINITY), 10.0..=INFINITY);
    /// assert_eq!(Center.align_size_within_range(INFINITY, 10.0..=INFINITY), 10.0..=INFINITY);
    /// assert_eq!(Max   .align_size_within_range(INFINITY, 10.0..=INFINITY), 10.0..=INFINITY);
    ///
    /// assert_eq!(Min   .align_size_within_range(INFINITY, NEG_INFINITY..=20.0), NEG_INFINITY..=20.0);
    /// assert_eq!(Center.align_size_within_range(INFINITY, NEG_INFINITY..=20.0), NEG_INFINITY..=20.0);
    /// assert_eq!(Max   .align_size_within_range(INFINITY, NEG_INFINITY..=20.0), NEG_INFINITY..=20.0);
    /// ```
    #[inline]
    pub fn align_size_within_range(
        self,
        size: f32,
        range: RangeInclusive<f32>,
    ) -> RangeInclusive<f32> {
        let min = *range.start();
        let max = *range.end();

        if max - min == f32::INFINITY && size == f32::INFINITY {
            return range;
        }

        match self {
            Self::Min => min..=min + size,
            Self::Center => {
                if size == f32::INFINITY {
                    f32::NEG_INFINITY..=f32::INFINITY
                } else {
                    let left = (min + max) / 2.0 - size / 2.0;
                    left..=left + size
                }
            }
            Self::Max => max - size..=max,
        }
    }
}

impl Default for Align {
    #[inline(always)]
    fn default() -> Align {
        Align::Min
    }
}

// ----------------------------------------------------------------------------

/// Two-dimension alignment, e.g. [`Align2::LEFT_TOP`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Align2(pub [Align; 2]);

impl Align2 {
    pub const LEFT_BOTTOM: Align2 = Align2([Align::Min, Align::Max]);
    pub const LEFT_CENTER: Align2 = Align2([Align::Min, Align::Center]);
    pub const LEFT_TOP: Align2 = Align2([Align::Min, Align::Min]);
    pub const CENTER_BOTTOM: Align2 = Align2([Align::Center, Align::Max]);
    pub const CENTER_CENTER: Align2 = Align2([Align::Center, Align::Center]);
    pub const CENTER_TOP: Align2 = Align2([Align::Center, Align::Min]);
    pub const RIGHT_BOTTOM: Align2 = Align2([Align::Max, Align::Max]);
    pub const RIGHT_CENTER: Align2 = Align2([Align::Max, Align::Center]);
    pub const RIGHT_TOP: Align2 = Align2([Align::Max, Align::Min]);
}

impl Align2 {
    /// Returns an alignment by the X (horizontal) axis
    #[inline(always)]
    pub fn x(self) -> Align {
        self.0[0]
    }

    /// Returns an alignment by the Y (vertical) axis
    #[inline(always)]
    pub fn y(self) -> Align {
        self.0[1]
    }

    /// -1, 0, or +1 for each axis
    pub fn to_sign(self) -> Vec2 {
        vec2(self.x().to_sign(), self.y().to_sign())
    }

    /// Used e.g. to anchor a piece of text to a part of the rectangle.
    /// Give a position within the rect, specified by the aligns
    pub fn anchor_rect(self, rect: Rect) -> Rect {
        let x = match self.x() {
            Align::Min => rect.left(),
            Align::Center => rect.left() - 0.5 * rect.width(),
            Align::Max => rect.left() - rect.width(),
        };
        let y = match self.y() {
            Align::Min => rect.top(),
            Align::Center => rect.top() - 0.5 * rect.height(),
            Align::Max => rect.top() - rect.height(),
        };
        Rect::from_min_size(pos2(x, y), rect.size())
    }

    /// e.g. center a size within a given frame
    pub fn align_size_within_rect(self, size: Vec2, frame: Rect) -> Rect {
        let x_range = self.x().align_size_within_range(size.x, frame.x_range());
        let y_range = self.y().align_size_within_range(size.y, frame.y_range());
        Rect::from_x_y_ranges(x_range, y_range)
    }

    /// Returns the point on the rect's frame or in the center of a rect according
    /// to the alignments of this object.
    ///
    /// ```text
    /// (*)-----------+------(*)------+-----------(*)--> X
    ///  |            |               |            |
    ///  |  Min, Min  |  Center, Min  |  Max, Min  |
    ///  |            |               |            |
    ///  +------------+---------------+------------+
    ///  |            |               |            |
    /// (*)Min, Center|Center(*)Center|Max, Center(*)
    ///  |            |               |            |
    ///  +------------+---------------+------------+
    ///  |            |               |            |
    ///  |  Min, Max  | Center, Max   |  Max, Max  |
    ///  |            |               |            |
    /// (*)-----------+------(*)------+-----------(*)
    ///  |
    ///  Y
    /// ```
    pub fn pos_in_rect(self, frame: &Rect) -> Pos2 {
        let x = match self.x() {
            Align::Min => frame.left(),
            Align::Center => frame.center().x,
            Align::Max => frame.right(),
        };
        let y = match self.y() {
            Align::Min => frame.top(),
            Align::Center => frame.center().y,
            Align::Max => frame.bottom(),
        };

        pos2(x, y)
    }
}

impl std::ops::Index<usize> for Align2 {
    type Output = Align;

    #[inline(always)]
    fn index(&self, index: usize) -> &Align {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Align2 {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Align {
        &mut self.0[index]
    }
}

/// Allocates a rectangle of the specified `size` inside the `frame` rectangle
/// around of its center.
///
/// If `size` is bigger than the `frame`s size the returned rect will bounce out
/// of the `frame`.
pub fn center_size_in_rect(size: Vec2, frame: Rect) -> Rect {
    Align2::CENTER_CENTER.align_size_within_rect(size, frame)
}
