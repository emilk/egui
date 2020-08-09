//! Vectors, positions, rectangles etc.

use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, RangeInclusive, Sub, SubAssign};

/// A size or direction in 2D space.
///
/// Normally given in points, e.g. logical pixels.
#[derive(Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[inline(always)]
pub fn vec2(x: f32, y: f32) -> Vec2 {
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

    pub fn distance(a: Self, b: Self) -> f32 {
        (a - b).length()
    }

    pub fn distance_sq(a: Self, b: Self) -> f32 {
        (a - b).length_sq()
    }

    pub fn angled(angle: f32) -> Self {
        vec2(angle.cos(), angle.sin())
    }

    /// Use this vector as a rotor, rotating something else.
    /// Example: `Vec2::angled(angle).rotate_other(some_vec)`
    #[must_use]
    pub fn rotate_other(self, v: Vec2) -> Self {
        Self {
            x: v.x * self.x + v.y * -self.y,
            y: v.x * self.y + v.y * self.x,
        }
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

// ----------------------------------------------------------------------------

// Sometimes called a Point. I prefer the shorter Pos2 so it is equal length to Vec2
/// A position on screen.
///
/// Normally given in points, e.g. logical pixels.
#[derive(Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Pos2 {
    pub x: f32,
    pub y: f32,
    // implicit w = 1
}

pub fn pos2(x: f32, y: f32) -> Pos2 {
    Pos2 { x, y }
}

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

impl Pos2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn to_vec2(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y,
        }
    }

    pub fn distance(self: Self, other: Self) -> f32 {
        (self - other).length()
    }

    pub fn distance_sq(self: Self, other: Self) -> f32 {
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

impl PartialEq for Pos2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
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

// ----------------------------------------------------------------------------

/// A rectangular region of space.
///
/// Normally given in points, e.g. logical pixels.
#[derive(Clone, Copy, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Rect {
    pub min: Pos2,
    pub max: Pos2,
}

impl Rect {
    /// Infinite rectangle that contains everything
    pub fn everything() -> Self {
        let inf = f32::INFINITY;
        Self {
            min: pos2(-inf, -inf),
            max: pos2(inf, inf),
        }
    }

    pub fn nothing() -> Self {
        let inf = f32::INFINITY;
        Self {
            min: pos2(inf, inf),
            max: pos2(-inf, -inf),
        }
    }

    pub fn from_min_max(min: Pos2, max: Pos2) -> Self {
        Rect { min, max }
    }

    pub fn from_min_size(min: Pos2, size: Vec2) -> Self {
        Rect {
            min,
            max: min + size,
        }
    }

    pub fn from_center_size(center: Pos2, size: Vec2) -> Self {
        Rect {
            min: center - size * 0.5,
            max: center + size * 0.5,
        }
    }

    /// Expand by this much in each direction, keeping the center
    #[must_use]
    pub fn expand(self, amnt: f32) -> Self {
        self.expand2(Vec2::splat(amnt))
    }

    /// Expand by this much in each direction, keeping the center
    #[must_use]
    pub fn expand2(self, amnt: Vec2) -> Self {
        Rect::from_min_max(self.min - amnt, self.max + amnt)
    }

    /// Shrink by this much in each direction, keeping the center
    #[must_use]
    pub fn shrink(self, amnt: f32) -> Self {
        self.shrink2(Vec2::splat(amnt))
    }

    /// Shrink by this much in each direction, keeping the center
    #[must_use]
    pub fn shrink2(self, amnt: Vec2) -> Self {
        Rect::from_min_max(self.min + amnt, self.max - amnt)
    }

    #[must_use]
    pub fn translate(self, amnt: Vec2) -> Self {
        Rect::from_min_size(self.min + amnt, self.size())
    }

