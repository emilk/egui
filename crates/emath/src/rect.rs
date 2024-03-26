use std::f32::INFINITY;

use crate::*;

/// A rectangular region of space.
///
/// Usually a [`Rect`] has a positive (or zero) size,
/// and then [`Self::min`] `<=` [`Self::max`].
/// In these cases [`Self::min`] is the left-top corner
/// and [`Self::max`] is the right-bottom corner.
///
/// A rectangle is allowed to have a negative size, which happens when the order
/// of `min` and `max` are swapped. These are usually a sign of an error.
///
/// Normally the unit is points (logical pixels) in screen space coordinates.
///
/// `Rect` does NOT implement `Default`, because there is no obvious default value.
/// [`Rect::ZERO`] may seem reasonable, but when used as a bounding box, [`Rect::NOTHING`]
/// is a better default - so be explicit instead!
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Rect {
    /// One of the corners of the rectangle, usually the left top one.
    pub min: Pos2,

    /// The other corner, opposing [`Self::min`]. Usually the right bottom one.
    pub max: Pos2,
}

impl Rect {
    /// Infinite rectangle that contains every point.
    pub const EVERYTHING: Self = Self {
        min: pos2(-INFINITY, -INFINITY),
        max: pos2(INFINITY, INFINITY),
    };

    /// The inverse of [`Self::EVERYTHING`]: stretches from positive infinity to negative infinity.
    /// Contains no points.
    ///
    /// This is useful as the seed for bounding boxes.
    ///
    /// # Example:
    /// ```
    /// # use emath::*;
    /// let mut rect = Rect::NOTHING;
    /// assert!(rect.size() == Vec2::splat(-f32::INFINITY));
    /// assert!(rect.contains(pos2(0.0, 0.0)) == false);
    /// rect.extend_with(pos2(2.0, 1.0));
    /// rect.extend_with(pos2(0.0, 3.0));
    /// assert_eq!(rect, Rect::from_min_max(pos2(0.0, 1.0), pos2(2.0, 3.0)))
    /// ```
    pub const NOTHING: Self = Self {
        min: pos2(INFINITY, INFINITY),
        max: pos2(-INFINITY, -INFINITY),
    };

    /// An invalid [`Rect`] filled with [`f32::NAN`].
    pub const NAN: Self = Self {
        min: pos2(f32::NAN, f32::NAN),
        max: pos2(f32::NAN, f32::NAN),
    };

    /// A [`Rect`] filled with zeroes.
    pub const ZERO: Self = Self {
        min: Pos2::ZERO,
        max: Pos2::ZERO,
    };

    #[inline(always)]
    pub const fn from_min_max(min: Pos2, max: Pos2) -> Self {
        Self { min, max }
    }

    /// left-top corner plus a size (stretching right-down).
    #[inline(always)]
    pub fn from_min_size(min: Pos2, size: Vec2) -> Self {
        Self {
            min,
            max: min + size,
        }
    }

    #[inline(always)]
    pub fn from_center_size(center: Pos2, size: Vec2) -> Self {
        Self {
            min: center - size * 0.5,
            max: center + size * 0.5,
        }
    }

    #[inline(always)]
    pub fn from_x_y_ranges(x_range: impl Into<Rangef>, y_range: impl Into<Rangef>) -> Self {
        let x_range = x_range.into();
        let y_range = y_range.into();
        Self {
            min: pos2(x_range.min, y_range.min),
            max: pos2(x_range.max, y_range.max),
        }
    }

