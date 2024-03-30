use emath::{vec2, Rect, Vec2};

/// A value for all four sides of a rectangle,
/// often used to express padding or spacing.
///
/// Can be added and subtracted to/from [`Rect`]s.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Margin {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Margin {
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

impl From<f32> for Margin {
    #[inline]
    fn from(v: f32) -> Self {
        Self::same(v)
    }
}

impl From<Vec2> for Margin {
    #[inline]
    fn from(v: Vec2) -> Self {
        Self::symmetric(v.x, v.y)
    }
}

/// `Margin + Margin`
impl std::ops::Add for Margin {
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

/// `Margin + f32`
impl std::ops::Add<f32> for Margin {
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
impl std::ops::AddAssign<f32> for Margin {
    #[inline]
    fn add_assign(&mut self, v: f32) {
        self.left += v;
        self.right += v;
        self.top += v;
        self.bottom += v;
    }
}

/// `Margin * f32`
impl std::ops::Mul<f32> for Margin {
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

/// `Margin *= f32`
impl std::ops::MulAssign<f32> for Margin {
    #[inline]
    fn mul_assign(&mut self, v: f32) {
        self.left *= v;
        self.right *= v;
        self.top *= v;
        self.bottom *= v;
    }
}

/// `Margin / f32`
impl std::ops::Div<f32> for Margin {
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

/// `Margin /= f32`
impl std::ops::DivAssign<f32> for Margin {
    #[inline]
    fn div_assign(&mut self, v: f32) {
        self.left /= v;
        self.right /= v;
        self.top /= v;
        self.bottom /= v;
    }
}

/// `Margin - Margin`
impl std::ops::Sub for Margin {
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

/// `Margin - f32`
impl std::ops::Sub<f32> for Margin {
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

/// `Margin -= f32`
impl std::ops::SubAssign<f32> for Margin {
    #[inline]
    fn sub_assign(&mut self, v: f32) {
        self.left -= v;
        self.right -= v;
        self.top -= v;
        self.bottom -= v;
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
