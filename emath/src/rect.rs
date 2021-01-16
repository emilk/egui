use std::ops::RangeInclusive;

use crate::*;

/// A rectangular region of space.
///
/// Normally given in points, e.g. logical pixels.
#[derive(Clone, Copy, Eq, PartialEq)]
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

    /// invalid, NAN filled Rect.
    pub fn invalid() -> Self {
        Self {
            min: pos2(f32::NAN, f32::NAN),
            max: pos2(f32::NAN, f32::NAN),
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

    pub fn from_x_y_ranges(x_range: RangeInclusive<f32>, y_range: RangeInclusive<f32>) -> Self {
        Rect {
            min: pos2(*x_range.start(), *y_range.start()),
            max: pos2(*x_range.end(), *y_range.end()),
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

    #[must_use]
    pub fn intersects(self, other: Rect) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
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

    /// Return the given points clamped to be inside the rectangle
    #[must_use]
    pub fn clamp(&self, mut p: Pos2) -> Pos2 {
        p.x = clamp(p.x, self.x_range());
        p.y = clamp(p.y, self.y_range());
        p
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

    pub fn x_range(&self) -> RangeInclusive<f32> {
        self.min.x..=self.max.x
    }
    pub fn y_range(&self) -> RangeInclusive<f32> {
        self.min.y..=self.max.y
    }
    pub fn bottom_up_range(&self) -> RangeInclusive<f32> {
        self.max.y..=self.min.y
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

/// from (min, max) or (left top, right bottom)
impl From<[Pos2; 2]> for Rect {
    fn from([min, max]: [Pos2; 2]) -> Self {
        Self { min, max }
    }
}
