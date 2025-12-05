//! Containers are pieces of the UI which wraps other pieces of UI. Examples: [`Window`], [`ScrollArea`], [`Resize`], [`Panel`], etc.
//!
//! For instance, a [`Frame`] adds a frame and background to some contained UI.

pub(crate) mod area;
mod close_tag;
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

pub use area::{Area, AreaState};
pub use close_tag::ClosableTag;
pub use collapsing_header::{CollapsingHeader, CollapsingResponse};
pub use combo_box::*;
pub use frame::Frame;
pub use modal::{Modal, ModalResponse};
pub use old_popup::*;
pub use panel::*;
pub use popup::*;
pub use resize::Resize;
pub use scene::{DragPanButtons, Scene};
pub use scroll_area::ScrollArea;
pub use sides::Sides;
pub use tooltip::*;
pub use window::Window;
