use std::ops::{RangeFrom, RangeFull, RangeInclusive, RangeToInclusive};

/// Includive range of floats, i.e. `min..=max`, but more ergonomic than [`RangeInclusive`].
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
    pub fn span(&self) -> f32 {
        self.max - self.min
    }

    #[inline]
    pub fn contains(&self, x: f32) -> bool {
        self.min <= x && x <= self.max
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
