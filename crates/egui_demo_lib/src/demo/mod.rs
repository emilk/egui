//! Demo-code for showing how egui is used.
//!
//! The demo-code is also used in benchmarks and tests.

// ----------------------------------------------------------------------------

pub mod about;
pub mod code_editor;
pub mod code_example;
pub mod dancing_strings;
pub mod demo_app_windows;
pub mod drag_and_drop;
pub mod extra_viewport;
pub mod font_book;
pub mod frame_demo;
pub mod highlighting;
pub mod interactive_container;
pub mod misc_demo_window;
pub mod modals;
pub mod multi_touch;
pub mod paint_bezier;
pub mod painting;
pub mod panels;
pub mod password;
mod popups;
pub mod scene;
pub mod screenshot;
pub mod scrolling;
pub mod sliders;
pub mod strip_demo;
pub mod table_demo;
pub mod tests;
pub mod text_edit;
pub mod text_layout;
pub mod toggle_switch;
pub mod tooltips;
pub mod undo_redo;
pub mod widget_gallery;
pub mod window_options;

pub use {
    about::About, demo_app_windows::DemoWindows, misc_demo_window::MiscDemoWindow,
    widget_gallery::WidgetGallery,
};

// ----------------------------------------------------------------------------

/// Something to view in the demo windows
pub trait View {
    fn ui(&mut self, ui: &mut egui::Ui);
}

/// Something to view
pub trait Demo {
    /// Is the demo enabled for this integration?
    fn is_enabled(&self, _ctx: &egui::Context) -> bool {
        true
    }

    /// `&'static` so we can also use it as a key to store open/close state.
    fn name(&self) -> &'static str;

    /// Show windows, etc
    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool);
}
