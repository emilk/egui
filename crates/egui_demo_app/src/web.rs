use eframe::{
    wasm_bindgen::{self, prelude::*},
    web::AppRunnerRef,
};
use egui::mutex::Mutex;

use crate::WrapApp;

#[wasm_bindgen]
pub struct WebHandle {
    panic_handler: eframe::web::PanicHandler,
    runner: Mutex<Option<AppRunnerRef>>,
}

#[wasm_bindgen]
impl WebHandle {
    #[allow(clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Redirect tracing to console.log and friends:
        eframe::web::WebLogger::init(log::LevelFilter::Debug).ok();

        // Install a panic handler right away so we can catch any panics
        // during initialization and report them to the user:
        let panic_handler = eframe::web::PanicHandler::install();

        Self {
            panic_handler,
            runner: Mutex::new(None),
        }
    }

    /// This is the entry-point for all the web-assembly.
    ///
    /// This is called once from the HTML.
    /// It loads the app, installs some callbacks, then returns.
    #[wasm_bindgen]
    pub async fn start(&self, canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
        self.destroy();

        let web_options = eframe::WebOptions::default();
        let runner = eframe::start_web(
            canvas_id,
            self.panic_handler.clone(),
            web_options,
            Box::new(|cc| Box::new(WrapApp::new(cc))),
        )
        .await?;
        *self.runner.lock() = Some(runner);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn destroy(&self) {
        if let Some(runner) = self.runner.lock().take() {
            runner.destroy();
        }
    }

    #[wasm_bindgen]
    pub fn has_panicked(&self) -> bool {
        self.panic_handler.has_panicked()
    }

    #[wasm_bindgen]
    pub fn panic_message(&self) -> Option<String> {
        self.panic_handler.panic_summary().map(|s| s.message())
    }

    #[wasm_bindgen]
    pub fn panic_callstack(&self) -> Option<String> {
        self.panic_handler.panic_summary().map(|s| s.callstack())
    }
}
