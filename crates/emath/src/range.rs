use std::ops::{RangeFrom, RangeFull, RangeInclusive, RangeToInclusive};

/// Inclusive range of floats, i.e. `min..=max`, but more ergonomic than [`RangeInclusive`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Rangef {
    pub min: f32,
    pub max: f32,
}

impl Rangef {
    /// Infinite range that contains everything, from -∞ to +∞, inclusive.
    pub const EVERYTHING: Self = Self {
        min: f32::NEG_INFINITY,
        max: f32::INFINITY,
    };

    /// The inverse of [`Self::EVERYTHING`]: stretches from positive infinity to negative infinity.
    /// Contains nothing.
    pub const NOTHING: Self = Self {
        min: f32::INFINITY,
        max: f32::NEG_INFINITY,
    };

    /// An invalid [`Rangef`] filled with [`f32::NAN`].
    pub const NAN: Self = Self {
        min: f32::NAN,
        max: f32::NAN,
    };

    #[inline]
    pub fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn point(min_and_max: f32) -> Self {
        Self {
            min: min_and_max,
            max: min_and_max,
        }
    }

    /// The length of the range, i.e. `max - min`.
    #[inline]
    pub fn span(self) -> f32 {
        self.max - self.min
    }

    /// The center of the range
    #[inline]
    pub fn center(self) -> f32 {
        0.5 * (self.min + self.max)
    }

    #[inline]
    #[must_use]
    pub fn contains(self, x: f32) -> bool {
        self.min <= x && x <= self.max
    }

    /// Equivalent to `x.clamp(min, max)`
    #[inline]
    #[must_use]
    pub fn clamp(self, x: f32) -> f32 {
        x.clamp(self.min, self.max)
    }

    /// Flip `min` and `max` if needed, so that `min <= max` after.
    #[inline]
    pub fn as_positive(self) -> Self {
        Self {
            min: self.min.min(self.max),
            max: self.min.max(self.max),
        }
    }

    /// Shrink by this much on each side, keeping the center
    #[inline]
    #[must_use]
    pub fn shrink(self, amnt: f32) -> Self {
        Self {
            min: self.min + amnt,
            max: self.max - amnt,
        }
    }

    /// Expand by this much on each side, keeping the center
    #[inline]
    #[must_use]
    pub fn expand(self, amnt: f32) -> Self {
        Self {
            min: self.min - amnt,
            max: self.max + amnt,
        }
    }

    /// Flip the min and the max
    #[inline]
    #[must_use]
    pub fn flip(self) -> Self {
        Self {
            min: self.max,
            max: self.min,
        }
    }

    /// The overlap of two ranges, i.e. the range that is contained by both.
    ///
    /// If the ranges do not overlap, returns a range with `span() < 0.0`.
    ///
    /// ```
    /// # use emath::Rangef;
    /// assert_eq!(Rangef::new(0.0, 10.0).intersection(Rangef::new(5.0, 15.0)), Rangef::new(5.0, 10.0));
    /// assert_eq!(Rangef::new(0.0, 10.0).intersection(Rangef::new(10.0, 20.0)), Rangef::new(10.0, 10.0));
    /// assert!(Rangef::new(0.0, 10.0).intersection(Rangef::new(20.0, 30.0)).span() < 0.0);
    /// ```
    #[inline]
    #[must_use]
    pub fn intersection(self, other: Self) -> Self {
        Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        }
    }

    /// Do the two ranges intersect?
    ///
    /// ```
    /// # use emath::Rangef;
    /// assert!(Rangef::new(0.0, 10.0).intersects(Rangef::new(5.0, 15.0)));
    /// assert!(Rangef::new(0.0, 10.0).intersects(Rangef::new(5.0, 6.0)));
    /// assert!(Rangef::new(0.0, 10.0).intersects(Rangef::new(10.0, 20.0)));
    /// assert!(!Rangef::new(0.0, 10.0).intersects(Rangef::new(20.0, 30.0)));
    /// ```
    #[inline]
    #[must_use]
    pub fn intersects(self, other: Self) -> bool {
        other.min <= self.max && self.min <= other.max
    }
}

impl From<Rangef> for RangeInclusive<f32> {
    #[inline]
    fn from(Rangef { min, max }: Rangef) -> Self {
        min..=max
    }
}

impl From<&Rangef> for RangeInclusive<f32> {
    #[inline]
    fn from(&Rangef { min, max }: &Rangef) -> Self {
        min..=max
    }
}

impl From<RangeInclusive<f32>> for Rangef {
    #[inline]
    fn from(range: RangeInclusive<f32>) -> Self {
        Self::new(*range.start(), *range.end())
    }
}

impl From<&RangeInclusive<f32>> for Rangef {
    #[inline]
    fn from(range: &RangeInclusive<f32>) -> Self {
        Self::new(*range.start(), *range.end())
    }
}

impl From<RangeFrom<f32>> for Rangef {
    #[inline]
    fn from(range: RangeFrom<f32>) -> Self {
        Self::new(range.start, f32::INFINITY)
    }
}

impl From<&RangeFrom<f32>> for Rangef {
    #[inline]
    fn from(range: &RangeFrom<f32>) -> Self {
        Self::new(range.start, f32::INFINITY)
    }
}

impl From<RangeFull> for Rangef {
    #[inline]
    fn from(_: RangeFull) -> Self {
        Self::new(f32::NEG_INFINITY, f32::INFINITY)
    }
}

impl From<&RangeFull> for Rangef {
    #[inline]
    fn from(_: &RangeFull) -> Self {
        Self::new(f32::NEG_INFINITY, f32::INFINITY)
    }
}

impl From<RangeToInclusive<f32>> for Rangef {
    #[inline]
    fn from(range: RangeToInclusive<f32>) -> Self {
        Self::new(f32::NEG_INFINITY, range.end)
    }
}

impl PartialEq<RangeInclusive<f32>> for Rangef {
    #[inline]
    fn eq(&self, other: &RangeInclusive<f32>) -> bool {
        self.min == *other.start() && self.max == *other.end()
    }
}

impl PartialEq<Rangef> for RangeInclusive<f32> {
    #[inline]
    fn eq(&self, other: &Rangef) -> bool {
        *self.start() == other.min && *self.end() == other.max
    }
}
