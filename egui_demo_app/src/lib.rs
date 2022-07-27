//! Demo app for egui

mod apps;
mod backend_panel;
pub(crate) mod frame_history;
mod wrap_app;

#[cfg(target_arch = "wasm32")]
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use eframe::web::AppRunner;

#[cfg(target_arch = "wasm32")]
use egui::mutex::Mutex;

pub use wrap_app::WrapApp;

/// Time of day as seconds since midnight. Used for clock in demo app.
pub(crate) fn seconds_since_midnight() -> f64 {
    use chrono::Timelike;
    let time = chrono::Local::now().time();
    time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64)
}

// ----------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WebHandle {
    handle: Arc<Mutex<AppRunner>>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen]
    #[cfg(target_arch = "wasm32")]
    pub fn stop_web(&self) -> Result<(), wasm_bindgen::JsValue> {
        let res = self.handle.lock().destroy();

        // let numw = Arc::weak_count(&uu);
        // let nums = Arc::strong_count(&uu);
        // tracing::debug!("runner ref {:?}, {:?}", numw, nums);

        res
    }
}

/// This is the entry-point for all the web-assembly.
/// This is called once from the HTML.
/// It loads the app, installs some callbacks, then returns.
/// You can add more callbacks like this if you want to call in to your code.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<WebHandle, wasm_bindgen::JsValue> {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();
    let handle = eframe::start_web(
        canvas_id,
        web_options,
        Box::new(|cc| Box::new(WrapApp::new(cc))),
    )
    .map(|handle| WebHandle { handle });

    handle
}
