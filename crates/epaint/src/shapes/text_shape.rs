use std::sync::Arc;

use emath::{Align2, Rot2};

use crate::*;

/// How to paint some text on screen.
///
/// This needs to be recreated if `pixels_per_point` (dpi scale) changes.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextShape {
    /// Where the origin of [`Self::galley`] is.
    ///
    /// Usually the top left corner of the first character.
    pub pos: Pos2,

    /// The laid out text, from [`FontsView::layout_job`].
    pub galley: Arc<Galley>,

    /// Add this underline to the whole text.
    /// You can also set an underline when creating the galley.
    pub underline: Stroke,

    /// Any [`Color32::PLACEHOLDER`] in the galley will be replaced by the given color.
    /// Affects everything: backgrounds, glyphs, strikethrough, underline, etc.
    pub fallback_color: Color32,

    /// If set, the text color in the galley will be ignored and replaced
    /// with the given color.
    ///
    /// This only affects the glyphs and will NOT replace background color nor strikethrough/underline color.
    pub override_text_color: Option<Color32>,

    /// If set, the text will be rendered with the given opacity in gamma space
    /// Affects everything: backgrounds, glyphs, strikethrough, underline, etc.
    pub opacity_factor: f32,

    /// Rotate text by this many radians clockwise.
    /// The pivot is `pos` (the upper left corner of the text).
    pub angle: f32,
}

impl TextShape {
    /// The given fallback color will be used for any uncolored part of the galley (using [`Color32::PLACEHOLDER`]).
    ///
    /// Any non-placeholder color in the galley takes precedence over this fallback color.
    #[inline]
    pub fn new(pos: Pos2, galley: Arc<Galley>, fallback_color: Color32) -> Self {
        Self {
            pos,
            galley,
            underline: Stroke::NONE,
            fallback_color,
            override_text_color: None,
            opacity_factor: 1.0,
            angle: 0.0,
        }
    }

    /// The visual bounding rectangle
    #[inline]
    pub fn visual_bounding_rect(&self) -> Rect {
        self.galley
            .mesh_bounds
            .rotate_bb(emath::Rot2::from_angle(self.angle))
            .translate(self.pos.to_vec2())
    }

    #[inline]
    pub fn with_underline(mut self, underline: Stroke) -> Self {
        self.underline = underline;
        self
    }

    /// Use the given color for the text, regardless of what color is already in the galley.
    #[inline]
    pub fn with_override_text_color(mut self, override_text_color: Color32) -> Self {
        self.override_text_color = Some(override_text_color);
        self
    }

    /// Set text rotation to `angle` radians clockwise.
    /// The pivot is `pos` (the upper left corner of the text).
    #[inline]
    pub fn with_angle(mut self, angle: f32) -> Self {
        self.angle = angle;
        self
    }

    /// Set the text rotation to the `angle` radians clockwise.
    /// The pivot is determined by the given `anchor` point on the text bounding box.
    #[inline]
    pub fn with_angle_and_anchor(mut self, angle: f32, anchor: Align2) -> Self {
        self.angle = angle;
        let a0 = anchor.pos_in_rect(&self.galley.rect).to_vec2();
        let a1 = Rot2::from_angle(angle) * a0;
        self.pos += a0 - a1;
        self
    }

    /// Render text with this opacity in gamma space
    #[inline]
    pub fn with_opacity_factor(mut self, opacity_factor: f32) -> Self {
        self.opacity_factor = opacity_factor;
        self
    }

    /// Move the shape by this many points, in-place.
    pub fn transform(&mut self, transform: emath::TSTransform) {
        let Self {
            pos,
            galley,
            underline,
            fallback_color: _,
            override_text_color: _,
            opacity_factor: _,
            angle: _,
        } = self;

        *pos = transform * *pos;
        underline.width *= transform.scaling;

        let Galley {
            job: _,
            rows,
            elided: _,
            rect,
            mesh_bounds,
            num_vertices: _,
            num_indices: _,
            pixels_per_point: _,
            intrinsic_size,
        } = Arc::make_mut(galley);

        *rect = transform.scaling * *rect;
        *mesh_bounds = transform.scaling * *mesh_bounds;
        *intrinsic_size = transform.scaling * *intrinsic_size;

        for text::PlacedRow {
            pos,
            row,
            ends_with_newline: _,
        } in rows
        {
            *pos *= transform.scaling;

            let text::Row {
                section_index_at_start: _,
                glyphs: _, // TODO(emilk): would it make sense to transform these?
                size,
                visuals,
            } = Arc::make_mut(row);

            *size *= transform.scaling;

            let text::RowVisuals {
                mesh,
                mesh_bounds,
                glyph_index_start: _,
                glyph_vertex_range: _,
            } = visuals;

            *mesh_bounds = transform.scaling * *mesh_bounds;

            for v in &mut mesh.vertices {
                v.pos *= transform.scaling;
            }
        }
    }
}

impl From<TextShape> for Shape {
    #[inline(always)]
    fn from(shape: TextShape) -> Self {
        Self::Text(shape)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::*, *};
    use crate::text::FontDefinitions;
    use emath::almost_equal;

    #[test]
    fn text_bounding_box_under_rotation() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::default());
        let font = FontId::monospace(12.0);

        let mut t = crate::Shape::text(
            &mut fonts.with_pixels_per_point(1.0),
            Pos2::ZERO,
            emath::Align2::CENTER_CENTER,
            "testing123",
            font,
            Color32::BLACK,
        );

        let size_orig = t.visual_bounding_rect().size();

        // 90 degree rotation
        if let Shape::Text(ts) = &mut t {
            ts.angle = std::f32::consts::PI / 2.0;
        }

        let size_rot = t.visual_bounding_rect().size();

        // make sure the box is actually rotated
        assert!(almost_equal(size_orig.x, size_rot.y, 1e-4));
        assert!(almost_equal(size_orig.y, size_rot.x, 1e-4));
    }
}