    #[must_use]
    pub fn intersect(self, other: Rect) -> Self {
        Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        }
    }

    /// keep min
    pub fn set_width(&mut self, w: f32) {
        self.max.x = self.min.x + w;
    }

    /// keep min
    pub fn set_height(&mut self, h: f32) {
        self.max.y = self.min.y + h;
    }

    /// Keep size
    pub fn set_center(&mut self, center: Pos2) {
        *self = self.translate(center - self.center());
    }

    #[must_use]
    pub fn contains(&self, p: Pos2) -> bool {
        self.min.x <= p.x
            && p.x <= self.min.x + self.size().x
            && self.min.y <= p.y
            && p.y <= self.min.y + self.size().y
    }

    pub fn extend_with(&mut self, p: Pos2) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    pub fn union(self, other: Rect) -> Rect {
        Rect {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn center(&self) -> Pos2 {
        Pos2 {
            x: self.min.x + self.size().x / 2.0,
            y: self.min.y + self.size().y / 2.0,
        }
    }
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    pub fn range_x(&self) -> RangeInclusive<f32> {
        self.min.x..=self.max.x
    }

    pub fn range_y(&self) -> RangeInclusive<f32> {
        self.min.y..=self.max.y
    }

    pub fn is_empty(&self) -> bool {
        self.max.x < self.min.x || self.max.y < self.min.y
    }

    pub fn is_finite(&self) -> bool {
        self.min.is_finite() && self.max.is_finite()
    }

    // Convenience functions (assumes origin is towards left top):
    pub fn left(&self) -> f32 {
        self.min.x
    }
    pub fn right(&self) -> f32 {
        self.max.x
    }
    pub fn top(&self) -> f32 {
        self.min.y
    }
    pub fn bottom(&self) -> f32 {
        self.max.y
    }
    pub fn left_top(&self) -> Pos2 {
        pos2(self.left(), self.top())
    }
    pub fn center_top(&self) -> Pos2 {
        pos2(self.center().x, self.top())
    }
    pub fn right_top(&self) -> Pos2 {
        pos2(self.right(), self.top())
    }
    pub fn left_center(&self) -> Pos2 {
        pos2(self.left(), self.center().y)
    }
    pub fn right_center(&self) -> Pos2 {
        pos2(self.right(), self.center().y)
    }
    pub fn left_bottom(&self) -> Pos2 {
        pos2(self.left(), self.bottom())
    }
    pub fn center_bottom(&self) -> Pos2 {
        pos2(self.center().x, self.bottom())
    }
    pub fn right_bottom(&self) -> Pos2 {
        pos2(self.right(), self.bottom())
    }
}

impl std::fmt::Debug for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?} - {:?}]", self.min, self.max)
    }
}

// ----------------------------------------------------------------------------

/// Linear interpolation.
pub fn lerp<T>(range: RangeInclusive<T>, t: f32) -> T
where
    f32: Mul<T, Output = T>,
    T: Add<T, Output = T> + Copy,
{
    (1.0 - t) * *range.start() + t * *range.end()
}

/// Linearly remap a value from one range to another,
/// so that when `x == from.start()` returns `to.start()`
/// and when `x == from.end()` returns `to.end()`.
pub fn remap(x: f32, from: RangeInclusive<f32>, to: RangeInclusive<f32>) -> f32 {
    let t = (x - from.start()) / (from.end() - from.start());
    lerp(to, t)
}

/// Like `remap`, but also clamps the value so that the returned value is always in the `to` range.
pub fn remap_clamp(x: f32, from: RangeInclusive<f32>, to: RangeInclusive<f32>) -> f32 {
    if x <= *from.start() {
        *to.start()
    } else if *from.end() <= x {
        *to.end()
    } else {
        let t = (x - from.start()) / (from.end() - from.start());
        // Ensure no numerical inaccurcies sneak in:
        if 1.0 <= t {
            *to.end()
        } else {
            lerp(to, t)
        }
    }
}

/// Returns `range.start()` if `x <= range.start()`,
/// returns `range.end()` if `x >= range.end()`
/// and returns `x` elsewhen.
pub fn clamp<T>(x: T, range: RangeInclusive<T>) -> T
where
    T: Copy + PartialOrd,
{
    if x <= *range.start() {
        *range.start()
    } else if *range.end() <= x {
        *range.end()
    } else {
        x
    }
}

/// For t=[0,1], returns [0,1] with a derivate of zero at both ends
pub fn ease_in_ease_out(t: f32) -> f32 {
    3.0 * t * t - 2.0 * t * t * t
}

/// The circumference of a circle divided by its radius.
///
/// Represents one turn in radian angles. Equal to `2 * pi`.
///
/// See <https://tauday.com/>
pub const TAU: f32 = 2.0 * std::f32::consts::PI;

/// Round a value to the given number of decimal places.
pub fn round_to_precision(value: f32, decimal_places: usize) -> f32 {
    // This is a stupid way of doing this, but stupid works.
    format!("{:.*}", decimal_places, value)
        .parse()
        .unwrap_or_else(|_| value)
}
