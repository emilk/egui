//! 2D graphics/rendering. Fonts, textures, color, geometry, tessellation etc.

pub mod color;
pub mod font;
pub mod fonts;
mod galley;
mod shadow;
pub mod shape;
pub mod stats;
pub mod tessellator;
mod texture_atlas;

pub use {
    color::{Color32, Rgba},
    fonts::{FontDefinitions, FontFamily, Fonts, TextStyle},
    galley::*,
    shadow::Shadow,
    shape::{Shape, Stroke},
    stats::PaintStats,
    tessellator::{
        PaintJob, PaintJobs, TessellationOptions, TextureId, Triangles, Vertex, WHITE_UV,
    },
    texture_atlas::{Texture, TextureAtlas},
};

pub(crate) struct PaintRect {
    pub rect: crate::Rect,
    /// How rounded the corners are. Use `0.0` for no rounding.
    pub corner_radius: f32,
    pub fill: Color32,
    pub stroke: Stroke,
}
