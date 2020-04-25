use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub fn vec2(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}

impl Vec2 {
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

    pub fn rot90(self) -> Self {
        vec2(self.y, -self.x)
    }

    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn dist(a: Self, b: Self) -> f32 {
        (a - b).length()
    }

    pub fn dist_sq(a: Self, b: Self) -> f32 {
        (a - b).length_sq()
    }

    pub fn angled(angle: f32) -> Self {
        vec2(angle.cos(), angle.sin())
    }

    pub fn floor(self) -> Self {
        vec2(self.x.floor(), self.y.floor())
    }

    pub fn round(self) -> Self {
        vec2(self.x.round(), self.y.round())
    }

    pub fn ceil(self) -> Self {
        vec2(self.x.ceil(), self.y.ceil())
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    pub fn min(self, other: Self) -> Self {
        vec2(self.x.min(other.x), self.y.min(other.y))
    }

    pub fn max(self, other: Self) -> Self {
        vec2(self.x.max(other.x), self.y.max(other.y))
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
        write!(f, "[{} {}]", self.x, self.y)
    }
}

// ----------------------------------------------------------------------------

/// Sometimes called a Point. I prefer the shorter Pos2 so it is equal length to Vec2
#[derive(Clone, Copy, Default, Deserialize, Serialize)]
pub struct Pos2 {
    pub x: f32,
    pub y: f32,
    // implicit w = 1
}

pub fn pos2(x: f32, y: f32) -> Pos2 {
    Pos2 { x, y }
}

impl Pos2 {
    pub fn dist(self: Self, other: Self) -> f32 {
        (self - other).length()
    }

    pub fn dist_sq(self: Self, other: Self) -> f32 {
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

    pub fn min(self, other: Self) -> Self {
        pos2(self.x.min(other.x), self.y.min(other.y))
    }

    pub fn max(self, other: Self) -> Self {
        pos2(self.x.max(other.x), self.y.max(other.y))
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
        write!(f, "[{} {}]", self.x, self.y)
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct Rect {
    min: Pos2,
    max: Pos2,
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

    /// Expand by this much in each direction
    #[must_use]
    pub fn expand(self, amnt: f32) -> Self {
        Rect::from_center_size(self.center(), self.size() + 2.0 * vec2(amnt, amnt))
    }

    #[must_use]
    pub fn translate(self, amnt: Vec2) -> Self {
        Rect::from_min_size(self.min() + amnt, self.size())
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

    pub fn contains(&self, p: Pos2) -> bool {
        self.min.x <= p.x
            && p.x <= self.min.x + self.size().x
            && self.min.y <= p.y
            && p.y <= self.min.y + self.size().y
    }

    pub fn center(&self) -> Pos2 {
        Pos2 {
            x: self.min.x + self.size().x / 2.0,
            y: self.min.y + self.size().y / 2.0,
        }
    }
    pub fn min(&self) -> Pos2 {
        self.min
    }
    pub fn max(&self) -> Pos2 {
        self.max
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

pub fn lerp<T>(min: T, max: T, t: f32) -> T
where
    f32: Mul<T, Output = T>,
    T: Add<T, Output = T>,
{
    (1.0 - t) * min + t * max
}

pub fn remap(from: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    let t = (from - from_min) / (from_max - from_min);
    lerp(to_min, to_max, t)
}

pub fn remap_clamp(from: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    let t = if from <= from_min {
        0.0
    } else if from >= from_max {
        1.0
    } else {
        (from - from_min) / (from_max - from_min)
    };
    lerp(to_min, to_max, t)
}

pub fn clamp(x: f32, min: f32, max: f32) -> f32 {
    if x <= min {
        min
    } else if x >= max {
        max
    } else {
        x
    }
}

/// For t=[0,1], returns [0,1] with a derivate of zero at both ends
pub fn ease_in_ease_out(t: f32) -> f32 {
    3.0 * t * t - 2.0 * t * t * t
}

pub const TAU: f32 = 2.0 * std::f32::consts::PI;
