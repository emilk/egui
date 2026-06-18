//! Demo app for egui

mod apps;
mod backend_panel;
mod frame_history;
mod wrap_app;

pub use wrap_app::{Anchor, WrapApp};

/// Time of day as seconds since midnight. Used for clock in demo app.
pub(crate) fn seconds_since_midnight() -> f64 {
    jiff::Zoned::now()
        .time()
        .duration_since(jiff::civil::Time::midnight())
        .as_secs_f64()
}

/// Trait that wraps different parts of the demo app.
pub trait DemoApp {
    fn demo_ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame);

    #[cfg(feature = "glow")]
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {}
}

// ----------------------------------------------------------------------------
#[cfg(feature = "accessibility_inspector")]
pub mod accessibility_inspector;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
pub use web::*;
