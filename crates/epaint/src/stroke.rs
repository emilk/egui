#![allow(clippy::derived_hash_with_manual_eq)] // We need to impl Hash for f32, but we don't implement Eq, which is fine

use std::{fmt::Debug, sync::Arc};

use super::*;

/// Describes the width and color of a line.
///
/// The default stroke is the same as [`Stroke::NONE`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Stroke {
    pub width: f32,
    pub color: Color32,
}

impl Stroke {
    /// Same as [`Stroke::default`].
    pub const NONE: Self = Self {
        width: 0.0,
        color: Color32::TRANSPARENT,
    };

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
    fn from((width, color): (f32, Color)) -> Self {
        Self::new(width, color)
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

/// How paths will be colored.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ColorMode {
    /// The entire path is one solid color, this is the default.
    Solid(Color32),

    /// Provide a callback which takes in a UV coordinate and converts it to a color. The values passed to this will always be between zero and one.
    ///
    /// **This cannot be serialized**
    #[cfg_attr(feature = "serde", serde(skip))]
    UV(Arc<Box<dyn Fn(Pos2) -> Color32 + Send + Sync>>),
}

impl Default for ColorMode {
    fn default() -> Self {
        Self::Solid(Color32::TRANSPARENT)
    }
}

impl Debug for ColorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Solid(arg0) => f.debug_tuple("Solid").field(arg0).finish(),
            Self::UV(_arg0) => f.debug_tuple("UV").field(&"<closure>").finish(),
        }
    }
}

impl PartialEq for ColorMode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Solid(l0), Self::Solid(r0)) => l0 == r0,
            (Self::UV(_l0), Self::UV(_r0)) => false,
            _ => false,
        }
    }
}

/// Describes the width and color of paths. The color can either be solid or provided by a callback. For more information, see [`ColorMode`]
///
/// The default stroke is the same as [`Stroke::NONE`].
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PathStroke {
    pub width: f32,
    pub color: ColorMode,
}

impl PathStroke {
    /// Same as [`PathStroke::default`].
    pub const NONE: Self = Self {
        width: 0.0,
        color: ColorMode::Solid(Color32::TRANSPARENT),
    };

    #[inline]
    pub fn new(width: impl Into<f32>, color: impl Into<Color32>) -> Self {
        Self {
            width: width.into(),
            color: ColorMode::Solid(color.into()),
        }
    }

    #[inline]
    pub fn new_uv(
        width: impl Into<f32>,
        callback: impl Fn(Pos2) -> Color32 + Send + Sync + 'static,
    ) -> Self {
        Self {
            width: width.into(),
            color: ColorMode::UV(Arc::new(Box::new(callback))),
        }
    }

    /// True if width is zero or color is solid and transparent
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.color == ColorMode::Solid(Color32::TRANSPARENT)
    }
}

impl<Color> From<(f32, Color)> for PathStroke
where
    Color: Into<Color32>,
{
    #[inline(always)]
    fn from((width, color): (f32, Color)) -> Self {
        Self::new(width, color)
    }
}

impl From<Stroke> for PathStroke {
    fn from(value: Stroke) -> Self {
        Self {
            width: value.width,
            color: ColorMode::Solid(value.color),
        }
    }
}
