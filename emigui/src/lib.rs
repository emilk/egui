#![deny(warnings)]

extern crate rusttype;
extern crate serde;

#[macro_use] // TODO: get rid of this
extern crate serde_derive;

pub mod color;
pub mod containers;
mod context;
mod emigui;
pub mod example_app;
mod font;
mod fonts;
mod id;
mod layers;
mod layout;
pub mod math;
mod memory;
pub mod mesher;
mod region;
mod style;
mod texture_atlas;
mod types;
pub mod widgets;

pub use {
    crate::emigui::Emigui,
    color::Color,
    context::Context,
    fonts::{FontDefinitions, Fonts, TextStyle},
    id::Id,
    layers::*,
    layout::{Align, GuiResponse},
    math::*,
    memory::Memory,
    mesher::{Mesh, PaintBatches, Vertex},
    region::Region,
    style::Style,
    texture_atlas::Texture,
    types::*,
};
