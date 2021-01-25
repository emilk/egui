//! Demo-code for showing how egui is used.
//!
//! The demo-code is also used in benchmarks and tests.

// ----------------------------------------------------------------------------

mod app;
pub mod dancing_strings;
pub mod demo_window;
mod demo_windows;
pub mod drag_and_drop;
pub mod font_book;
pub mod font_contents_emoji;
pub mod font_contents_ubuntu;
pub mod input_test;
pub mod layout_test;
pub mod painting;
pub mod scrolling;
pub mod sliders;
pub mod tests;
pub mod toggle_switch;
pub mod widget_gallery;
mod widgets;
pub mod window_options;

pub use {app::*, demo_window::DemoWindow, demo_windows::*, widgets::Widgets};

// ----------------------------------------------------------------------------

/// Something to view in the demo windows
pub trait View {
    fn ui(&mut self, ui: &mut egui::Ui);
}

/// Something to view
pub trait Demo {
    fn name(&self) -> &str;

    /// Show windows, etc
    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool);
}
