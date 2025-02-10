use emath::{vec2, Rect, Vec2};

use crate::Margin;

/// A value for all four sides of a rectangle,
/// often used to express padding or spacing.
///
/// Can be added and subtracted to/from [`Rect`]s.
///
/// For storage, use [`crate::Margin`] instead.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct MarginF32 {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[deprecated = "Renamed to MarginF32"]
pub type Marginf = MarginF32;

impl From<Margin> for MarginF32 {
    #[inline]
    fn from(margin: Margin) -> Self {
        Self {
            left: margin.left as _,
            right: margin.right as _,
            top: margin.top as _,
            bottom: margin.bottom as _,
        }
    }
}

impl From<MarginF32> for Margin {
    #[inline]
    fn from(marginf: MarginF32) -> Self {
        Self {
            left: marginf.left as _,
            right: marginf.right as _,
            top: marginf.top as _,
            bottom: marginf.bottom as _,
        }
    }
}

impl MarginF32 {
    pub const ZERO: Self = Self {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        bottom: 0.0,
    };

    /// The same margin on every side.
    #[doc(alias = "symmetric")]
    #[inline]
    pub const fn same(margin: f32) -> Self {
        Self {
            left: margin,
            right: margin,
            top: margin,
            bottom: margin,
        }
    }

    /// Margins with the same size on opposing sides
    #[inline]
    pub const fn symmetric(x: f32, y: f32) -> Self {
        Self {
            left: x,
            right: x,
            top: y,
            bottom: y,
        }
    }

    /// Total margins on both sides
    #[inline]
    pub fn sum(&self) -> Vec2 {
        vec2(self.left + self.right, self.top + self.bottom)
    }

    #[inline]
    pub const fn left_top(&self) -> Vec2 {
        vec2(self.left, self.top)
    }

    #[inline]
    pub const fn right_bottom(&self) -> Vec2 {
        vec2(self.right, self.bottom)
    }

    /// Are the margin on every side the same?
    #[doc(alias = "symmetric")]
    #[inline]
    pub fn is_same(&self) -> bool {
        self.left == self.right && self.left == self.top && self.left == self.bottom
    }

    #[deprecated = "Use `rect + margin` instead"]
    #[inline]
    pub fn expand_rect(&self, rect: Rect) -> Rect {
        Rect::from_min_max(rect.min - self.left_top(), rect.max + self.right_bottom())
    }

    #[deprecated = "Use `rect - margin` instead"]
    #[inline]
    pub fn shrink_rect(&self, rect: Rect) -> Rect {
        Rect::from_min_max(rect.min + self.left_top(), rect.max - self.right_bottom())
    }
}

impl From<f32> for MarginF32 {
    #[inline]
    fn from(v: f32) -> Self {
        Self::same(v)
    }
}

impl From<Vec2> for MarginF32 {
    #[inline]
    fn from(v: Vec2) -> Self {
        Self::symmetric(v.x, v.y)
    }
}

/// `MarginF32 + MarginF32`
impl std::ops::Add for MarginF32 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            left: self.left + other.left,
            right: self.right + other.right,
            top: self.top + other.top,
            bottom: self.bottom + other.bottom,
        }
    }
}

/// `MarginF32 + f32`
impl std::ops::Add<f32> for MarginF32 {
    type Output = Self;

    #[inline]
    fn add(self, v: f32) -> Self {
        Self {
            left: self.left + v,
            right: self.right + v,
            top: self.top + v,
            bottom: self.bottom + v,
        }
    }
}

/// `Margind += f32`
impl std::ops::AddAssign<f32> for MarginF32 {
    #[inline]
    fn add_assign(&mut self, v: f32) {
        self.left += v;
        self.right += v;
        self.top += v;
        self.bottom += v;
    }
}

/// `MarginF32 * f32`
impl std::ops::Mul<f32> for MarginF32 {
    type Output = Self;

    #[inline]
    fn mul(self, v: f32) -> Self {
        Self {
            left: self.left * v,
            right: self.right * v,
            top: self.top * v,
            bottom: self.bottom * v,
        }
    }
}

/// `MarginF32 *= f32`
impl std::ops::MulAssign<f32> for MarginF32 {
    #[inline]
    fn mul_assign(&mut self, v: f32) {
        self.left *= v;
        self.right *= v;
        self.top *= v;
        self.bottom *= v;
    }
}

/// `MarginF32 / f32`
impl std::ops::Div<f32> for MarginF32 {
    type Output = Self;

    #[inline]
    fn div(self, v: f32) -> Self {
        Self {
            left: self.left / v,
            right: self.right / v,
            top: self.top / v,
            bottom: self.bottom / v,
        }
    }
}

/// `MarginF32 /= f32`
impl std::ops::DivAssign<f32> for MarginF32 {
    #[inline]
    fn div_assign(&mut self, v: f32) {
        self.left /= v;
        self.right /= v;
        self.top /= v;
        self.bottom /= v;
    }
}

/// `MarginF32 - MarginF32`
impl std::ops::Sub for MarginF32 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            left: self.left - other.left,
            right: self.right - other.right,
            top: self.top - other.top,
            bottom: self.bottom - other.bottom,
        }
    }
}

/// `MarginF32 - f32`
impl std::ops::Sub<f32> for MarginF32 {
    type Output = Self;

    #[inline]
    fn sub(self, v: f32) -> Self {
        Self {
            left: self.left - v,
            right: self.right - v,
            top: self.top - v,
            bottom: self.bottom - v,
        }
    }
}

/// `MarginF32 -= f32`
impl std::ops::SubAssign<f32> for MarginF32 {
    #[inline]
    fn sub_assign(&mut self, v: f32) {
        self.left -= v;
        self.right -= v;
        self.top -= v;
        self.bottom -= v;
    }
}

/// `Rect + MarginF32`
impl std::ops::Add<MarginF32> for Rect {
    type Output = Self;

    #[inline]
    fn add(self, margin: MarginF32) -> Self {
        Self::from_min_max(
            self.min - margin.left_top(),
            self.max + margin.right_bottom(),
        )
    }
}

/// `Rect += MarginF32`
impl std::ops::AddAssign<MarginF32> for Rect {
    #[inline]
    fn add_assign(&mut self, margin: MarginF32) {
        *self = *self + margin;
    }
}

/// `Rect - MarginF32`
impl std::ops::Sub<MarginF32> for Rect {
    type Output = Self;

    #[inline]
    fn sub(self, margin: MarginF32) -> Self {
        Self::from_min_max(
            self.min + margin.left_top(),
            self.max - margin.right_bottom(),
        )
    }
}

/// `Rect -= MarginF32`
impl std::ops::SubAssign<MarginF32> for Rect {
    #[inline]
    fn sub_assign(&mut self, margin: MarginF32) {
        *self = *self - margin;
    }
}
