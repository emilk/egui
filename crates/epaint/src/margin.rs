use emath::{Rect, Vec2, vec2};

/// A value for all four sides of a rectangle,
/// often used to express padding or spacing.
///
/// Can be added and subtracted to/from [`Rect`]s.
///
/// Negative margins are possible, but may produce weird behavior.
/// Use with care.
///
/// All values are stored as [`i8`] to keep the size of [`Margin`] small.
/// If you want floats, use [`crate::MarginF32`] instead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Margin {
    pub left: i8,
    pub right: i8,
    pub top: i8,
    pub bottom: i8,
}

impl Margin {
    pub const ZERO: Self = Self {
        left: 0,
        right: 0,
        top: 0,
        bottom: 0,
    };

    /// The same margin on every side.
    #[doc(alias = "symmetric")]
    #[inline]
    pub const fn same(margin: i8) -> Self {
        Self {
            left: margin,
            right: margin,
            top: margin,
            bottom: margin,
        }
    }

    /// Margins with the same size on opposing sides
    #[inline]
    pub const fn symmetric(x: i8, y: i8) -> Self {
        Self {
            left: x,
            right: x,
            top: y,
            bottom: y,
        }
    }

    /// Left margin, as `f32`
    #[inline]
    pub const fn leftf(self) -> f32 {
        self.left as _
    }

    /// Right margin, as `f32`
    #[inline]
    pub const fn rightf(self) -> f32 {
        self.right as _
    }

    /// Top margin, as `f32`
    #[inline]
    pub const fn topf(self) -> f32 {
        self.top as _
    }

    /// Bottom margin, as `f32`
    #[inline]
    pub const fn bottomf(self) -> f32 {
        self.bottom as _
    }

    /// Total margins on both sides
    #[inline]
    pub fn sum(self) -> Vec2 {
        vec2(self.leftf() + self.rightf(), self.topf() + self.bottomf())
    }

    #[inline]
    pub const fn left_top(self) -> Vec2 {
        vec2(self.leftf(), self.topf())
    }

    #[inline]
    pub const fn right_bottom(self) -> Vec2 {
        vec2(self.rightf(), self.bottomf())
    }

    /// Are the margin on every side the same?
    #[doc(alias = "symmetric")]
    #[inline]
    pub const fn is_same(self) -> bool {
        self.left == self.right && self.left == self.top && self.left == self.bottom
    }
}

impl From<i8> for Margin {
    #[inline]
    fn from(v: i8) -> Self {
        Self::same(v)
    }
}

impl From<f32> for Margin {
    #[inline]
    fn from(v: f32) -> Self {
        Self::same(v.round() as _)
    }
}

impl From<Vec2> for Margin {
    #[inline]
    fn from(v: Vec2) -> Self {
        Self::symmetric(v.x.round() as _, v.y.round() as _)
    }
}

/// `Margin + Margin`
impl std::ops::Add for Margin {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            left: self.left.saturating_add(other.left),
            right: self.right.saturating_add(other.right),
            top: self.top.saturating_add(other.top),
            bottom: self.bottom.saturating_add(other.bottom),
        }
    }
}

/// `Margin + i8`
impl std::ops::Add<i8> for Margin {
    type Output = Self;

    #[inline]
    fn add(self, v: i8) -> Self {
        Self {
            left: self.left.saturating_add(v),
            right: self.right.saturating_add(v),
            top: self.top.saturating_add(v),
            bottom: self.bottom.saturating_add(v),
        }
    }
}

/// `Margin += i8`
impl std::ops::AddAssign<i8> for Margin {
    #[inline]
    fn add_assign(&mut self, v: i8) {
        *self = *self + v;
    }
}

/// `Margin * f32`
impl std::ops::Mul<f32> for Margin {
    type Output = Self;

    #[inline]
    fn mul(self, v: f32) -> Self {
        Self {
            left: (self.leftf() * v).round() as _,
            right: (self.rightf() * v).round() as _,
            top: (self.topf() * v).round() as _,
            bottom: (self.bottomf() * v).round() as _,
        }
    }
}

/// `Margin *= f32`
impl std::ops::MulAssign<f32> for Margin {
    #[inline]
    fn mul_assign(&mut self, v: f32) {
        *self = *self * v;
    }
}

/// `Margin / f32`
impl std::ops::Div<f32> for Margin {
    type Output = Self;

    #[inline]
    fn div(self, v: f32) -> Self {
        #![expect(clippy::suspicious_arithmetic_impl)]
        self * v.recip()
    }
}

/// `Margin /= f32`
impl std::ops::DivAssign<f32> for Margin {
    #[inline]
    fn div_assign(&mut self, v: f32) {
        *self = *self / v;
    }
}

/// `Margin - Margin`
impl std::ops::Sub for Margin {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            left: self.left.saturating_sub(other.left),
            right: self.right.saturating_sub(other.right),
            top: self.top.saturating_sub(other.top),
            bottom: self.bottom.saturating_sub(other.bottom),
        }
    }
}

/// `Margin - i8`
impl std::ops::Sub<i8> for Margin {
    type Output = Self;

    #[inline]
    fn sub(self, v: i8) -> Self {
        Self {
            left: self.left.saturating_sub(v),
            right: self.right.saturating_sub(v),
            top: self.top.saturating_sub(v),
            bottom: self.bottom.saturating_sub(v),
        }
    }
}

/// `Margin -= i8`
impl std::ops::SubAssign<i8> for Margin {
    #[inline]
    fn sub_assign(&mut self, v: i8) {
        *self = *self - v;
    }
}

/// `Rect + Margin`
impl std::ops::Add<Margin> for Rect {
    type Output = Self;

    #[inline]
    fn add(self, margin: Margin) -> Self {
        Self::from_min_max(
            self.min - margin.left_top(),
            self.max + margin.right_bottom(),
        )
    }
}

/// `Rect += Margin`
impl std::ops::AddAssign<Margin> for Rect {
    #[inline]
    fn add_assign(&mut self, margin: Margin) {
        *self = *self + margin;
    }
}

/// `Rect - Margin`
impl std::ops::Sub<Margin> for Rect {
    type Output = Self;

    #[inline]
    fn sub(self, margin: Margin) -> Self {
        Self::from_min_max(
            self.min + margin.left_top(),
            self.max - margin.right_bottom(),
        )
    }
}

/// `Rect -= Margin`
impl std::ops::SubAssign<Margin> for Rect {
    #[inline]
    fn sub_assign(&mut self, margin: Margin) {
        *self = *self - margin;
    }
}
