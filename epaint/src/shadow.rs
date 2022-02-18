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
}

impl Shadow {
    /// Tooltips, menus, …
    pub fn small_dark() -> Self {
        Self {
            extrusion: 16.0,
            color: Color32::from_black_alpha(96),
        }
    }

    /// Tooltips, menus, …
    pub fn small_light() -> Self {
        Self {
            extrusion: 16.0,
            color: Color32::from_black_alpha(32),
        }
    }

    /// Subtle and nice on dark backgrounds
    pub fn big_dark() -> Self {
        Self {
            extrusion: 32.0,
            color: Color32::from_black_alpha(96),
        }
    }

    /// Subtle and nice on white backgrounds
    pub fn big_light() -> Self {
        Self {
            extrusion: 32.0,
            color: Color32::from_black_alpha(40),
        }
    }

    pub fn tessellate(&self, rect: emath::Rect, rounding: impl Into<Rounding>) -> Mesh {
        // tessellator.clip_rect = clip_rect; // TODO: culling

        let Self { extrusion, color } = *self;

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
        let mut tessellator = Tessellator::from_options(TessellationOptions {
            aa_size: extrusion,
            anti_alias: true,
            ..Default::default()
        });
        let mut mesh = Mesh::default();
        tessellator.tessellate_rect(&rect, &mut mesh);
        mesh
    }
}
