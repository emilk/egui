use std::ops::{Add, AddAssign, RangeInclusive, Sub, SubAssign};

use crate::*;

/// A position on screen.
///
/// Normally given in points (logical pixels).
///
/// Mathematically this is known as a "point", but the term position was chosen so not to
/// conflict with the unit (one point = X physical pixels).
#[derive(Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Pos2 {
    pub x: f32,
    pub y: f32,
    // implicit w = 1
}

/// `pos2(x,y) == Pos2::new(x, y)`
#[inline(always)]
pub const fn pos2(x: f32, y: f32) -> Pos2 {
    Pos2 { x, y }
}

// ----------------------------------------------------------------------------
// Compatibility and convenience conversions to and from [f32; 2]:

impl From<[f32; 2]> for Pos2 {
    fn from(v: [f32; 2]) -> Self {
        Self { x: v[0], y: v[1] }
    }
}

impl From<&[f32; 2]> for Pos2 {
    fn from(v: &[f32; 2]) -> Self {
        Self { x: v[0], y: v[1] }
    }
}

impl From<Pos2> for [f32; 2] {
    fn from(v: Pos2) -> Self {
        [v.x, v.y]
    }
}

impl From<&Pos2> for [f32; 2] {
    fn from(v: &Pos2) -> Self {
        [v.x, v.y]
    }
}

// ----------------------------------------------------------------------------
// Compatibility and convenience conversions to and from (f32, f32):

impl From<(f32, f32)> for Pos2 {
    fn from(v: (f32, f32)) -> Self {
        Self { x: v.0, y: v.1 }
    }
}

impl From<&(f32, f32)> for Pos2 {
    fn from(v: &(f32, f32)) -> Self {
        Self { x: v.0, y: v.1 }
    }
}

impl From<Pos2> for (f32, f32) {
    fn from(v: Pos2) -> Self {
        (v.x, v.y)
    }
}

impl From<&Pos2> for (f32, f32) {
    fn from(v: &Pos2) -> Self {
        (v.x, v.y)
    }
}

// ----------------------------------------------------------------------------

impl Pos2 {
    /// The zero position, the origin.
    /// The top left corner in a GUI.
    /// Same as `Pos2::default()`.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    #[deprecated = "Use Pos2::ZERO instead"]
    pub const fn zero() -> Self {
        Self::ZERO
    }

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// The vector from origin to this position.
    /// `p.to_vec2()` is equivalent to `p - Pos2::default()`.
    pub fn to_vec2(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y,
        }
    }

    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    pub fn distance_sq(self, other: Self) -> f32 {
        (self - other).length_sq()
    }

    pub fn floor(self) -> Self {
        pos2(self.x.floor(), self.y.floor())
    }

    pub fn round(self) -> Self {
        pos2(self.x.round(), self.y.round())
    }

    pub fn ceil(self) -> Self {
        pos2(self.x.ceil(), self.y.ceil())
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    #[must_use]
    pub fn min(self, other: Self) -> Self {
        pos2(self.x.min(other.x), self.y.min(other.y))
    }

    #[must_use]
    pub fn max(self, other: Self) -> Self {
        pos2(self.x.max(other.x), self.y.max(other.y))
    }

    #[must_use]
    pub fn clamp(self, range: RangeInclusive<Self>) -> Self {
        Self {
            x: clamp(self.x, range.start().x..=range.end().x),
            y: clamp(self.y, range.start().y..=range.end().y),
        }
    }
}

impl std::ops::Index<usize> for Pos2 {
    type Output = f32;
    fn index(&self, index: usize) -> &f32 {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("Pos2 index out of bounds: {}", index),
        }
    }
}

impl std::ops::IndexMut<usize> for Pos2 {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => panic!("Pos2 index out of bounds: {}", index),
        }
    }
}

impl Eq for Pos2 {}

impl AddAssign<Vec2> for Pos2 {
    fn add_assign(&mut self, rhs: Vec2) {
        *self = Pos2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        };
    }
}

impl SubAssign<Vec2> for Pos2 {
    fn sub_assign(&mut self, rhs: Vec2) {
        *self = Pos2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        };
    }
}

impl Add<Vec2> for Pos2 {
    type Output = Pos2;
    fn add(self, rhs: Vec2) -> Pos2 {
        Pos2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Pos2 {
    type Output = Vec2;
    fn sub(self, rhs: Pos2) -> Vec2 {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Sub<Vec2> for Pos2 {
    type Output = Pos2;
    fn sub(self, rhs: Vec2) -> Pos2 {
        Pos2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::fmt::Debug for Pos2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:.1} {:.1}]", self.x, self.y)
    }
}
