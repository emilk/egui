//! Graphics module.
//!
//! Handles fonts, textures, color, geometry and tesselation.

pub mod color;
pub mod command;
pub mod font;
pub mod fonts;
pub mod tessellator;
mod texture_atlas;

pub use {
    color::Color,
    command::{LineStyle, PaintCmd},
    fonts::{FontDefinitions, Fonts, TextStyle},
    tessellator::{PaintJobs, PaintOptions, Path, Triangles, Vertex},
    texture_atlas::Texture,
};
