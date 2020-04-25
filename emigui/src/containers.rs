pub mod collapsing_header;
pub mod floating;
pub mod frame;
pub mod resize;
pub mod scroll_area;
pub mod window;

pub use {
    collapsing_header::CollapsingHeader, floating::Floating, frame::Frame, resize::Resize,
    scroll_area::ScrollArea, window::Window,
};

// TODO
// pub trait Container {
//     fn show(self, region: &mut Region, add_contents: impl FnOnce(&mut Region));
// }

// pub trait Container {
//     fn begin(&mut self, parent: &mut Region) -> Region;
//     fn end(self, parent: &mut Region, content: Region);
// }
