use std::sync::Arc;

use egui::mutex::Mutex;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(msg: String);

    type Error;

    #[wasm_bindgen(constructor)]
    fn new() -> Error;

    #[wasm_bindgen(structural, method, getter)]
    fn stack(error: &Error) -> String;
}

#[derive(Clone, Debug)]
pub struct PanicSummary {
    message: String,
    callstack: String,
}

impl PanicSummary {
    pub fn new(info: &std::panic::PanicInfo<'_>) -> Self {
        let message = info.to_string();
        let callstack = Error::new().stack();
        Self { message, callstack }
    }

    pub fn message(&self) -> String {
        self.message.clone()
    }

    pub fn callstack(&self) -> String {
        self.callstack.clone()
    }
}

/// Handle to information about any panic than has occurred
#[derive(Clone, Default)]
struct PanicHandlerInner {
    summary: Option<PanicSummary>,
}

impl PanicHandlerInner {
    pub fn has_panicked(&self) -> bool {
        self.summary.is_some()
    }

    pub fn panic_summary(&self) -> Option<PanicSummary> {
        self.summary.clone()
    }

    pub fn on_panic(&mut self, info: &std::panic::PanicInfo<'_>) {
        self.summary = Some(PanicSummary::new(info));
    }
}

/// Handle to information about any panic than has occurred.
#[derive(Clone)]
pub struct PanicHandler(Arc<Mutex<PanicHandlerInner>>);

impl PanicHandler {
    pub fn install() -> Self {
        let handler = Self(Arc::new(Mutex::new(Default::default())));

        let handler_clone = handler.clone();
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            log::info!("eframe detected a panic");
            handler_clone.on_panic(panic_info);

            // Propagate panic info to the previously registered panic hook
            previous_hook(panic_info);
        }));

        handler
    }

    pub fn has_panicked(&self) -> bool {
        self.0.lock().has_panicked()
    }

    pub fn panic_summary(&self) -> Option<PanicSummary> {
        self.0.lock().panic_summary()
    }

    fn on_panic(&self, info: &std::panic::PanicInfo<'_>) {
        self.0.lock().on_panic(info);
    }
}
