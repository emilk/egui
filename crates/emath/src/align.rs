//! One- and two-dimensional alignment ([`Align::Center`], [`Align2::LEFT_TOP`] etc).

use crate::{pos2, vec2, Pos2, Rangef, Rect, Vec2};

/// left/center/right or top/center/bottom alignment for e.g. anchors and layouts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Align {
    /// Left or top.
    #[default]
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

    /// Returns the inverse alignment.
    /// `Min` becomes `Max`, `Center` stays the same, `Max` becomes `Min`.
    pub fn flip(self) -> Self {
        match self {
            Self::Min => Self::Max,
            Self::Center => Self::Center,
            Self::Max => Self::Min,
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
    pub fn align_size_within_range(self, size: f32, range: impl Into<Rangef>) -> Rangef {
        let range = range.into();
        let Rangef { min, max } = range;

        if max - min == f32::INFINITY && size == f32::INFINITY {
            return range;
        }

        match self {
            Self::Min => Rangef::new(min, min + size),
            Self::Center => {
                if size == f32::INFINITY {
                    Rangef::new(f32::NEG_INFINITY, f32::INFINITY)
                } else {
                    let left = (min + max) / 2.0 - size / 2.0;
                    Rangef::new(left, left + size)
                }
            }
            Self::Max => Rangef::new(max - size, max),
        }
    }
}

// ----------------------------------------------------------------------------

/// Two-dimension alignment, e.g. [`Align2::LEFT_TOP`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Align2(pub [Align; 2]);

impl Align2 {
    pub const LEFT_BOTTOM: Self = Self([Align::Min, Align::Max]);
    pub const LEFT_CENTER: Self = Self([Align::Min, Align::Center]);
    pub const LEFT_TOP: Self = Self([Align::Min, Align::Min]);
    pub const CENTER_BOTTOM: Self = Self([Align::Center, Align::Max]);
    pub const CENTER_CENTER: Self = Self([Align::Center, Align::Center]);
    pub const CENTER_TOP: Self = Self([Align::Center, Align::Min]);
    pub const RIGHT_BOTTOM: Self = Self([Align::Max, Align::Max]);
    pub const RIGHT_CENTER: Self = Self([Align::Max, Align::Center]);
    pub const RIGHT_TOP: Self = Self([Align::Max, Align::Min]);
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

    /// Flip on the x-axis
    /// e.g. `TOP_LEFT` -> `TOP_RIGHT`
    pub fn flip_x(self) -> Self {
        Self([self.x().flip(), self.y()])
    }

    /// Flip on the y-axis
    /// e.g. `TOP_LEFT` -> `BOTTOM_LEFT`
    pub fn flip_y(self) -> Self {
        Self([self.x(), self.y().flip()])
    }

    /// Flip on both axes
    /// e.g. `TOP_LEFT` -> `BOTTOM_RIGHT`
    pub fn flip(self) -> Self {
        Self([self.x().flip(), self.y().flip()])
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

    /// Use this anchor to position something around `pos`,
    /// e.g. [`Self::RIGHT_TOP`] means the right-top of the rect
    /// will end up at `pos`.
    pub fn anchor_size(self, pos: Pos2, size: Vec2) -> Rect {
        let x = match self.x() {
            Align::Min => pos.x,
            Align::Center => pos.x - 0.5 * size.x,
            Align::Max => pos.x - size.x,
        };
        let y = match self.y() {
            Align::Min => pos.y,
            Align::Center => pos.y - 0.5 * size.y,
            Align::Max => pos.y - size.y,
        };
        Rect::from_min_size(pos2(x, y), size)
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

/// Similar to [`Align2`] but also allows for aligning something outside a rect.
///
/// The corner from [`Align4::focus`] on the new rect will be aligned to
/// the corner from [`Align4::align`] on the original rect.
///
/// There are helper constants for the 12 common menu positions:
/// ```text
///              ┌───────────┐  ┌────────┐  ┌─────────┐              
///              │ TOP_START │  │  TOP   │  │ TOP_END │              
///              └───────────┘  └────────┘  └─────────┘               
/// ┌──────────┐ ┌────────────────────────────────────┐ ┌───────────┐
/// │LEFT_START│ │                                    │ │RIGHT_START│
/// └──────────┘ │                                    │ └───────────┘
/// ┌──────────┐ │                                    │ ┌───────────┐
/// │   LEFT   │ │             some_rect              │ │   RIGHT   │
/// └──────────┘ │                                    │ └───────────┘
/// ┌──────────┐ │                                    │ ┌───────────┐
/// │ LEFT_END │ │                                    │ │ RIGHT_END │
/// └──────────┘ └────────────────────────────────────┘ └───────────┘
///              ┌────────────┐  ┌──────┐  ┌──────────┐              
///              │BOTTOM_START│  │BOTTOM│  │BOTTOM_END│              
///              └────────────┘  └──────┘  └──────────┘              
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Align4 {
    pub align: Align2,
    pub focus: Align2,
}

impl Default for Align4 {
    fn default() -> Self {
        Self::BOTTOM_START
    }
}

impl Align4 {
    pub const TOP_START: Self = Self::new(Align2::LEFT_TOP, Align2::LEFT_BOTTOM);
    pub const TOP: Self = Self::new(Align2::CENTER_TOP, Align2::CENTER_BOTTOM);
    pub const TOP_END: Self = Self::new(Align2::RIGHT_TOP, Align2::RIGHT_BOTTOM);
    pub const RIGHT_START: Self = Self::new(Align2::RIGHT_TOP, Align2::LEFT_TOP);
    pub const RIGHT: Self = Self::new(Align2::RIGHT_CENTER, Align2::LEFT_CENTER);
    pub const RIGHT_END: Self = Self::new(Align2::RIGHT_BOTTOM, Align2::LEFT_BOTTOM);
    pub const BOTTOM_END: Self = Self::new(Align2::RIGHT_BOTTOM, Align2::RIGHT_TOP);
    pub const BOTTOM: Self = Self::new(Align2::CENTER_BOTTOM, Align2::CENTER_TOP);
    pub const BOTTOM_START: Self = Self::new(Align2::LEFT_BOTTOM, Align2::LEFT_TOP);
    pub const LEFT_END: Self = Self::new(Align2::LEFT_BOTTOM, Align2::RIGHT_BOTTOM);
    pub const LEFT: Self = Self::new(Align2::LEFT_CENTER, Align2::RIGHT_CENTER);
    pub const LEFT_START: Self = Self::new(Align2::LEFT_TOP, Align2::RIGHT_TOP);

    /// The 12 most common menu positions as an array, for use with [`Align4::find_best_align`].
    pub const MENU_ALIGNS: [Self; 12] = [
        Self::BOTTOM_START,
        Self::BOTTOM_END,
        Self::TOP_START,
        Self::TOP_END,
        Self::RIGHT_END,
        Self::RIGHT_START,
        Self::LEFT_END,
        Self::LEFT_START,
        // These come last on purpose, we prefer the corner ones
        Self::TOP,
        Self::RIGHT,
        Self::BOTTOM,
        Self::LEFT,
    ];

    pub const fn new(align: Align2, focus: Align2) -> Self {
        Self { align, focus }
    }

    pub fn align(&self) -> Align2 {
        self.align
    }

    pub fn focus(&self) -> Align2 {
        self.focus
    }

    /// Convert an [`Align2`] to an [`Align4`], positioning the new rect inside the old one.
    pub fn from_align2(align: Align2) -> Self {
        Self {
            align,
            focus: align,
        }
    }

    /// The center of the new rect will be aligned to a corner of the old rect.
    pub fn over_corner(align: Align2) -> Self {
        Self {
            align,
            focus: Align2::CENTER_CENTER,
        }
    }

    /// Position the new rect outside the old rect.
    pub fn outside(align: Align2) -> Self {
        Self {
            align,
            focus: align.flip(),
        }
    }

    /// Calculate the new rect based on a size and some optional gap.
    pub fn align_rect(&self, rect: &Rect, size: Vec2, gap: f32) -> Rect {
        let (pivot, anchor) = self.pivot_pos(rect, gap);
        pivot.anchor_size(anchor, size)
    }

    /// Returns a [`Align2`] and a [`Pos2`] that you can e.g. use with [`Area::fixed_pos`]
    /// and [`Area::pivot`] to align an [`Area`] to some rect.
    pub fn pivot_pos(&self, rect: &Rect, gap: f32) -> (Align2, Pos2) {
        (self.focus(), self.anchor(rect, gap))
    }

    /// Returns a normalized vector that can be used as an offset to create a gap between the rects
    /// while keeping the edges aligned.
    pub fn gap_factor(&self) -> Vec2 {
        let mut gap = -self.focus.to_sign();

        // Align the edges in these cases
        match *self {
            Self::TOP_START | Self::TOP_END | Self::BOTTOM_START | Self::BOTTOM_END => {
                gap.x = 0.0;
            }
            Self::LEFT_START | Self::LEFT_END | Self::RIGHT_START | Self::RIGHT_END => {
                gap.y = 0.0;
            }
            _ => {}
        }

        gap
    }

    /// Calculator the anchor point for the new rect, based on the old rect and an optional gap.
    pub fn anchor(&self, rect: &Rect, gap: f32) -> Pos2 {
        let pos = self.align.pos_in_rect(rect);

        let offset = self.gap_factor() * gap;

        pos + offset
    }

    /// Returns the 3 alternative [`Align4`]s that are flipped in various ways, for use
    /// with [`Align4::find_best_align`].
    pub fn alternatives(self) -> [Self; 3] {
        let flip_x = Self::new(self.align.flip_x(), self.focus.flip_x());
        let flip_y = Self::new(self.align.flip_y(), self.focus.flip_y());
        let flip_xy = Self::new(self.align.flip(), self.focus.flip());
        [flip_x, flip_y, flip_xy]
    }

    /// Look for the [`Align4`] that fits best in the available space.
    ///
    /// See also:
    /// - [`Align4::alternatives`] to calculate alternatives
    /// - [`Align4::MENU_ALIGNS`] for the 12 common menu positions
    pub fn find_best_align(
        mut values_to_try: impl Iterator<Item = Self>,
        available_space: Rect,
        widget_rect: Rect,
        gap: f32,
        size: Vec2,
    ) -> Self {
        let area = size.x * size.y;

        let blocked_area = |pos: Self| {
            let rect = pos.align_rect(&widget_rect, size, gap);
            area - available_space.intersect(rect).area()
        };

        let first = values_to_try.next().unwrap_or_default();

        if blocked_area(first) == 0.0 {
            return first;
        }

        let mut best_area = blocked_area(first);
        let mut best = first;

        for align in values_to_try {
            let blocked = blocked_area(align);
            if blocked == 0.0 {
                return align;
            }
            if blocked < best_area {
                best = align;
                best_area = blocked;
            }
        }

        best
    }
}
