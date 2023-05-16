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
    #[allow(clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Redirect [`log`] message to `console.log` and friends:
        eframe::web::WebLogger::init(log::LevelFilter::Debug).ok();

        Self {
            runner: AppRunnerRef::new(),
        }
    }

    /// This is the entry-point for all the web-assembly.
    ///
    /// This is called once from the HTML.
    /// It loads the app, installs some callbacks, then returns.
    #[wasm_bindgen]
    pub async fn start(&self, canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
        let web_options = eframe::WebOptions::default();
        self.runner
            .start(
                canvas_id,
                web_options,
                Box::new(|cc| Box::new(WrapApp::new(cc))),
            )
            .await?;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn destroy(&self) {
        self.runner.destroy();
    }

    /// Example on how to call into your app from JavaScript.
    #[wasm_bindgen]
    pub fn example(&self) {
        if let Some(_app) = self.runner.app_mut::<WrapApp>() {
            // _app.example();
        }
    }

    #[wasm_bindgen]
    pub fn has_panicked(&self) -> bool {
        self.runner.has_panicked()
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
