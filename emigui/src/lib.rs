#![deny(warnings)]

extern crate rusttype;
extern crate serde;

#[macro_use] // TODO: get rid of this
extern crate serde_derive;

pub mod color;
mod emigui;
pub mod example_app;
mod font;
mod fonts;
mod layers;
mod layout;
pub mod math;
pub mod mesher;
mod style;
mod texture_atlas;
mod types;
pub mod widgets;
mod window;

pub use {
    crate::emigui::Emigui,
    color::Color,
    fonts::{FontDefinitions, Fonts, TextStyle},
    layers::*,
    layout::{Align, Id, Region},
    math::*,
    mesher::{Mesh, Vertex},
    style::Style,
    texture_atlas::Texture,
    types::*,
    window::Window,
};
