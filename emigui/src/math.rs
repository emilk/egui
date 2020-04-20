#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl PartialEq for Vec2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}
impl Eq for Vec2 {}

impl Vec2 {
    pub fn splat(v: impl Into<f32>) -> Vec2 {
        let v: f32 = v.into();
        Vec2 { x: v, y: v }
    }

    #[must_use]
    pub fn normalized(self) -> Vec2 {
        let len = self.length();
        if len <= 0.0 {
            self
        } else {
            self / len
        }
    }

    pub fn rot90(self) -> Vec2 {
        vec2(self.y, -self.x)
    }

    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn dist(a: Vec2, b: Vec2) -> f32 {
        (a - b).length()
    }

    pub fn dist_sq(a: Vec2, b: Vec2) -> f32 {
        (a - b).length_sq()
    }

    pub fn angled(angle: f32) -> Vec2 {
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

    pub fn min(self, other: Vec2) -> Self {
        vec2(self.x.min(other.x), self.y.min(other.y))
    }

    pub fn max(self, other: Vec2) -> Self {
        vec2(self.x.max(other.x), self.y.max(other.y))
    }
}

impl std::ops::Neg for Vec2 {
    type Output = Vec2;

    fn neg(self) -> Vec2 {
        vec2(-self.x, -self.y)
    }
}

impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Vec2) {
        *self = Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        };
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, factor: f32) -> Vec2 {
        Vec2 {
            x: self.x * factor,
            y: self.y * factor,
        }
    }
}

impl std::ops::Mul<Vec2> for f32 {
    type Output = Vec2;
    fn mul(self, vec: Vec2) -> Vec2 {
        Vec2 {
            x: self * vec.x,
            y: self * vec.y,
        }
    }
}

impl std::ops::Div<f32> for Vec2 {
    type Output = Vec2;
    fn div(self, factor: f32) -> Vec2 {
        Vec2 {
            x: self.x / factor,
            y: self.y / factor,
        }
    }
}

pub fn vec2(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}

// ----------------------------------------------------------------------------

/// Sometimes called a Point. I prefer the shorter Pos2 so it is equal length to Vec2
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct Pos2 {
    pub x: f32,
    pub y: f32,
    // implicit w = 1
}

impl PartialEq for Pos2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}
impl Eq for Pos2 {}

impl Pos2 {
    pub fn dist(self: Pos2, other: Pos2) -> f32 {
        (self - other).length()
    }

    pub fn dist_sq(self: Pos2, other: Pos2) -> f32 {
        (self - other).length_sq()
    }

    // TODO: remove?
    pub fn to_vec2(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y,
        }
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
}

impl std::ops::AddAssign<Vec2> for Pos2 {
    fn add_assign(&mut self, rhs: Vec2) {
        *self = Pos2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        };
    }
}

impl std::ops::Add<Vec2> for Pos2 {
    type Output = Pos2;
    fn add(self, rhs: Vec2) -> Pos2 {
        Pos2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

// impl std::ops::Add<Pos2> for Vec2 {
//     type Output = Pos2;
//     fn add(self, rhs: Pos2) -> Pos2 {
//         Pos2 {
//             x: self.x + rhs.x,
//             y: self.y + rhs.y,
//         }
//     }
// }

impl std::ops::Sub for Pos2 {
    type Output = Vec2;
    fn sub(self, rhs: Pos2) -> Vec2 {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Sub<Vec2> for Pos2 {
    type Output = Pos2;
    fn sub(self, rhs: Vec2) -> Pos2 {
        Pos2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

pub fn pos2(x: f32, y: f32) -> Pos2 {
    Pos2 { x, y }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct Rect {
    min: Pos2,
    max: Pos2,
}

impl Rect {
    /// Infinite rectangle that contains everything
    pub fn everything() -> Self {
        let inf = std::f32::INFINITY;
        Self {
            min: pos2(-inf, -inf),
            max: pos2(inf, inf),
        }
    }

    pub fn nothing() -> Self {
        let inf = std::f32::INFINITY;
        Self {
            min: pos2(inf, inf),
            max: pos2(-inf, -inf),
        }
    }

    pub fn from_min_max(min: Pos2, max: Pos2) -> Self {
        Rect { min, max: max }
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
    pub fn expand(self, amnt: f32) -> Self {
        Rect::from_center_size(self.center(), self.size() + 2.0 * vec2(amnt, amnt))
    }

    pub fn translate(self, amnt: Vec2) -> Self {
        Rect::from_min_size(self.min() + amnt, self.size())
    }

    // keep min
    pub fn set_width(&mut self, w: f32) {
        self.max.x = self.min.x + w;
    }

    // keep min
    pub fn set_height(&mut self, h: f32) {
        self.max.y = self.min.y + h;
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

    // Convenience functions (assumes origin is towards left top):

    pub fn left_top(&self) -> Pos2 {
        pos2(self.min().x, self.min().y)
    }
    pub fn center_top(&self) -> Pos2 {
        pos2(self.center().x, self.min().y)
    }
    pub fn right_top(&self) -> Pos2 {
        pos2(self.max().x, self.min().y)
    }
    pub fn left_center(&self) -> Pos2 {
        pos2(self.min().x, self.center().y)
    }
    pub fn right_center(&self) -> Pos2 {
        pos2(self.max().x, self.center().y)
    }
    pub fn left_bottom(&self) -> Pos2 {
        pos2(self.min().x, self.max().y)
    }
    pub fn center_bottom(&self) -> Pos2 {
        pos2(self.center().x, self.max().y)
    }
    pub fn right_bottom(&self) -> Pos2 {
        pos2(self.max().x, self.max().y)
    }
}

// ----------------------------------------------------------------------------

pub fn lerp<T>(min: T, max: T, t: f32) -> T
where
    f32: std::ops::Mul<T, Output = T>,
    T: std::ops::Add<T, Output = T>,
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
    return 3.0 * t * t - 2.0 * t * t * t;
}

pub const TAU: f32 = 2.0 * std::f32::consts::PI;
