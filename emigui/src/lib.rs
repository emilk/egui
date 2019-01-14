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
mod painter;
mod style;
mod texture_atlas;
pub mod types;
pub mod widgets;

pub use crate::{
    emigui::Emigui,
    fonts::TextStyle,
    layout::{Align, LayoutOptions, Region},
    painter::{Frame, Painter, Vertex},
    style::Style,
    types::RawInput,
};
