//! Graphics module.
//!
//! Handles fonts, textures, color, geometry and tesselation.

pub mod color;
pub mod command;
pub mod font;
pub mod fonts;
pub mod stats;
pub mod tessellator;
mod texture_atlas;

pub use {
    color::{Rgba, Srgba},
    command::{PaintCmd, Stroke},
    fonts::{FontDefinitions, Fonts, TextStyle},
    stats::PaintStats,
    tessellator::{PaintJob, PaintJobs, PaintOptions, TextureId, Triangles, Vertex, WHITE_UV},
    texture_atlas::Texture,
};
