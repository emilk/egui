//! Containers are pieces of the UI which wraps other pieces of UI. Examples: [`Window`], [`ScrollArea`], [`Resize`], [`SidePanel`], etc.
//!
//! For instance, a [`Frame`] adds a frame and background to some contained UI.

pub(crate) mod area;
pub mod collapsing_header;
mod combo_box;
pub mod frame;
pub mod modal;
pub mod panel;
pub mod popup;
pub(crate) mod resize;
mod scene;
pub mod scroll_area;
mod sides;
pub(crate) mod window;

pub use {
    area::{Area, AreaState},
    collapsing_header::{CollapsingHeader, CollapsingResponse},
    combo_box::*,
    frame::Frame,
    modal::{Modal, ModalResponse},
    panel::{CentralPanel, SidePanel, TopBottomPanel},
    popup::*,
    resize::Resize,
    scene::Scene,
    scroll_area::ScrollArea,
    sides::Sides,
    window::Window,
};
