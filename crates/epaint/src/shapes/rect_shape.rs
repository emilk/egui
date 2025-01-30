use std::sync::Arc;

use crate::*;

/// How to paint a rectangle.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RectShape {
    pub rect: Rect,

    /// How rounded the corners are. Use `Rounding::ZERO` for no rounding.
    pub rounding: Rounding,

    /// How to fill the rectangle.
    pub fill: Color32,

    /// The thickness and color of the outline.
    ///
    /// Whether or not the stroke is inside or outside the edge of [`Self::rect`],
    /// is controlled by [`Self::stroke_kind`].
    pub stroke: Stroke,

    /// Is the stroke on the inside, outside, or centered on the rectangle?
    ///
    /// If you want to perfectly tile rectangles, use [`StrokeKind::Inside`].
    pub stroke_kind: StrokeKind,

    /// Snap the rectangle to pixels?
    ///
    /// Rounding produces sharper rectangles.
    ///
    /// If `None`, [`crate::TessellationOptions::round_rects_to_pixels`] will be used.
    pub round_to_pixels: Option<bool>,

    /// If larger than zero, the edges of the rectangle
    /// (for both fill and stroke) will be blurred.
    ///
    /// This can be used to produce shadows and glow effects.
    ///
    /// The blur is currently implemented using a simple linear blur in sRGBA gamma space.
    pub blur_width: f32,

    /// Controls texturing, if any.
    ///
    /// Since most rectangles do not have a texture, this is optional and in an `Arc`,
    /// so that [`RectShape`] is kept small..
    pub brush: Option<Arc<Brush>>,
}

#[test]
fn rect_shape_size() {
    assert_eq!(
        std::mem::size_of::<RectShape>(), 48,
        "RectShape changed size! If it shrank - good! Update this test. If it grew - bad! Try to find a way to avoid it."
    );
    assert!(
        std::mem::size_of::<RectShape>() <= 64,
        "RectShape is getting way too big!"
    );
}

impl RectShape {
    /// See also [`Self::filled`] and [`Self::stroke`].
    #[inline]
    pub fn new(
        rect: Rect,
        rounding: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
        stroke: impl Into<Stroke>,
        stroke_kind: StrokeKind,
    ) -> Self {
        Self {
            rect,
            rounding: rounding.into(),
            fill: fill_color.into(),
            stroke: stroke.into(),
            stroke_kind,
            round_to_pixels: None,
            blur_width: 0.0,
            brush: Default::default(),
        }
    }

    #[inline]
    pub fn filled(
        rect: Rect,
        rounding: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
    ) -> Self {
        Self::new(
            rect,
            rounding,
            fill_color,
            Stroke::NONE,
            StrokeKind::Outside, // doesn't matter
        )
    }

    #[inline]
    pub fn stroke(
        rect: Rect,
        rounding: impl Into<Rounding>,
        stroke: impl Into<Stroke>,
        stroke_kind: StrokeKind,
    ) -> Self {
        let fill = Color32::TRANSPARENT;
        Self::new(rect, rounding, fill, stroke, stroke_kind)
    }

    /// Set if the stroke is on the inside, outside, or centered on the rectangle.
    #[inline]
    pub fn with_stroke_kind(mut self, stroke_kind: StrokeKind) -> Self {
        self.stroke_kind = stroke_kind;
        self
    }

    /// Snap the rectangle to pixels?
    ///
    /// Rounding produces sharper rectangles.
    ///
    /// If `None`, [`crate::TessellationOptions::round_rects_to_pixels`] will be used.
    #[inline]
    pub fn with_round_to_pixels(mut self, round_to_pixels: bool) -> Self {
        self.round_to_pixels = Some(round_to_pixels);
        self
    }

    /// If larger than zero, the edges of the rectangle
    /// (for both fill and stroke) will be blurred.
    ///
    /// This can be used to produce shadows and glow effects.
    ///
    /// The blur is currently implemented using a simple linear blur in `sRGBA` gamma space.
    #[inline]
    pub fn with_blur_width(mut self, blur_width: f32) -> Self {
        self.blur_width = blur_width;
        self
    }

    /// Set the texture to use when painting this rectangle, if any.
    #[inline]
    pub fn with_texture(mut self, fill_texture_id: TextureId, uv: Rect) -> Self {
        self.brush = Some(Arc::new(Brush {
            fill_texture_id,
            uv,
        }));
        self
    }

    /// The visual bounding rectangle (includes stroke width)
    #[inline]
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            let expand = match self.stroke_kind {
                StrokeKind::Inside => 0.0,
                StrokeKind::Middle => self.stroke.width / 2.0,
                StrokeKind::Outside => self.stroke.width,
            };
            self.rect.expand(expand + self.blur_width / 2.0)
        }
    }

    /// The texture to use when painting this rectangle, if any.
    ///
    /// If no texture is set, this will return [`TextureId::default`].
    pub fn fill_texture_id(&self) -> TextureId {
        self.brush
            .as_ref()
            .map_or_else(TextureId::default, |brush| brush.fill_texture_id)
    }
}

impl From<RectShape> for Shape {
    #[inline(always)]
    fn from(shape: RectShape) -> Self {
        Self::Rect(shape)
    }
}
