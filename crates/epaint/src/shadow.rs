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
    pub const NONE: Self = Self {
        extrusion: 0.0,
        color: Color32::TRANSPARENT,
    };

    /// Tooltips, menus, …, for dark mode.
    pub fn small_dark() -> Self {
        Self {
            extrusion: 16.0,
            color: Color32::from_black_alpha(96),
        }
    }

    /// Tooltips, menus, …, for light mode.
    pub fn small_light() -> Self {
        Self {
            extrusion: 16.0,
            color: Color32::from_black_alpha(20),
        }
    }

    /// Used for egui windows in dark mode.
    pub fn big_dark() -> Self {
        Self {
            extrusion: 32.0,
            color: Color32::from_black_alpha(96),
        }
    }

    /// Used for egui windows in light mode.
    pub fn big_light() -> Self {
        Self {
            extrusion: 32.0,
            color: Color32::from_black_alpha(16),
        }
    }
}

/// Functions for painting shadows
pub struct ShadowPainter {
    clip_rect: std::sync::Arc<dyn Fn(Shadow, Rect, Rounding) -> Rect + Send + Sync>,
    paint: std::sync::Arc<dyn Fn(Shadow, Rect, Rounding) -> Shape + Send + Sync>,
}
impl ShadowPainter {
    /// Create a custom [`ShadowPainter`].
    /// For arguments, see [`Self::shadow_clip_rect`] and [`Self::paint`]
    pub fn new(
        clip_rect: impl Fn(Shadow, Rect, Rounding) -> Rect + Send + Sync + 'static,
        paint: impl Fn(Shadow, Rect, Rounding) -> Shape + Send + Sync + 'static,
    ) -> Self {
        Self {
            clip_rect: std::sync::Arc::new(clip_rect),
            paint: std::sync::Arc::new(paint),
        }
    }

    /// Transform (typically expand) a clip rect to fit the given shadow
    pub fn shadow_clip_rect(&self, shadow: Shadow, content_rect: Rect, rounding: Rounding) -> Rect {
        (self.clip_rect)(shadow, content_rect, rounding)
    }

    /// Convert a shadow into a [`Shape`]
    pub fn paint(&self, shadow: Shadow, content_rect: Rect, rounding: Rounding) -> Shape {
        (self.paint)(shadow, content_rect, rounding)
    }
}
impl Default for ShadowPainter {
    fn default() -> Self {
        Self::new(
            |shadow, rect, _| rect.expand(shadow.extrusion),
            |shadow, rect, rounding| {
                // tessellator.clip_rect = clip_rect; // TODO(emilk): culling

                let Shadow { extrusion, color } = shadow;

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
                Shape::Mesh(mesh)
            },
        )
    }
}
