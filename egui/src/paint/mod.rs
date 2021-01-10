//! 2D graphics/rendering. Fonts, textures, color, geometry, tessellation etc.

pub mod color;
mod shadow;
pub mod shape;
pub mod stats;
mod stroke;
pub mod tessellator;
pub mod text;
mod texture_atlas;
mod triangles;

pub use {
    color::{Color32, Rgba},
    shadow::Shadow,
    shape::Shape,
    stats::PaintStats,
    stroke::Stroke,
    tessellator::{PaintJob, PaintJobs, TessellationOptions},
    text::{Galley, TextStyle},
    texture_atlas::{Texture, TextureAtlas},
    triangles::{Triangles, Vertex},
};

/// The UV coordinate of a white region of the texture mesh.
/// The default Egui texture has the top-left corner pixel fully white.
/// You need need use a clamping texture sampler for this to work
/// (so it doesn't do bilinear blending with bottom right corner).
pub const WHITE_UV: crate::Pos2 = crate::pos2(0.0, 0.0);

/// What texture to use in a [`Triangles`] mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureId {
    /// The Egui font texture.
    /// If you don't want to use a texture, pick this and the [`WHITE_UV`] for uv-coord.
    Egui,

    /// Your own texture, defined in any which way you want.
    /// Egui won't care. The backend renderer will presumably use this to look up what texture to use.
    User(u64),
}

impl Default for TextureId {
    fn default() -> Self {
        Self::Egui
    }
}

pub(crate) struct PaintRect {
    pub rect: crate::Rect,
    /// How rounded the corners are. Use `0.0` for no rounding.
    pub corner_radius: f32,
    pub fill: Color32,
    pub stroke: Stroke,
}
