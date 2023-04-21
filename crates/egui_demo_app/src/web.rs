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
    /// This is the entry-point for all the web-assembly.
    ///
    /// This is called once from the HTML.
    /// It loads the app, installs some callbacks, then returns.
    #[wasm_bindgen(constructor)]
    pub async fn new(canvas_id: &str) -> Result<WebHandle, wasm_bindgen::JsValue> {
        // Redirect tracing to console.log and friends:
        eframe::web::WebLogger::init(log::LevelFilter::Debug).ok();

        // Make sure panics are logged using `console.error`.
        console_error_panic_hook::set_once();

        let web_options = eframe::WebOptions::default();
        let runner = eframe::start_web(
            canvas_id,
            web_options,
            Box::new(|cc| Box::new(WrapApp::new(cc))),
        )
        .await?;

        Ok(WebHandle { runner })
    }

    #[wasm_bindgen]
    pub fn destroy(&self) {
        self.runner.destroy();
    }

    #[wasm_bindgen]
    pub fn has_panicked(&self) -> bool {
        self.runner.panic_summary().is_some()
    }

    #[wasm_bindgen]
    pub fn panic_message(&self) -> Option<String> {
        self.runner.panic_summary().map(|s| s.message())
    }

    #[wasm_bindgen]
    pub fn panic_callstack(&self) -> Option<String> {
        self.runner.panic_summary().map(|s| s.callstack())
    }
}
