use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, RangeInclusive, Sub, SubAssign};

use crate::math::*;

/// A vector has a direction and length.
/// A [`Vec2`] is often used to represent a size.
///
/// Egui represents positions using [`Pos2`].
///
/// Normally the units are points (logical pixels).
#[derive(Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

/// `vec2(x,y) == Vec2::new(x, y)`
#[inline(always)]
pub const fn vec2(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}

impl From<[f32; 2]> for Vec2 {
    fn from(v: [f32; 2]) -> Self {
        Self { x: v[0], y: v[1] }
    }
}

impl From<&[f32; 2]> for Vec2 {
    fn from(v: &[f32; 2]) -> Self {
        Self { x: v[0], y: v[1] }
    }
}

impl Vec2 {
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn infinity() -> Self {
        Self {
            x: f32::INFINITY,
            y: f32::INFINITY,
        }
    }

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn splat(v: impl Into<f32>) -> Self {
        let v: f32 = v.into();
        Self { x: v, y: v }
    }

    #[must_use]
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len <= 0.0 {
            self
        } else {
            self / len
        }
    }

    /// Rotates the vector by 90°, i.e positive X to positive Y
    /// (clockwise in Egui coordinates).
    #[inline(always)]
    pub fn rot90(self) -> Self {
        vec2(self.y, -self.x)
    }

    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Create a unit vector with the given angle (in radians).
    /// * An angle of zero gives the unit X axis.
    /// * An angle of 𝞃/4 = 90° gives the unit Y axis.
    pub fn angled(angle: f32) -> Self {
        vec2(angle.cos(), angle.sin())
    }

    #[must_use]
    pub fn floor(self) -> Self {
        vec2(self.x.floor(), self.y.floor())
    }

    #[must_use]
    pub fn round(self) -> Self {
        vec2(self.x.round(), self.y.round())
    }

    #[must_use]
    pub fn ceil(self) -> Self {
        vec2(self.x.ceil(), self.y.ceil())
    }

    /// True if all members are also finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    #[must_use]
    pub fn min(self, other: Self) -> Self {
        vec2(self.x.min(other.x), self.y.min(other.y))
    }

    #[must_use]
    pub fn max(self, other: Self) -> Self {
        vec2(self.x.max(other.x), self.y.max(other.y))
    }

    /// Returns the minimum of `self.x` and `self.y`.
    #[must_use]
    pub fn min_elem(self) -> f32 {
        self.x.min(self.y)
    }

    /// Returns the maximum of `self.x` and `self.y`.
    #[must_use]
    pub fn max_elem(self) -> f32 {
        self.x.max(self.y)
    }

    #[must_use]
    pub fn clamp(self, range: RangeInclusive<Self>) -> Self {
        Self {
            x: clamp(self.x, range.start().x..=range.end().x),
            y: clamp(self.y, range.start().y..=range.end().y),
        }
    }
}

impl PartialEq for Vec2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}
impl Eq for Vec2 {}

impl Neg for Vec2 {
    type Output = Vec2;

    fn neg(self) -> Vec2 {
        vec2(-self.x, -self.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Vec2) {
        *self = Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        };
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Vec2) {
        *self = Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        };
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, factor: f32) -> Vec2 {
        Vec2 {
            x: self.x * factor,
            y: self.y * factor,
        }
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;
    fn mul(self, vec: Vec2) -> Vec2 {
        Vec2 {
            x: self * vec.x,
            y: self * vec.y,
        }
    }
}

impl Div<f32> for Vec2 {
    type Output = Vec2;
    fn div(self, factor: f32) -> Vec2 {
        Vec2 {
            x: self.x / factor,
            y: self.y / factor,
        }
    }
}

impl std::fmt::Debug for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:.1} {:.1}]", self.x, self.y)
    }
}
