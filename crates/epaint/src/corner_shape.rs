use std::hash::{Hash, Hasher};

/// How to paint the corners of a rectangle.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum CornerShape {
    /// Standard circular corner. The `round` keyword is equivalent to `superellipse(1.0)`.
    Round,

    /// A straight, diagonal corner. The `bevel` keyword is equivalent to `superellipse(0.0)`.
    Bevel,

    /// A 90-degree concave square corner. The `notch` keyword is equivalent to `superellipse(-infinity)`.
    Notch,

    /// A concave ordinary ellipse. The `scoop` keyword is equivalent to `superellipse(-1.0)`.
    Scoop,

    /// A 90-degree convex square corner. The `square` keyword is equivalent to `superellipse(infinity)`.
    Square,

    /// A "squircle", which is a convex curve in between `round` and `square`. The `squircle` keyword is equivalent to `superellipse(2.0)`.
    Squircle,

    /// A superellipse with a given exponent for each corner.
    Superellipse { ne: f32, nw: f32, sw: f32, se: f32 },
}

impl Default for CornerShape {
    #[inline]
    fn default() -> Self {
        Self::Round
    }
}

/// Exponents for each corner of a rectangle.
#[derive(Clone, Copy, Debug)]
pub struct CornerExponents {
    pub ne: f32,
    pub nw: f32,
    pub sw: f32,
    pub se: f32,
}

impl CornerExponents {
    #[inline]
    fn bits(self) -> [u32; 4] {
        [
            self.ne.to_bits(),
            self.nw.to_bits(),
            self.sw.to_bits(),
            self.se.to_bits(),
        ]
    }
}

impl PartialEq for CornerExponents {
    fn eq(&self, other: &Self) -> bool {
        self.bits() == other.bits()
    }
}

impl Eq for CornerExponents {}

impl Hash for CornerExponents {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bits().hash(state);
    }
}

impl CornerShape {
    /// Creates a `Superellipse` with the same exponent for all corners.
    pub fn superellipse(exponent: f32) -> Self {
        Self::Superellipse {
            ne: exponent,
            nw: exponent,
            sw: exponent,
            se: exponent,
        }
    }

    /// Returns the exponents for each corner.
    pub fn exponents(&self) -> CornerExponents {
        match *self {
            Self::Round => CornerExponents {
                ne: 1.0,
                nw: 1.0,
                sw: 1.0,
                se: 1.0,
            },
            Self::Bevel => CornerExponents {
                ne: 0.0,
                nw: 0.0,
                sw: 0.0,
                se: 0.0,
            },
            Self::Notch => CornerExponents {
                ne: f32::NEG_INFINITY,
                nw: f32::NEG_INFINITY,
                sw: f32::NEG_INFINITY,
                se: f32::NEG_INFINITY,
            },
            Self::Scoop => CornerExponents {
                ne: -1.0,
                nw: -1.0,
                sw: -1.0,
                se: -1.0,
            },
            Self::Square => CornerExponents {
                ne: f32::INFINITY,
                nw: f32::INFINITY,
                sw: f32::INFINITY,
                se: f32::INFINITY,
            },
            Self::Squircle => CornerExponents {
                ne: 2.0,
                nw: 2.0,
                sw: 2.0,
                se: 2.0,
            },
            Self::Superellipse { ne, nw, sw, se } => CornerExponents { ne, nw, sw, se },
        }
    }
}

impl PartialEq for CornerShape {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Round, Self::Round)
            | (Self::Bevel, Self::Bevel)
            | (Self::Notch, Self::Notch)
            | (Self::Scoop, Self::Scoop)
            | (Self::Square, Self::Square)
            | (Self::Squircle, Self::Squircle) => true,

            _ => self.exponents() == other.exponents(),
        }
    }
}

impl Eq for CornerShape {}

impl Hash for CornerShape {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.exponents().hash(state);
    }
}
