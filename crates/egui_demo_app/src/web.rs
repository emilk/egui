use eframe::{
    wasm_bindgen::{self, prelude::*},
    web::AppRunnerRef,
};

use crate::WrapApp;

#[wasm_bindgen]
pub struct WebHandle {
    runner: AppRunnerRef,
}

#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen]
    pub fn stop_web(&self) -> Result<(), wasm_bindgen::JsValue> {
        let mut app = self.runner.lock();
        app.destroy()
    }

    #[wasm_bindgen]
    pub fn has_panicked(&self) -> bool {
        self.runner.panic_summary().is_some()
    }
}

/// This is the entry-point for all the web-assembly.
/// This is called once from the HTML.
/// It loads the app, installs some callbacks, then returns.
/// You can add more callbacks like this if you want to call in to your code.
#[wasm_bindgen]
pub async fn start(canvas_id: &str) -> Result<WebHandle, wasm_bindgen::JsValue> {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    eframe::web::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();
    let runner = eframe::start_web(
        canvas_id,
        web_options,
        Box::new(|cc| Box::new(WrapApp::new(cc))),
    )
    .await?;

    Ok(WebHandle { runner })
}
