#![allow(clippy::derive_hash_xor_eq)] // We need to impl Hash for f32, but we don't implement Eq, which is fine

use super::*;

/// Describes the width and color of a line.
///
/// The default stroke is the same as [`Stroke::none`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Stroke {
    pub width: f32,
    pub color: Color32,
}

impl Stroke {
    /// Same as [`Stroke::default`].
    pub const NONE: Stroke = Stroke {
        width: 0.0,
        color: Color32::TRANSPARENT,
    };

    #[deprecated = "Use Stroke::NONE instead"]
    #[inline(always)]
    pub fn none() -> Self {
        Self::new(0.0, Color32::TRANSPARENT)
    }

    #[inline]
    pub fn new(width: impl Into<f32>, color: impl Into<Color32>) -> Self {
        Self {
            width: width.into(),
            color: color.into(),
        }
    }

    /// True if width is zero or color is transparent
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.color == Color32::TRANSPARENT
    }
}

impl<Color> From<(f32, Color)> for Stroke
where
    Color: Into<Color32>,
{
    #[inline(always)]
    fn from((width, color): (f32, Color)) -> Stroke {
        Stroke::new(width, color)
    }
}

impl std::hash::Hash for Stroke {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self { width, color } = *self;
        crate::f32_hash(state, width);
        color.hash(state);
    }
}
