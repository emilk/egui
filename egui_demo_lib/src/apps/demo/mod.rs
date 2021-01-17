//! Demo-code for showing how egui is used.
//!
//! The demo-code is also used in benchmarks and tests.

// ----------------------------------------------------------------------------

mod app;
mod dancing_strings;
pub mod demo_window;
mod demo_windows;
mod drag_and_drop;
mod font_book;
pub mod font_contents_emoji;
pub mod font_contents_ubuntu;
mod painting;
mod scrolls;
mod sliders;
mod tests;
pub mod toggle_switch;
mod widget_gallery;
mod widgets;
mod window_options;

pub use {
    app::*, dancing_strings::DancingStrings, demo_window::DemoWindow, demo_windows::*,
    drag_and_drop::*, font_book::FontBook, painting::Painting, scrolls::Scrolls, sliders::Sliders,
    tests::Tests, widget_gallery::*, widgets::Widgets, window_options::WindowOptions,
};

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
