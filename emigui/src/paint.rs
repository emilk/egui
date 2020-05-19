pub mod color;
pub mod command;
pub mod font;
pub mod fonts;
pub mod mesher;
mod texture_atlas;

pub use {
    color::Color,
    command::{Outline, PaintCmd},
    fonts::{FontDefinitions, Fonts, TextStyle},
    mesher::{PaintBatches, PaintOptions, Path, Triangles, Vertex},
    texture_atlas::Texture,
};
