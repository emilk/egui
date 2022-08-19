//! Demo app for egui

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use egui_demo_app::WrapApp;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        drag_and_drop_support: true,

        initial_window_size: Some([1280.0, 1024.0].into()),

        #[cfg(feature = "wgpu")]
        renderer: eframe::Renderer::Wgpu,

        ..Default::default()
    };
    eframe::run_native(
        "egui demo app",
        options,
        Box::new(|cc| Box::new(WrapApp::new(cc))),
    );
}

#[cfg(target_arch = "wasm32")]
use eframe::web::AppRunnerRef;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WebHandle {
    handle: AppRunnerRef,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen]
    #[cfg(target_arch = "wasm32")]
    pub fn stop_web(&self) -> Result<(), wasm_bindgen::JsValue> {
        let mut app = self.handle.lock();
        let res = app.destroy();

        // let numw = Arc::weak_count(&app);
        // let nums = Arc::strong_count(&app);
        // tracing::debug!("runner ref {:?}, {:?}", numw, nums);

        res
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn init_wasm_hooks() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start_separate(canvas_id: &str) -> Result<WebHandle, wasm_bindgen::JsValue> {
    let web_options = eframe::WebOptions::default();
    let handle = eframe::start_web(
        canvas_id,
        web_options,
        Box::new(|cc| Box::new(WrapApp::new(cc))),
    )
    .map(|handle| WebHandle { handle });

    handle
}

/// This is the entry-point for all the web-assembly.
/// This is called once from the HTML.
/// It loads the app, installs some callbacks, then returns.
/// You can add more callbacks like this if you want to call in to your code.
#[cfg(target_arch = "wasm32")]
fn main() {
    fn remove_loading_text() -> Option<()> {
        web_sys::window()?
            .document()?
            .get_element_by_id("loading_text")?
            .remove();
        None
    }

    let canvas_id = "the_canvas_id";
    init_wasm_hooks();
    start_separate(canvas_id).unwrap();
    remove_loading_text();
}
