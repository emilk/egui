#![deny(warnings)]

extern crate rusttype;
extern crate serde;

#[macro_use] // TODO: get rid of this
extern crate serde_derive;

mod emgui;
mod font;
mod layout;
pub mod math;
mod painter;
mod style;
pub mod types;

pub use crate::{
    emgui::Emgui,
    layout::Layout,
    layout::LayoutOptions,
    painter::{Frame, Painter, Vertex},
    style::Style,
    types::RawInput,
};
