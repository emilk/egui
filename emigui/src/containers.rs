pub mod area;
pub mod collapsing_header;
pub mod frame;
pub mod menu;
pub mod resize;
pub mod scroll_area;
pub mod window;

pub use {
    area::Area, collapsing_header::CollapsingHeader, frame::Frame, resize::Resize,
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
