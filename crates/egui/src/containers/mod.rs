//! Containers are pieces of the UI which wraps other pieces of UI. Examples: [`Window`], [`ScrollArea`], [`Resize`], [`SidePanel`], etc.
//!
//! For instance, a [`Frame`] adds a frame and background to some contained UI.

pub(crate) mod area;
pub mod close_tag;
pub mod collapsing_header;
mod combo_box;
pub mod frame;
pub mod menu;
pub mod modal;
pub mod old_popup;
pub mod panel;
mod popup;
pub(crate) mod resize;
mod scene;
pub mod scroll_area;
mod sides;
mod tooltip;
pub(crate) mod window;

pub use {
    area::{Area, AreaState},
    collapsing_header::{CollapsingHeader, CollapsingResponse},
    combo_box::*,
    frame::Frame,
    modal::{Modal, ModalResponse},
    old_popup::*,
    panel::{CentralPanel, SidePanel, TopBottomPanel},
    popup::*,
    resize::Resize,
    scene::Scene,
    scroll_area::ScrollArea,
    sides::Sides,
    tooltip::*,
    window::Window,
};
