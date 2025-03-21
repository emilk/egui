/// How rounded the corners of things should be.
///
/// This specific the _corner radius_ of the underlying geometric shape (e.g. rectangle).
/// If there is a stroke, then the stroke will have an inner and outer corner radius
/// which will depends on its width and [`crate::StrokeKind`].
///
/// The rounding uses `u8` to save space,
/// so the amount of rounding is limited to integers in the range `[0, 255]`.
///
/// For calculations, you may want to use [`crate::CornerRadiusF32`] instead, which uses `f32`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CornerRadius {
    /// Radius of the rounding of the North-West (left top) corner.
    pub nw: u8,

    /// Radius of the rounding of the North-East (right top) corner.
    pub ne: u8,

    /// Radius of the rounding of the South-West (left bottom) corner.
    pub sw: u8,

    /// Radius of the rounding of the South-East (right bottom) corner.
    pub se: u8,
}

impl Default for CornerRadius {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<u8> for CornerRadius {
    #[inline]
    fn from(radius: u8) -> Self {
        Self::same(radius)
    }
}

impl From<f32> for CornerRadius {
    #[inline]
    fn from(radius: f32) -> Self {
        Self::same(radius.round() as u8)
    }
}

impl CornerRadius {
    /// No rounding on any corner.
    pub const ZERO: Self = Self {
        nw: 0,
        ne: 0,
        sw: 0,
        se: 0,
    };

    /// Same rounding on all four corners.
    #[inline]
    pub const fn same(radius: u8) -> Self {
        Self {
            nw: radius,
            ne: radius,
            sw: radius,
            se: radius,
        }
    }

    /// Do all corners have the same rounding?
    #[inline]
    pub fn is_same(self) -> bool {
        self.nw == self.ne && self.nw == self.sw && self.nw == self.se
    }

    /// Make sure each corner has a rounding of at least this.
    #[inline]
    pub fn at_least(self, min: u8) -> Self {
        Self {
            nw: self.nw.max(min),
            ne: self.ne.max(min),
            sw: self.sw.max(min),
            se: self.se.max(min),
        }
    }

    /// Make sure each corner has a rounding of at most this.
    #[inline]
    pub fn at_most(self, max: u8) -> Self {
        Self {
            nw: self.nw.min(max),
            ne: self.ne.min(max),
            sw: self.sw.min(max),
            se: self.se.min(max),
        }
    }

    /// Average rounding of the corners.
    pub fn average(&self) -> f32 {
        (self.nw as f32 + self.ne as f32 + self.sw as f32 + self.se as f32) / 4.0
    }
}

impl std::ops::Add for CornerRadius {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            nw: self.nw.saturating_add(rhs.nw),
            ne: self.ne.saturating_add(rhs.ne),
            sw: self.sw.saturating_add(rhs.sw),
            se: self.se.saturating_add(rhs.se),
        }
    }
}

impl std::ops::Add<u8> for CornerRadius {
    type Output = Self;
    #[inline]
    fn add(self, rhs: u8) -> Self {
        Self {
            nw: self.nw.saturating_add(rhs),
            ne: self.ne.saturating_add(rhs),
            sw: self.sw.saturating_add(rhs),
            se: self.se.saturating_add(rhs),
        }
    }
}

impl std::ops::AddAssign for CornerRadius {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = Self {
            nw: self.nw.saturating_add(rhs.nw),
            ne: self.ne.saturating_add(rhs.ne),
            sw: self.sw.saturating_add(rhs.sw),
            se: self.se.saturating_add(rhs.se),
        };
    }
}

impl std::ops::AddAssign<u8> for CornerRadius {
    #[inline]
    fn add_assign(&mut self, rhs: u8) {
        *self = Self {
            nw: self.nw.saturating_add(rhs),
            ne: self.ne.saturating_add(rhs),
            sw: self.sw.saturating_add(rhs),
            se: self.se.saturating_add(rhs),
        };
    }
}

impl std::ops::Sub for CornerRadius {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            nw: self.nw.saturating_sub(rhs.nw),
            ne: self.ne.saturating_sub(rhs.ne),
            sw: self.sw.saturating_sub(rhs.sw),
            se: self.se.saturating_sub(rhs.se),
        }
    }
}

impl std::ops::Sub<u8> for CornerRadius {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: u8) -> Self {
        Self {
            nw: self.nw.saturating_sub(rhs),
            ne: self.ne.saturating_sub(rhs),
            sw: self.sw.saturating_sub(rhs),
            se: self.se.saturating_sub(rhs),
        }
    }
}

impl std::ops::SubAssign for CornerRadius {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = Self {
            nw: self.nw.saturating_sub(rhs.nw),
            ne: self.ne.saturating_sub(rhs.ne),
            sw: self.sw.saturating_sub(rhs.sw),
            se: self.se.saturating_sub(rhs.se),
        };
    }
}

impl std::ops::SubAssign<u8> for CornerRadius {
    #[inline]
    fn sub_assign(&mut self, rhs: u8) {
        *self = Self {
            nw: self.nw.saturating_sub(rhs),
            ne: self.ne.saturating_sub(rhs),
            sw: self.sw.saturating_sub(rhs),
            se: self.se.saturating_sub(rhs),
        };
    }
}

impl std::ops::Div<f32> for CornerRadius {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self {
            nw: (self.nw as f32 / rhs) as u8,
            ne: (self.ne as f32 / rhs) as u8,
            sw: (self.sw as f32 / rhs) as u8,
            se: (self.se as f32 / rhs) as u8,
        }
    }
}

impl std::ops::DivAssign<f32> for CornerRadius {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: (self.nw as f32 / rhs) as u8,
            ne: (self.ne as f32 / rhs) as u8,
            sw: (self.sw as f32 / rhs) as u8,
            se: (self.se as f32 / rhs) as u8,
        };
    }
}

impl std::ops::Mul<f32> for CornerRadius {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self {
            nw: (self.nw as f32 * rhs) as u8,
            ne: (self.ne as f32 * rhs) as u8,
            sw: (self.sw as f32 * rhs) as u8,
            se: (self.se as f32 * rhs) as u8,
        }
    }
}

impl std::ops::MulAssign<f32> for CornerRadius {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: (self.nw as f32 * rhs) as u8,
            ne: (self.ne as f32 * rhs) as u8,
            sw: (self.sw as f32 * rhs) as u8,
            se: (self.se as f32 * rhs) as u8,
        };
    }
}
