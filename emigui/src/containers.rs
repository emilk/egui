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
//     fn show(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui));
// }

// pub trait Container {
//     fn begin(&mut self, parent: &mut Ui) -> Ui;
//     fn end(self, parent: &mut Ui, content: Ui);
// }
