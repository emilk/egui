use crate::Rounding;

/// How rounded the corners of things should be, in `f32`.
///
/// This is used for calculations, but storage is usually done with the more compact [`Rounding`].
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Roundingf {
    /// Radius of the rounding of the North-West (left top) corner.
    pub nw: f32,

    /// Radius of the rounding of the North-East (right top) corner.
    pub ne: f32,

    /// Radius of the rounding of the South-West (left bottom) corner.
    pub sw: f32,

    /// Radius of the rounding of the South-East (right bottom) corner.
    pub se: f32,
}

impl From<Rounding> for Roundingf {
    #[inline]
    fn from(rounding: Rounding) -> Self {
        Self {
            nw: rounding.nw as f32,
            ne: rounding.ne as f32,
            sw: rounding.sw as f32,
            se: rounding.se as f32,
        }
    }
}

impl From<Roundingf> for Rounding {
    #[inline]
    fn from(rounding: Roundingf) -> Self {
        Self {
            nw: rounding.nw.round() as u8,
            ne: rounding.ne.round() as u8,
            sw: rounding.sw.round() as u8,
            se: rounding.se.round() as u8,
        }
    }
}

impl Default for Roundingf {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<f32> for Roundingf {
    #[inline]
    fn from(radius: f32) -> Self {
        Self {
            nw: radius,
            ne: radius,
            sw: radius,
            se: radius,
        }
    }
}

impl Roundingf {
    /// No rounding on any corner.
    pub const ZERO: Self = Self {
        nw: 0.0,
        ne: 0.0,
        sw: 0.0,
        se: 0.0,
    };

    /// Same rounding on all four corners.
    #[inline]
    pub const fn same(radius: f32) -> Self {
        Self {
            nw: radius,
            ne: radius,
            sw: radius,
            se: radius,
        }
    }

    /// Do all corners have the same rounding?
    #[inline]
    pub fn is_same(&self) -> bool {
        self.nw == self.ne && self.nw == self.sw && self.nw == self.se
    }

    /// Make sure each corner has a rounding of at least this.
    #[inline]
    pub fn at_least(&self, min: f32) -> Self {
        Self {
            nw: self.nw.max(min),
            ne: self.ne.max(min),
            sw: self.sw.max(min),
            se: self.se.max(min),
        }
    }

    /// Make sure each corner has a rounding of at most this.
    #[inline]
    pub fn at_most(&self, max: f32) -> Self {
        Self {
            nw: self.nw.min(max),
            ne: self.ne.min(max),
            sw: self.sw.min(max),
            se: self.se.min(max),
        }
    }
}

impl std::ops::Add for Roundingf {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            nw: self.nw + rhs.nw,
            ne: self.ne + rhs.ne,
            sw: self.sw + rhs.sw,
            se: self.se + rhs.se,
        }
    }
}

impl std::ops::AddAssign for Roundingf {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = Self {
            nw: self.nw + rhs.nw,
            ne: self.ne + rhs.ne,
            sw: self.sw + rhs.sw,
            se: self.se + rhs.se,
        };
    }
}

impl std::ops::AddAssign<f32> for Roundingf {
    #[inline]
    fn add_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw + rhs,
            ne: self.ne + rhs,
            sw: self.sw + rhs,
            se: self.se + rhs,
        };
    }
}

impl std::ops::Sub for Roundingf {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            nw: self.nw - rhs.nw,
            ne: self.ne - rhs.ne,
            sw: self.sw - rhs.sw,
            se: self.se - rhs.se,
        }
    }
}

impl std::ops::SubAssign for Roundingf {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = Self {
            nw: self.nw - rhs.nw,
            ne: self.ne - rhs.ne,
            sw: self.sw - rhs.sw,
            se: self.se - rhs.se,
        };
    }
}

impl std::ops::SubAssign<f32> for Roundingf {
    #[inline]
    fn sub_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw - rhs,
            ne: self.ne - rhs,
            sw: self.sw - rhs,
            se: self.se - rhs,
        };
    }
}

impl std::ops::Div<f32> for Roundingf {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self {
            nw: self.nw / rhs,
            ne: self.ne / rhs,
            sw: self.sw / rhs,
            se: self.se / rhs,
        }
    }
}

impl std::ops::DivAssign<f32> for Roundingf {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw / rhs,
            ne: self.ne / rhs,
            sw: self.sw / rhs,
            se: self.se / rhs,
        };
    }
}

impl std::ops::Mul<f32> for Roundingf {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self {
            nw: self.nw * rhs,
            ne: self.ne * rhs,
            sw: self.sw * rhs,
            se: self.se * rhs,
        }
    }
}

impl std::ops::MulAssign<f32> for Roundingf {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw * rhs,
            ne: self.ne * rhs,
            sw: self.sw * rhs,
            se: self.se * rhs,
        };
    }
}
