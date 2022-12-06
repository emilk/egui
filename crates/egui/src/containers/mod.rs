//! Containers are pieces of the UI which wraps other pieces of UI. Examples: [`Window`], [`ScrollArea`], [`Resize`], [`SidePanel`], etc.
//!
//! For instance, a [`Frame`] adds a frame and background to some contained UI.

pub(crate) mod area;
pub mod collapsing_header;
mod combo_box;
pub(crate) mod frame;
pub mod panel;
pub mod popup;
pub(crate) mod resize;
pub mod scroll_area;
pub(crate) mod window;

pub use {
    area::Area,
    collapsing_header::{CollapsingHeader, CollapsingResponse},
    combo_box::*,
    frame::Frame,
    panel::{CentralPanel, SidePanel, TopBottomPanel},
    popup::*,
    resize::Resize,
    scroll_area::ScrollArea,
    window::Window,
};
