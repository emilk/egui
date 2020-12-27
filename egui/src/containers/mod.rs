//! Containers are pieces of the UI which wraps other pieces of UI. Examples: [`Window`], [`ScrollArea`], [`Resize`], etc.
//!
//! For instance, a [`Frame`] adds a frame and background to some contained UI.

pub(crate) mod area;
pub(crate) mod collapsing_header;
mod combo_box;
pub(crate) mod frame;
pub(crate) mod panel;
pub(crate) mod popup;
pub(crate) mod resize;
pub(crate) mod scroll_area;
pub(crate) mod window;

pub use {
    area::Area,
    collapsing_header::*,
    combo_box::*,
    frame::Frame,
    panel::{CentralPanel, SidePanel, TopPanel},
    popup::*,
    resize::Resize,
    scroll_area::ScrollArea,
    window::Window,
};
