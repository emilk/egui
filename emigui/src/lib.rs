#![deny(warnings)]

extern crate rusttype;
extern crate serde;

#[macro_use] // TODO: get rid of this
extern crate serde_derive;

mod emigui;
mod font;
mod fonts;
mod layout;
pub mod math;
pub mod mesher;
mod style;
mod texture_atlas;
pub mod types;
pub mod widgets;

pub use crate::{
    emigui::Emigui,
    fonts::{FontSizes, Fonts, TextStyle},
    layout::{Align, Region},
    mesher::{Mesh, Vertex},
    style::Style,
    texture_atlas::Texture,
    types::RawInput,
};