    /// Returns the bounding rectangle of the two points.
    #[inline]
    pub fn from_two_pos(a: Pos2, b: Pos2) -> Self {
        Self {
            min: pos2(a.x.min(b.x), a.y.min(b.y)),
            max: pos2(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    /// Bounding-box around the points.
    pub fn from_points(points: &[Pos2]) -> Self {
        let mut rect = Self::NOTHING;
        for &p in points {
            rect.extend_with(p);
        }
        rect
    }

    /// A [`Rect`] that contains every point to the right of the given X coordinate.
    #[inline]
    pub fn everything_right_of(left_x: f32) -> Self {
        let mut rect = Self::EVERYTHING;
        rect.set_left(left_x);
        rect
    }

    /// A [`Rect`] that contains every point to the left of the given X coordinate.
    #[inline]
    pub fn everything_left_of(right_x: f32) -> Self {
        let mut rect = Self::EVERYTHING;
        rect.set_right(right_x);
        rect
    }

    /// A [`Rect`] that contains every point below a certain y coordinate
    #[inline]
    pub fn everything_below(top_y: f32) -> Self {
        let mut rect = Self::EVERYTHING;
        rect.set_top(top_y);
        rect
    }

    /// A [`Rect`] that contains every point above a certain y coordinate
    #[inline]
    pub fn everything_above(bottom_y: f32) -> Self {
        let mut rect = Self::EVERYTHING;
        rect.set_bottom(bottom_y);
        rect
    }

    #[must_use]
    #[inline]
    pub fn with_min_x(mut self, min_x: f32) -> Self {
        self.min.x = min_x;
        self
    }

    #[must_use]
    #[inline]
    pub fn with_min_y(mut self, min_y: f32) -> Self {
        self.min.y = min_y;
        self
    }

    #[must_use]
    #[inline]
    pub fn with_max_x(mut self, max_x: f32) -> Self {
        self.max.x = max_x;
        self
    }

    #[must_use]
    #[inline]
    pub fn with_max_y(mut self, max_y: f32) -> Self {
        self.max.y = max_y;
        self
    }

    /// Expand by this much in each direction, keeping the center
    #[must_use]
    pub fn expand(self, amnt: f32) -> Self {
        self.expand2(Vec2::splat(amnt))
    }

    /// Expand by this much in each direction, keeping the center
    #[must_use]
    pub fn expand2(self, amnt: Vec2) -> Self {
        Self::from_min_max(self.min - amnt, self.max + amnt)
    }

    /// Shrink by this much in each direction, keeping the center
    #[must_use]
    pub fn shrink(self, amnt: f32) -> Self {
        self.shrink2(Vec2::splat(amnt))
    }

    /// Shrink by this much in each direction, keeping the center
    #[must_use]
    pub fn shrink2(self, amnt: Vec2) -> Self {
        Self::from_min_max(self.min + amnt, self.max - amnt)
    }

    #[must_use]
    #[inline]
    pub fn translate(self, amnt: Vec2) -> Self {
        Self::from_min_size(self.min + amnt, self.size())
    }

    /// Rotate the bounds (will expand the [`Rect`])
    #[must_use]
    #[inline]
    pub fn rotate_bb(self, rot: Rot2) -> Self {
        let a = rot * self.left_top().to_vec2();
        let b = rot * self.right_top().to_vec2();
        let c = rot * self.left_bottom().to_vec2();
        let d = rot * self.right_bottom().to_vec2();

        Self::from_min_max(
            a.min(b).min(c).min(d).to_pos2(),
            a.max(b).max(c).max(d).to_pos2(),
        )
    }

    #[must_use]
    #[inline]
    pub fn intersects(self, other: Self) -> bool {
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
    #[inline(always)]
    pub fn contains(&self, p: Pos2) -> bool {
        self.min.x <= p.x && p.x <= self.max.x && self.min.y <= p.y && p.y <= self.max.y
    }

    #[must_use]
    pub fn contains_rect(&self, other: Self) -> bool {
        self.contains(other.min) && self.contains(other.max)
    }

    /// Return the given points clamped to be inside the rectangle
    /// Panics if [`Self::is_negative`].
    #[must_use]
    pub fn clamp(&self, p: Pos2) -> Pos2 {
        p.clamp(self.min, self.max)
    }

    #[inline(always)]
    pub fn extend_with(&mut self, p: Pos2) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    #[inline(always)]
    /// Expand to include the given x coordinate
    pub fn extend_with_x(&mut self, x: f32) {
        self.min.x = self.min.x.min(x);
        self.max.x = self.max.x.max(x);
    }

    #[inline(always)]
    /// Expand to include the given y coordinate
    pub fn extend_with_y(&mut self, y: f32) {
        self.min.y = self.min.y.min(y);
        self.max.y = self.max.y.max(y);
    }

    /// The union of two bounding rectangle, i.e. the minimum [`Rect`]
    /// that contains both input rectangles.
    #[inline(always)]
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// The intersection of two [`Rect`], i.e. the area covered by both.
    #[inline]
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        }
    }

    #[inline(always)]
    pub fn center(&self) -> Pos2 {
        Pos2 {
            x: (self.min.x + self.max.x) / 2.0,
            y: (self.min.y + self.max.y) / 2.0,
        }
    }

    /// `rect.size() == Vec2 { x: rect.width(), y: rect.height() }`
    #[inline(always)]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    #[inline(always)]
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    #[inline(always)]
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Width / height
    ///
    /// * `aspect_ratio < 1`: portrait / high
    /// * `aspect_ratio = 1`: square
    /// * `aspect_ratio > 1`: landscape / wide
    pub fn aspect_ratio(&self) -> f32 {
        self.width() / self.height()
    }

    /// `[2, 1]` for wide screen, and `[1, 2]` for portrait, etc.
    /// At least one dimension = 1, the other >= 1
    /// Returns the proportions required to letter-box a square view area.
    pub fn square_proportions(&self) -> Vec2 {
        let w = self.width();
        let h = self.height();
        if w > h {
            vec2(w / h, 1.0)
        } else {
            vec2(1.0, h / w)
        }
    }

    #[inline(always)]
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    /// The distance from the rect to the position.
    ///
    /// The distance is zero when the position is in the interior of the rectangle.
    #[inline]
    pub fn distance_to_pos(&self, pos: Pos2) -> f32 {
        self.distance_sq_to_pos(pos).sqrt()
    }

    /// The distance from the rect to the position, squared.
    ///
    /// The distance is zero when the position is in the interior of the rectangle.
    #[inline]
    pub fn distance_sq_to_pos(&self, pos: Pos2) -> f32 {
        let dx = if self.min.x > pos.x {
            self.min.x - pos.x
        } else if pos.x > self.max.x {
            pos.x - self.max.x
        } else {
            0.0
        };

        let dy = if self.min.y > pos.y {
            self.min.y - pos.y
        } else if pos.y > self.max.y {
            pos.y - self.max.y
        } else {
            0.0
        };

        dx * dx + dy * dy
    }

    /// Signed distance to the edge of the box.
    ///
    /// Negative inside the box.
    ///
    /// ```
    /// # use emath::{pos2, Rect};
    /// let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    /// assert_eq!(rect.signed_distance_to_pos(pos2(0.50, 0.50)), -0.50);
    /// assert_eq!(rect.signed_distance_to_pos(pos2(0.75, 0.50)), -0.25);
    /// assert_eq!(rect.signed_distance_to_pos(pos2(1.50, 0.50)), 0.50);
    /// ```
    pub fn signed_distance_to_pos(&self, pos: Pos2) -> f32 {
        let edge_distances = (pos - self.center()).abs() - self.size() * 0.5;
        let inside_dist = edge_distances.max_elem().min(0.0);
        let outside_dist = edge_distances.max(Vec2::ZERO).length();
        inside_dist + outside_dist
    }

    /// Linearly interpolate so that `[0, 0]` is [`Self::min`] and
    /// `[1, 1]` is [`Self::max`].
    #[inline]
    pub fn lerp_inside(&self, t: Vec2) -> Pos2 {
        Pos2 {
            x: lerp(self.min.x..=self.max.x, t.x),
            y: lerp(self.min.y..=self.max.y, t.y),
        }
    }

    /// Linearly self towards other rect.
    #[inline]
    pub fn lerp_towards(&self, other: &Self, t: f32) -> Self {
        Self {
            min: self.min.lerp(other.min, t),
            max: self.max.lerp(other.max, t),
        }
    }

    #[inline(always)]
    pub fn x_range(&self) -> Rangef {
        Rangef::new(self.min.x, self.max.x)
    }

    #[inline(always)]
    pub fn y_range(&self) -> Rangef {
        Rangef::new(self.min.y, self.max.y)
    }

    #[inline(always)]
    pub fn bottom_up_range(&self) -> Rangef {
        Rangef::new(self.max.y, self.min.y)
    }

    /// `width < 0 || height < 0`
    #[inline(always)]
    pub fn is_negative(&self) -> bool {
        self.max.x < self.min.x || self.max.y < self.min.y
    }

    /// `width > 0 && height > 0`
    #[inline(always)]
    pub fn is_positive(&self) -> bool {
        self.min.x < self.max.x && self.min.y < self.max.y
    }

    /// True if all members are also finite.
    #[inline(always)]
    pub fn is_finite(&self) -> bool {
        self.min.is_finite() && self.max.is_finite()
    }

    /// True if any member is NaN.
    #[inline(always)]
    pub fn any_nan(self) -> bool {
        self.min.any_nan() || self.max.any_nan()
    }
}

/// ## Convenience functions (assumes origin is towards left top):
impl Rect {
    /// `min.x`
    #[inline(always)]
    pub fn left(&self) -> f32 {
        self.min.x
    }

    /// `min.x`
    #[inline(always)]
    pub fn left_mut(&mut self) -> &mut f32 {
        &mut self.min.x
    }

    /// `min.x`
    #[inline(always)]
    pub fn set_left(&mut self, x: f32) {
        self.min.x = x;
    }

    /// `max.x`
    #[inline(always)]
    pub fn right(&self) -> f32 {
        self.max.x
    }

    /// `max.x`
    #[inline(always)]
    pub fn right_mut(&mut self) -> &mut f32 {
        &mut self.max.x
    }

    /// `max.x`
    #[inline(always)]
    pub fn set_right(&mut self, x: f32) {
        self.max.x = x;
    }

    /// `min.y`
    #[inline(always)]
    pub fn top(&self) -> f32 {
        self.min.y
    }

    /// `min.y`
    #[inline(always)]
    pub fn top_mut(&mut self) -> &mut f32 {
        &mut self.min.y
    }

    /// `min.y`
    #[inline(always)]
    pub fn set_top(&mut self, y: f32) {
        self.min.y = y;
    }

    /// `max.y`
    #[inline(always)]
    pub fn bottom(&self) -> f32 {
        self.max.y
    }

    /// `max.y`
    #[inline(always)]
    pub fn bottom_mut(&mut self) -> &mut f32 {
        &mut self.max.y
    }

    /// `max.y`
    #[inline(always)]
    pub fn set_bottom(&mut self, y: f32) {
        self.max.y = y;
    }

    #[inline(always)]
    pub fn left_top(&self) -> Pos2 {
        pos2(self.left(), self.top())
    }

    #[inline(always)]
    pub fn center_top(&self) -> Pos2 {
        pos2(self.center().x, self.top())
    }

    #[inline(always)]
    pub fn right_top(&self) -> Pos2 {
        pos2(self.right(), self.top())
    }

    #[inline(always)]
    pub fn left_center(&self) -> Pos2 {
        pos2(self.left(), self.center().y)
    }

    #[inline(always)]
    pub fn right_center(&self) -> Pos2 {
        pos2(self.right(), self.center().y)
    }

    #[inline(always)]
    pub fn left_bottom(&self) -> Pos2 {
        pos2(self.left(), self.bottom())
    }

    #[inline(always)]
    pub fn center_bottom(&self) -> Pos2 {
        pos2(self.center().x, self.bottom())
    }

    #[inline(always)]
    pub fn right_bottom(&self) -> Pos2 {
        pos2(self.right(), self.bottom())
    }

    /// Split rectangle in left and right halves. `t` is expected to be in the (0,1) range.
    pub fn split_left_right_at_fraction(&self, t: f32) -> (Self, Self) {
        self.split_left_right_at_x(lerp(self.min.x..=self.max.x, t))
    }

    /// Split rectangle in left and right halves at the given `x` coordinate.
    pub fn split_left_right_at_x(&self, split_x: f32) -> (Self, Self) {
        let left = Self::from_min_max(self.min, Pos2::new(split_x, self.max.y));
        let right = Self::from_min_max(Pos2::new(split_x, self.min.y), self.max);
        (left, right)
    }

    /// Split rectangle in top and bottom halves. `t` is expected to be in the (0,1) range.
    pub fn split_top_bottom_at_fraction(&self, t: f32) -> (Self, Self) {
        self.split_top_bottom_at_y(lerp(self.min.y..=self.max.y, t))
    }

    /// Split rectangle in top and bottom halves at the given `y` coordinate.
    pub fn split_top_bottom_at_y(&self, split_y: f32) -> (Self, Self) {
        let top = Self::from_min_max(self.min, Pos2::new(self.max.x, split_y));
        let bottom = Self::from_min_max(Pos2::new(self.min.x, split_y), self.max);
        (top, bottom)
    }
}

impl Rect {
    /// Does this Rect intersect the given ray (where `d` is normalized)?
    pub fn intersects_ray(&self, o: Pos2, d: Vec2) -> bool {
        let mut tmin = -f32::INFINITY;
        let mut tmax = f32::INFINITY;

        if d.x != 0.0 {
            let tx1 = (self.min.x - o.x) / d.x;
            let tx2 = (self.max.x - o.x) / d.x;

            tmin = tmin.max(tx1.min(tx2));
            tmax = tmax.min(tx1.max(tx2));
        }

        if d.y != 0.0 {
            let ty1 = (self.min.y - o.y) / d.y;
            let ty2 = (self.max.y - o.y) / d.y;

            tmin = tmin.max(ty1.min(ty2));
            tmax = tmax.min(ty1.max(ty2));
        }

        tmin <= tmax
    }
}

impl std::fmt::Debug for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?} - {:?}]", self.min, self.max)
    }
}

