use crate::CornerRadius;

/// How rounded the corners of things should be, in `f32`.
///
/// This is used for calculations, but storage is usually done with the more compact [`CornerRadius`].
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CornerRadiusF32 {
    /// Radius of the rounding of the North-West (left top) corner.
    pub nw: f32,

    /// Radius of the rounding of the North-East (right top) corner.
    pub ne: f32,

    /// Radius of the rounding of the South-West (left bottom) corner.
    pub sw: f32,

    /// Radius of the rounding of the South-East (right bottom) corner.
    pub se: f32,

    /// The shape of the corners.
    ///
    /// If `None`, defaults to [`crate::CornerShape::Round`] for backward compatibility.
    #[cfg_attr(feature = "serde", serde(default))]
    pub shape: Option<crate::CornerShape>,
}

impl From<CornerRadius> for CornerRadiusF32 {
    #[inline]
    fn from(cr: CornerRadius) -> Self {
        Self {
            nw: cr.nw as f32,
            ne: cr.ne as f32,
            sw: cr.sw as f32,
            se: cr.se as f32,
            shape: cr.shape,
        }
    }
}

impl From<CornerRadiusF32> for CornerRadius {
    #[inline]
    fn from(cr: CornerRadiusF32) -> Self {
        Self {
            nw: cr.nw.round() as u8,
            ne: cr.ne.round() as u8,
            sw: cr.sw.round() as u8,
            se: cr.se.round() as u8,
            shape: cr.shape,
        }
    }
}

impl Default for CornerRadiusF32 {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<f32> for CornerRadiusF32 {
    #[inline]
    fn from(radius: f32) -> Self {
        Self {
            nw: radius,
            ne: radius,
            sw: radius,
            se: radius,
            shape: None,
        }
    }
}

impl CornerRadiusF32 {
    /// No rounding on any corner.
    pub const ZERO: Self = Self {
        nw: 0.0,
        ne: 0.0,
        sw: 0.0,
        se: 0.0,
        shape: None,
    };

    /// Same rounding on all four corners.
    #[inline]
    pub const fn same(radius: f32) -> Self {
        Self {
            nw: radius,
            ne: radius,
            sw: radius,
            se: radius,
            shape: None,
        }
    }

    /// Set the shape of the corners.
    #[inline]
    pub const fn with_shape(mut self, shape: crate::CornerShape) -> Self {
        self.shape = Some(shape);
        self
    }

    /// Get the actual shape to use, defaulting to Round if not specified.
    #[inline]
    pub fn shape(&self) -> crate::CornerShape {
        self.shape.unwrap_or(crate::CornerShape::Round)
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
            shape: self.shape,
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
            shape: self.shape,
        }
    }
}

impl std::ops::Add for CornerRadiusF32 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            nw: self.nw + rhs.nw,
            ne: self.ne + rhs.ne,
            sw: self.sw + rhs.sw,
            se: self.se + rhs.se,
            shape: self.shape,
        }
    }
}

impl std::ops::AddAssign for CornerRadiusF32 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = Self {
            nw: self.nw + rhs.nw,
            ne: self.ne + rhs.ne,
            sw: self.sw + rhs.sw,
            se: self.se + rhs.se,
            shape: self.shape,
        };
    }
}

impl std::ops::AddAssign<f32> for CornerRadiusF32 {
    #[inline]
    fn add_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw + rhs,
            ne: self.ne + rhs,
            sw: self.sw + rhs,
            se: self.se + rhs,
            shape: self.shape,
        };
    }
}

impl std::ops::Sub for CornerRadiusF32 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            nw: self.nw - rhs.nw,
            ne: self.ne - rhs.ne,
            sw: self.sw - rhs.sw,
            se: self.se - rhs.se,
            shape: self.shape,
        }
    }
}

impl std::ops::SubAssign for CornerRadiusF32 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = Self {
            nw: self.nw - rhs.nw,
            ne: self.ne - rhs.ne,
            sw: self.sw - rhs.sw,
            se: self.se - rhs.se,
            shape: self.shape,
        };
    }
}

impl std::ops::SubAssign<f32> for CornerRadiusF32 {
    #[inline]
    fn sub_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw - rhs,
            ne: self.ne - rhs,
            sw: self.sw - rhs,
            se: self.se - rhs,
            shape: self.shape,
        };
    }
}

impl std::ops::Div<f32> for CornerRadiusF32 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self {
            nw: self.nw / rhs,
            ne: self.ne / rhs,
            sw: self.sw / rhs,
            se: self.se / rhs,
            shape: self.shape,
        }
    }
}

impl std::ops::DivAssign<f32> for CornerRadiusF32 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw / rhs,
            ne: self.ne / rhs,
            sw: self.sw / rhs,
            se: self.se / rhs,
            shape: self.shape,
        };
    }
}

impl std::ops::Mul<f32> for CornerRadiusF32 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self {
            nw: self.nw * rhs,
            ne: self.ne * rhs,
            sw: self.sw * rhs,
            se: self.se * rhs,
            shape: self.shape,
        }
    }
}

impl std::ops::MulAssign<f32> for CornerRadiusF32 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw * rhs,
            ne: self.ne * rhs,
            sw: self.sw * rhs,
            se: self.se * rhs,
            shape: self.shape,
        };
    }
}
