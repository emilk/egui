//! Demo app for egui

mod apps;
mod backend_panel;
pub(crate) mod frame_history;
mod wrap_app;

pub use wrap_app::WrapApp;

/// Time of day as seconds since midnight. Used for clock in demo app.
pub(crate) fn seconds_since_midnight() -> f64 {
    use chrono::Timelike;
    let time = chrono::Local::now().time();
    time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64)
}
