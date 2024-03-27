use emath::NumExt as _;

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

    pub fn tessellate(&self, rect: Rect, rounding: impl Into<Rounding>) -> Mesh {
        // tessellator.clip_rect = clip_rect; // TODO(emilk): culling

        use crate::tessellator::*;

        let Self {
            offset,
            blur,
            spread,
            color,
        } = *self;

        let rect = rect.translate(offset).expand(spread);

        // We simulate a blurry shadow by tessellating a solid rectangle using a very large feathering.
        // Feathering is usually used to make the edges of a shape softer for anti-aliasing.
        // The tessellator can't handle blurring/feathering larger than the smallest side of the rect.
        // Thats because the tessellator approximate very thin rectangles as line segments,
        // and these line segments don't have rounded corners.
        // When the feathering is small (the size of a pixel), this is usually fine,
        // but here we have a huge feathering to simulate blur,
        // so we need to avoid this optimization in the tessellator,
        // which is also why we add this rather big epsilon:
        let eps = 0.1;
        let blur = blur.at_most(rect.size().min_elem() - eps).at_least(0.0);

        // TODO(emilk): if blur <= 0, return a simple `Shape::Rect` instead of using the tessellator

        let rounding_expansion = spread.abs() + 0.5 * blur;
        let rounding = rounding.into() + Rounding::same(rounding_expansion);

        let rect = RectShape::filled(rect, rounding, color);
        let pixels_per_point = 1.0; // doesn't matter here
        let font_tex_size = [1; 2]; // unused since we are not tessellating text.
        let mut tessellator = Tessellator::new(
            pixels_per_point,
            TessellationOptions {
                feathering: true,
                feathering_size_in_pixels: blur * pixels_per_point,
                ..Default::default()
            },
            font_tex_size,
            vec![],
        );
        let mut mesh = Mesh::default();
        tessellator.tessellate_rect(&rect, &mut mesh);
        mesh
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
