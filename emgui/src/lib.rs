#![deny(warnings)]

extern crate serde;

#[macro_use] // TODO: get rid of this
extern crate serde_derive;

mod emgui;
mod layout;
pub mod math;
mod style;
pub mod types;

pub use crate::{
    emgui::Emgui, layout::Layout, layout::LayoutOptions, style::Style, types::RawInput,
};
