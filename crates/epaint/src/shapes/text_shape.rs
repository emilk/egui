use std::sync::Arc;

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

    /// The laid out text, from [`Fonts::layout_job`].
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
        self.galley.mesh_bounds.translate(self.pos.to_vec2())
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

    /// Rotate text by this many radians clockwise.
    /// The pivot is `pos` (the upper left corner of the text).
    #[inline]
    pub fn with_angle(mut self, angle: f32) -> Self {
        self.angle = angle;
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
        } = Arc::make_mut(galley);

        *rect = transform.scaling * *rect;
        *mesh_bounds = transform.scaling * *mesh_bounds;

        for text::PlacedRow { pos, row } in rows {
            *pos *= transform.scaling;

            let text::Row {
                section_index_at_start: _,
                glyphs: _, // TODO(emilk): would it make sense to transform these?
                size,
                visuals,
                ends_with_newline: _,
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