/// from (min, max) or (left top, right bottom)
impl From<[Pos2; 2]> for Rect {
    #[inline]
    fn from([min, max]: [Pos2; 2]) -> Self {
        Self { min, max }
    }
}

impl Mul<f32> for Rect {
    type Output = Self;

    #[inline]
    fn mul(self, factor: f32) -> Self {
        Self {
            min: self.min * factor,
            max: self.max * factor,
        }
    }
}

impl Mul<Rect> for f32 {
    type Output = Rect;

    #[inline]
    fn mul(self, vec: Rect) -> Rect {
        Rect {
            min: self * vec.min,
            max: self * vec.max,
        }
    }
}

impl Div<f32> for Rect {
    type Output = Self;

    #[inline]
    fn div(self, factor: f32) -> Self {
        Self {
            min: self.min / factor,
            max: self.max / factor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect() {
        let r = Rect::from_min_max(pos2(10.0, 10.0), pos2(20.0, 20.0));
        assert_eq!(r.distance_sq_to_pos(pos2(15.0, 15.0)), 0.0);
        assert_eq!(r.distance_sq_to_pos(pos2(10.0, 15.0)), 0.0);
        assert_eq!(r.distance_sq_to_pos(pos2(10.0, 10.0)), 0.0);

        assert_eq!(r.distance_sq_to_pos(pos2(5.0, 15.0)), 25.0); // left of
        assert_eq!(r.distance_sq_to_pos(pos2(25.0, 15.0)), 25.0); // right of
        assert_eq!(r.distance_sq_to_pos(pos2(15.0, 5.0)), 25.0); // above
        assert_eq!(r.distance_sq_to_pos(pos2(15.0, 25.0)), 25.0); // below
        assert_eq!(r.distance_sq_to_pos(pos2(25.0, 5.0)), 50.0); // right and above
    }
}
