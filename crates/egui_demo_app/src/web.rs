use core::cell::RefCell;

use eframe::wasm_bindgen::{self, prelude::*};

use crate::WrapApp;

/// Our handle to the web app from JavaScript.
#[derive(Clone)]
#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
}

#[wasm_bindgen]
impl WebHandle {
    /// Installs a panic hook, then returns.
    #[allow(clippy::allow_attributes, clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Redirect [`log`] message to `console.log` and friends:
        let log_level = if cfg!(debug_assertions) {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Debug
        };
        eframe::WebLogger::init(log_level).ok();

        Self {
            runner: eframe::WebRunner::new(),
        }
    }

    /// Call this once from JavaScript to start your app.
    ///
    /// # Errors
    /// Returns an error if the app could not start.
    #[wasm_bindgen]
    pub async fn start(
        &self,
        canvas: web_sys::HtmlCanvasElement,
    ) -> Result<(), wasm_bindgen::JsValue> {
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(WrapApp::new(cc)))),
            )
            .await
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

    /// The JavaScript can check whether or not your app has crashed:
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

thread_local! {
    /// The runner must outlive `start_offscreen_egui`; the worker has no JS object
    /// holding it (unlike [`WebHandle`] in DOM mode).
    static OFFSCREEN_RUNNER: RefCell<Option<eframe::WebRunner>> = const { RefCell::new(None) };
}

/// Start the demo in a web worker, rendering to an `OffscreenCanvas`.
///
/// Call this from the worker's `onmessage` handler with the canvas transferred
/// by the main thread.
///
/// # Errors
/// Returns an error if the renderer could not be initialized.
#[wasm_bindgen]
pub async fn start_offscreen_egui(
    canvas: web_sys::OffscreenCanvas,
) -> Result<(), wasm_bindgen::JsValue> {
    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Debug
    };
    eframe::WebLogger::init(log_level).ok();

    let runner = eframe::WebRunner::new();
    runner
        .start_offscreen(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|cc| Ok(Box::new(WrapApp::new(cc)))),
        )
        .await?;
    OFFSCREEN_RUNNER.with(|slot| *slot.borrow_mut() = Some(runner));
    Ok(())
}

/// Feed one message from the host page into the offscreen app.
#[wasm_bindgen]
#[expect(clippy::needless_pass_by_value)]
pub fn offscreen_on_message(message: wasm_bindgen::JsValue) {
    OFFSCREEN_RUNNER.with(|slot| {
        if let Some(runner) = slot.borrow().as_ref() {
            runner.on_worker_message(&message);
        }
    });
}
