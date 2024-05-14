use super::*;

/// The color and fuzziness of a fuzzy shape.
///
/// Can be used for a rectangular shadow with a soft penumbra.
///
/// Very similar to a box-shadow in CSS.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Shadow {
    /// Move the shadow by this much.
    ///
    /// For instance, a value of `[1.0, 2.0]` will move the shadow 1 point to the right and 2 points down,
    /// causing a drop-shadow effet.
    pub offset: Vec2,

    /// The width of the blur, i.e. the width of the fuzzy penumbra.
    ///
    /// A value of 0.0 means a sharp shadow.
    pub blur: f32,

    /// Expand the shadow in all directions by this much.
    pub spread: f32,

    /// Color of the opaque center of the shadow.
    pub color: Color32,
}

impl Shadow {
    /// No shadow at all.
    pub const NONE: Self = Self {
        offset: Vec2::ZERO,
        blur: 0.0,
        spread: 0.0,
        color: Color32::TRANSPARENT,
    };

    /// The argument is the rectangle of the shadow caster.
    pub fn as_shape(&self, rect: Rect, rounding: impl Into<Rounding>) -> RectShape {
        // tessellator.clip_rect = clip_rect; // TODO(emilk): culling

        let Self {
            offset,
            blur,
            spread,
            color,
        } = *self;

        let rect = rect.translate(offset).expand(spread);
        let rounding = rounding.into() + Rounding::same(spread.abs());

        RectShape::filled(rect, rounding, color).with_blur_width(blur)
    }

    /// How much larger than the parent rect are we in each direction?
    pub fn margin(&self) -> Margin {
        let Self {
            offset,
            blur,
            spread,
            color: _,
        } = *self;
        Margin {
            left: spread + 0.5 * blur - offset.x,
            right: spread + 0.5 * blur + offset.x,
            top: spread + 0.5 * blur - offset.y,
            bottom: spread + 0.5 * blur + offset.y,
        }
    }
}
