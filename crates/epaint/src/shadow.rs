use super::*;

/// The color and fuzziness of a fuzzy shape.
/// Can be used for a rectangular shadow with a soft penumbra.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Shadow {
    /// The shadow extends this much outside the rect.
    /// The size of the fuzzy penumbra.
    pub extrusion: f32,

    /// Color of the opaque center of the shadow.
    pub color: Color32,

    /// Move the shadow by this much.
    ///
    /// For instance, a value of `[1.0, 2.0]` will move the shadow 1 point to the right and 2 points down,
    /// causing a drop-shadow effet.
    pub offset: Vec2,
}

impl Shadow {
    pub const NONE: Self = Self {
        extrusion: 0.0,
        color: Color32::TRANSPARENT,
        offset: Vec2::ZERO,
    };

    pub const fn new(extrusion: f32, color: Color32) -> Self {
        Self {
            extrusion,
            color,
            offset: Vec2::ZERO,
        }
    }

    /// Tooltips, menus, …, for dark mode.
    pub const fn small_dark() -> Self {
        Self {
            extrusion: 12.0,
            color: Color32::from_black_alpha(96),
            offset: Vec2::splat(4.0),
        }
    }

    /// Tooltips, menus, …, for light mode.
    pub const fn small_light() -> Self {
        Self {
            extrusion: 12.0,
            color: Color32::from_black_alpha(20),
            offset: Vec2::splat(4.0),
        }
    }

    /// Used for egui windows in dark mode.
    pub const fn big_dark() -> Self {
        Self {
            extrusion: 32.0,
            color: Color32::from_black_alpha(96),
            offset: Vec2::splat(0.0),
        }
    }

    /// Used for egui windows in light mode.
    pub const fn big_light() -> Self {
        Self {
            extrusion: 32.0,
            color: Color32::from_black_alpha(16),
            offset: Vec2::splat(0.0),
        }
    }

    pub fn tessellate(&self, rect: Rect, rounding: impl Into<Rounding>) -> Mesh {
        // tessellator.clip_rect = clip_rect; // TODO(emilk): culling

        let Self {
            extrusion,
            color,
            offset,
        } = *self;

        let rect = rect.translate(offset);

        let rounding: Rounding = rounding.into();
        let half_ext = 0.5 * extrusion;

        let ext_rounding = Rounding {
            nw: rounding.nw + half_ext,
            ne: rounding.ne + half_ext,
            sw: rounding.sw + half_ext,
            se: rounding.se + half_ext,
        };

        use crate::tessellator::*;
        let rect = RectShape::filled(rect.expand(half_ext), ext_rounding, color);
        let pixels_per_point = 1.0; // doesn't matter here
        let font_tex_size = [1; 2]; // unused size we are not tessellating text.
        let mut tessellator = Tessellator::new(
            pixels_per_point,
            TessellationOptions {
                feathering: true,
                feathering_size_in_pixels: extrusion * pixels_per_point,
                ..Default::default()
            },
            font_tex_size,
            vec![],
        );
        let mut mesh = Mesh::default();
        tessellator.tessellate_rect(&rect, &mut mesh);
        mesh
    }
}
