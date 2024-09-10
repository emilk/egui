#![allow(clippy::derived_hash_with_manual_eq)] // We need to impl Hash for f32, but we don't implement Eq, which is fine

use std::{fmt::Debug, sync::Arc};

use super::{emath, Color32, ColorMode, Pos2, Rect};

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
        emath::OrderedFloat(width).hash(state);
        color.hash(state);
    }
}

/// Describes how the stroke of a shape should be painted.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum StrokeKind {
    /// The stroke should be painted entirely outside of the shape
    Outside,

    /// The stroke should be painted entirely inside of the shape
    Inside,

    /// The stroke should be painted right on the edge of the shape, half inside and half outside.
    Middle,
}

impl Default for StrokeKind {
    fn default() -> Self {
        Self::Middle
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
    pub kind: StrokeKind,
}

impl PathStroke {
    /// Same as [`PathStroke::default`].
    pub const NONE: Self = Self {
        width: 0.0,
        color: ColorMode::TRANSPARENT,
        kind: StrokeKind::Middle,
    };

    #[inline]
    pub fn new(width: impl Into<f32>, color: impl Into<Color32>) -> Self {
        Self {
            width: width.into(),
            color: ColorMode::Solid(color.into()),
            kind: StrokeKind::default(),
        }
    }

    /// Create a new `PathStroke` with a UV function
    ///
    /// The bounding box passed to the callback will have a margin of [`TessellationOptions::feathering_size_in_pixels`](`crate::tessellator::TessellationOptions::feathering_size_in_pixels`)
    #[inline]
    pub fn new_uv(
        width: impl Into<f32>,
        callback: impl Fn(Rect, Pos2) -> Color32 + Send + Sync + 'static,
    ) -> Self {
        Self {
            width: width.into(),
            color: ColorMode::UV(Arc::new(callback)),
            kind: StrokeKind::default(),
        }
    }

    /// Set the stroke to be painted right on the edge of the shape, half inside and half outside.
    pub fn middle(self) -> Self {
        Self {
            kind: StrokeKind::Middle,
            ..self
        }
    }

    /// Set the stroke to be painted entirely outside of the shape
    pub fn outside(self) -> Self {
        Self {
            kind: StrokeKind::Outside,
            ..self
        }
    }

    /// Set the stroke to be painted entirely inside of the shape
    pub fn inside(self) -> Self {
        Self {
            kind: StrokeKind::Inside,
            ..self
        }
    }

    /// True if width is zero or color is solid and transparent
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.color == ColorMode::TRANSPARENT
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
            kind: StrokeKind::default(),
        }
    }
}
