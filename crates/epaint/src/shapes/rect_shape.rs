use crate::*;

/// How to paint a rectangle.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RectShape {
    pub rect: Rect,

    /// How rounded the corners are. Use `Rounding::ZERO` for no rounding.
    pub rounding: Rounding,

    /// How to fill the rectangle.
    pub fill: Color32,

    /// The thickness and color of the outline.
    ///
    /// The stroke extends _outside_ the edge of [`Self::rect`],
    /// i.e. using [`crate::StrokeKind::Outside`].
    ///
    /// This means the [`Self::visual_bounding_rect`] is `rect.size() + 2.0 * stroke.width`.
    pub stroke: Stroke,

    /// If larger than zero, the edges of the rectangle
    /// (for both fill and stroke) will be blurred.
    ///
    /// This can be used to produce shadows and glow effects.
    ///
    /// The blur is currently implemented using a simple linear blur in sRGBA gamma space.
    pub blur_width: f32,

    /// If the rect should be filled with a texture, which one?
    ///
    /// The texture is multiplied with [`Self::fill`].
    pub fill_texture_id: TextureId,

    /// What UV coordinates to use for the texture?
    ///
    /// To display a texture, set [`Self::fill_texture_id`],
    /// and set this to `Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))`.
    ///
    /// Use [`Rect::ZERO`] to turn off texturing.
    pub uv: Rect,
}

#[test]
fn rect_shape_size() {
    assert_eq!(
        std::mem::size_of::<RectShape>(), 72,
        "RectShape changed size! If it shrank - good! Update this test. If it grew - bad! Try to find a way to avoid it."
    );
}

impl RectShape {
    /// The stroke extends _outside_ the [`Rect`].
    #[inline]
    pub fn new(
        rect: Rect,
        rounding: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
        stroke: impl Into<Stroke>,
    ) -> Self {
        Self {
            rect,
            rounding: rounding.into(),
            fill: fill_color.into(),
            stroke: stroke.into(),
            blur_width: 0.0,
            fill_texture_id: Default::default(),
            uv: Rect::ZERO,
        }
    }

    #[inline]
    pub fn filled(
        rect: Rect,
        rounding: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
    ) -> Self {
        Self {
            rect,
            rounding: rounding.into(),
            fill: fill_color.into(),
            stroke: Default::default(),
            blur_width: 0.0,
            fill_texture_id: Default::default(),
            uv: Rect::ZERO,
        }
    }

    /// The stroke extends _outside_ the [`Rect`].
    #[inline]
    pub fn stroke(rect: Rect, rounding: impl Into<Rounding>, stroke: impl Into<Stroke>) -> Self {
        Self {
            rect,
            rounding: rounding.into(),
            fill: Default::default(),
            stroke: stroke.into(),
            blur_width: 0.0,
            fill_texture_id: Default::default(),
            uv: Rect::ZERO,
        }
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

    /// The visual bounding rectangle (includes stroke width)
    #[inline]
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            let Stroke { width, .. } = self.stroke; // Make sure we remember to update this if we change `stroke` to `PathStroke`
            self.rect.expand(width + self.blur_width / 2.0)
        }
    }
}

impl From<RectShape> for Shape {
    #[inline(always)]
    fn from(shape: RectShape) -> Self {
        Self::Rect(shape)
    }
}
