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
pub mod layout_test;
pub mod painting;
pub mod password;
pub mod plot_demo;
pub mod scrolling;
pub mod sliders;
pub mod tests;
pub mod toggle_switch;
pub mod widget_gallery;
mod widgets;
pub mod window_options;
pub mod zoom_rotate;

pub use {app::*, demo_window::DemoWindow, demo_windows::*, widgets::Widgets};

// ----------------------------------------------------------------------------

/// Something to view in the demo windows
pub trait View {
    fn ui(&mut self, ui: &mut egui::Ui);
}

/// Something to view
pub trait Demo {
    /// `&'static` so we can also use it as a key to store open/close state.
    fn name(&self) -> &'static str;

    /// Show windows, etc
    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool);
}
