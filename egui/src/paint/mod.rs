//! 2D graphics/rendering. Fonts, textures, color, geometry, tessellation etc.

pub mod color;
pub mod command;
pub mod font;
pub mod fonts;
mod galley;
pub mod stats;
pub mod tessellator;
mod texture_atlas;

pub use {
    color::{Rgba, Srgba},
    command::{PaintCmd, Stroke},
    fonts::{FontDefinitions, FontFamily, Fonts, TextStyle},
    galley::*,
    stats::PaintStats,
    tessellator::{
        PaintJob, PaintJobs, TessellationOptions, TextureId, Triangles, Vertex, WHITE_UV,
    },
    texture_atlas::{Texture, TextureAtlas},
};
