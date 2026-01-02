use std::{fmt::Debug, sync::Arc};

use emath::GuiRounding as _;

use super::{Color32, ColorMode, Pos2, Rect, emath};

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

    /// For vertical or horizontal lines:
    /// round the stroke center to produce a sharp, pixel-aligned line.
    pub fn round_center_to_pixel(&self, pixels_per_point: f32, coord: &mut f32) {
        // If the stroke is an odd number of pixels wide,
        // we want to round the center of it to the center of a pixel.
        //
        // If however it is an even number of pixels wide,
        // we want to round the center to be between two pixels.
        //
        // We also want to treat strokes that are _almost_ odd as it it was odd,
        // to make it symmetric. Same for strokes that are _almost_ even.
        //
        // For strokes less than a pixel wide we also round to the center,
        // because it will rendered as a single row of pixels by the tessellator.

        let pixel_size = 1.0 / pixels_per_point;

        if self.width <= pixel_size || is_nearest_integer_odd(pixels_per_point * self.width) {
            *coord = coord.round_to_pixel_center(pixels_per_point);
        } else {
            *coord = coord.round_to_pixels(pixels_per_point);
        }
    }

    pub(crate) fn round_rect_to_pixel(&self, pixels_per_point: f32, rect: &mut Rect) {
        // We put odd-width strokes in the center of pixels.
        // To understand why, see `fn round_center_to_pixel`.

        let pixel_size = 1.0 / pixels_per_point;

        let width = self.width;
        if width <= 0.0 {
            *rect = rect.round_to_pixels(pixels_per_point);
        } else if width <= pixel_size || is_nearest_integer_odd(pixels_per_point * width) {
            *rect = rect.round_to_pixel_center(pixels_per_point);
        } else {
            *rect = rect.round_to_pixels(pixels_per_point);
        }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum StrokeKind {
    /// The stroke should be painted entirely inside of the shape
    Inside,

    /// The stroke should be painted right on the edge of the shape, half inside and half outside.
    Middle,

    /// The stroke should be painted entirely outside of the shape
    Outside,
}

/// Describes the width and color of paths. The color can either be solid or provided by a callback. For more information, see [`ColorMode`]
///
/// The default stroke is the same as [`Stroke::NONE`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PathStroke {
    pub width: f32,
    pub color: ColorMode,
    pub kind: StrokeKind,
}

impl Default for PathStroke {
    #[inline]
    fn default() -> Self {
        Self::NONE
    }
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
            kind: StrokeKind::Middle,
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
            kind: StrokeKind::Middle,
        }
    }

    #[inline]
    pub fn with_kind(self, kind: StrokeKind) -> Self {
        Self { kind, ..self }
    }

    /// Set the stroke to be painted right on the edge of the shape, half inside and half outside.
    #[inline]
    pub fn middle(self) -> Self {
        Self {
            kind: StrokeKind::Middle,
            ..self
        }
    }

    /// Set the stroke to be painted entirely outside of the shape
    #[inline]
    pub fn outside(self) -> Self {
        Self {
            kind: StrokeKind::Outside,
            ..self
        }
    }

    /// Set the stroke to be painted entirely inside of the shape
    #[inline]
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
        if value.is_empty() {
            // Important, since we use the stroke color when doing feathering of the fill!
            Self::NONE
        } else {
            Self {
                width: value.width,
                color: ColorMode::Solid(value.color),
                kind: StrokeKind::Middle,
            }
        }
    }
}

/// Returns true if the nearest integer is odd.
fn is_nearest_integer_odd(x: f32) -> bool {
    (x * 0.5 + 0.25).fract() > 0.5
}

#[test]
fn test_is_nearest_integer_odd() {
    assert!(is_nearest_integer_odd(0.6));
    assert!(is_nearest_integer_odd(1.0));
    assert!(is_nearest_integer_odd(1.4));
    assert!(!is_nearest_integer_odd(1.6));
    assert!(!is_nearest_integer_odd(2.0));
    assert!(!is_nearest_integer_odd(2.4));
    assert!(is_nearest_integer_odd(2.6));
    assert!(is_nearest_integer_odd(3.0));
    assert!(is_nearest_integer_odd(3.4));
}
