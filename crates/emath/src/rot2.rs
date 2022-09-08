use super::Vec2;

// {s,c} represents the rotation matrix:
//
// | c -s |
// | s  c |
//
// `vec2(c,s)` represents where the X axis will end up after rotation.
//
/// Represents a rotation in the 2D plane.
//
/// A rotation of 𝞃/4 = 90° rotates the X axis to the Y axis.
//
/// Normally a [`Rot2`] is normalized (unit-length).
/// If not, it will also scale vectors.
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Rot2 {
    /// angle.sin()
    s: f32,

    /// angle.cos()
    c: f32,
}

/// Identity rotation
impl Default for Rot2 {
    /// Identity rotation
    fn default() -> Self {
        Self { s: 0.0, c: 1.0 }
    }
}

impl Rot2 {
    /// The identity rotation: nothing rotates
    pub const IDENTITY: Self = Self { s: 0.0, c: 1.0 };

    /// Angle is clockwise in radians.
    /// A 𝞃/4 = 90° rotation means rotating the X axis to the Y axis.
    pub fn from_angle(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self { s, c }
    }

    pub fn angle(self) -> f32 {
        self.s.atan2(self.c)
    }

    /// The factor by which vectors will be scaled.
    pub fn length(self) -> f32 {
        self.c.hypot(self.s)
    }

    pub fn length_squared(self) -> f32 {
        self.c.powi(2) + self.s.powi(2)
    }

    pub fn is_finite(self) -> bool {
        self.c.is_finite() && self.s.is_finite()
    }

    #[must_use]
    pub fn inverse(self) -> Rot2 {
        Self {
            s: -self.s,
            c: self.c,
        } / self.length_squared()
    }

    #[must_use]
    pub fn normalized(self) -> Self {
        let l = self.length();
        let ret = Self {
            c: self.c / l,
            s: self.s / l,
        };
        crate::emath_assert!(ret.is_finite());
        ret
    }
}

impl std::fmt::Debug for Rot2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rot2 {{ angle: {:.1}°, length: {} }}",
            self.angle().to_degrees(),
            self.length()
        )
    }
}

impl std::ops::Mul<Rot2> for Rot2 {
    type Output = Rot2;

    fn mul(self, r: Rot2) -> Rot2 {
        /*
        |lc -ls| * |rc -rs|
        |ls  lc|   |rs  rc|
        */
        Rot2 {
            c: self.c * r.c - self.s * r.s,
            s: self.s * r.c + self.c * r.s,
        }
    }
}

/// Rotates (and maybe scales) the vector.
impl std::ops::Mul<Vec2> for Rot2 {
    type Output = Vec2;

    fn mul(self, v: Vec2) -> Vec2 {
        Vec2 {
            x: self.c * v.x - self.s * v.y,
            y: self.s * v.x + self.c * v.y,
        }
    }
}

/// Scales the rotor.
impl std::ops::Mul<Rot2> for f32 {
    type Output = Rot2;

    fn mul(self, r: Rot2) -> Rot2 {
        Rot2 {
            c: self * r.c,
            s: self * r.s,
        }
    }
}

/// Scales the rotor.
impl std::ops::Mul<f32> for Rot2 {
    type Output = Rot2;

    fn mul(self, r: f32) -> Rot2 {
        Rot2 {
            c: self.c * r,
            s: self.s * r,
        }
    }
}

/// Scales the rotor.
impl std::ops::Div<f32> for Rot2 {
    type Output = Rot2;

    fn div(self, r: f32) -> Rot2 {
        Rot2 {
            c: self.c / r,
            s: self.s / r,
        }
    }
}

#[cfg(test)]
mod test {
    use super::Rot2;
    use crate::vec2;

    #[test]
    fn test_rotation2() {
        {
            let angle = std::f32::consts::TAU / 6.0;
            let rot = Rot2::from_angle(angle);
            assert!((rot.angle() - angle).abs() < 1e-5);
            assert!((rot * rot.inverse()).angle().abs() < 1e-5);
            assert!((rot.inverse() * rot).angle().abs() < 1e-5);
        }

        {
            let angle = std::f32::consts::TAU / 4.0;
            let rot = Rot2::from_angle(angle);
            assert!(((rot * vec2(1.0, 0.0)) - vec2(0.0, 1.0)).length() < 1e-5);
        }

        {
            // Test rotation and scaling
            let angle = std::f32::consts::TAU / 4.0;
            let rot = 3.0 * Rot2::from_angle(angle);
            let rotated = rot * vec2(1.0, 0.0);
            let expected = vec2(0.0, 3.0);
            assert!(
                (rotated - expected).length() < 1e-5,
                "Expected {:?} to equal {:?}. rot: {:?}",
                rotated,
                expected,
                rot,
            );

            let undone = rot.inverse() * rot;
            assert!(undone.angle().abs() < 1e-5);
            assert!((undone.length() - 1.0).abs() < 1e-5,);
        }
    }
}
