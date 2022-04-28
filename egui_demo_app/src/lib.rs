//! Demo app for egui

mod apps;
mod backend_panel;
pub(crate) mod frame_history;
mod wrap_app;

pub use wrap_app::WrapApp;

/// Time of day as seconds since midnight. Used for clock in demo app.
pub(crate) fn seconds_since_midnight() -> Option<f64> {
    use chrono::Timelike;
    let time = chrono::Local::now().time();
    let seconds_since_midnight =
        time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64);
    Some(seconds_since_midnight)
}

// ----------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

/// This is the entry-point for all the web-assembly.
/// This is called once from the HTML.
/// It loads the app, installs some callbacks, then returns.
/// You can add more callbacks like this if you want to call in to your code.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    eframe::start_web(canvas_id, Box::new(|cc| Box::new(WrapApp::new(cc))))
}
