//! Containers are pieces of the UI which wraps other pieces of UI.
//!
//! For instance, a `Frame` adds a frame and background to some contained UI.

pub(crate) mod area;
pub(crate) mod collapsing_header;
pub(crate) mod frame;
pub(crate) mod popup;
pub(crate) mod resize;
pub(crate) mod scroll_area;
pub(crate) mod window;

pub use {
    area::Area, collapsing_header::CollapsingHeader, frame::Frame, popup::*, resize::Resize,
    scroll_area::ScrollArea, window::Window,
};
