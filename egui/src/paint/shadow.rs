use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Shadow {
    // The shadow extends this much outside the rect.
    pub extrusion: f32,
    pub color: Srgba,
}

impl Shadow {
    /// Tooltips, menus, ...
    pub fn small() -> Self {
        Self {
            extrusion: 8.0,
            color: Srgba::black_alpha(64),
        }
    }

    /// Windows
    pub fn big() -> Self {
        Self {
            extrusion: 32.0,
            color: Srgba::black_alpha(96),
        }
    }

    pub fn tessellate(&self, rect: crate::Rect, corner_radius: f32) -> Triangles {
        // tessellator.clip_rect = clip_rect; // TODO: culling

        let Self { extrusion, color } = *self;

        use crate::paint::tessellator::*;
        let rect = PaintRect {
            rect: rect.expand(0.5 * extrusion),
            corner_radius: corner_radius + 0.5 * extrusion,
            fill: color,
            stroke: Default::default(),
        };
        let mut tessellator = Tessellator::from_options(TessellationOptions {
            aa_size: extrusion,
            anti_alias: true,
            ..Default::default()
        });
        let mut triangles = Triangles::default();
        tessellator.tessellate_rect(&rect, &mut triangles);
        triangles
    }
}
