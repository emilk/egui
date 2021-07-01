//! One- and two-dimensional alignment ([`Align::Center`], [`Align2::LEFT_TOP`] etc).

use crate::*;

/// left/center/right or top/center/bottom alignment for e.g. anchors and layouts.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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

    #[deprecated = "Use Self::LEFT"]
    pub fn left() -> Self {
        Self::LEFT
    }
    #[deprecated = "Use Self::RIGHT"]
    pub fn right() -> Self {
        Self::RIGHT
    }
    #[deprecated = "Use Self::TOP"]
    pub fn top() -> Self {
        Self::TOP
    }
    #[deprecated = "Use Self::BOTTOM"]
    pub fn bottom() -> Self {
        Self::BOTTOM
    }

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
}

impl Default for Align {
    #[inline(always)]
    fn default() -> Align {
        Align::Min
    }
}

// ----------------------------------------------------------------------------

/// Two-dimension alignment, e.g. [`Align2::LEFT_TOP`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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
    #[inline(always)]
    pub fn x(self) -> Align {
        self.0[0]
    }

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
        let x = match self.x() {
            Align::Min => frame.left(),
            Align::Center => frame.center().x - size.x / 2.0,
            Align::Max => frame.right() - size.x,
        };
        let y = match self.y() {
            Align::Min => frame.top(),
            Align::Center => frame.center().y - size.y / 2.0,
            Align::Max => frame.bottom() - size.y,
        };

        Rect::from_min_size(Pos2::new(x, y), size)
    }

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

pub fn center_size_in_rect(size: Vec2, frame: Rect) -> Rect {
    Align2::CENTER_CENTER.align_size_within_rect(size, frame)
}
