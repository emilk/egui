//! Graphics module.
//!
//! Handles fonts, textures, color, geometry and tesselation.

pub mod color;
pub mod command;
pub mod fonts;
pub mod tessellator;
mod texture_atlas;

pub use {
    color::{Rgba, Srgba},
    command::{PaintCmd, Stroke},
    fonts::{FontConfiguration, Fonts, TextStyle},
    tessellator::{PaintJobs, PaintOptions, TextureId, Triangles, Vertex, WHITE_UV},
    texture_atlas::Texture,
};
