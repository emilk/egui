use std::sync::Arc;

use egui::mutex::Mutex;
use wasm_bindgen::prelude::*;

/// Detects panics, logs them using `console.error`, and stores the panics message and callstack.
///
/// This lets you query `PanicHandler` for the panic message (if any) so you can show it in the HTML.
///
/// Chep to clone (ref-counted).
#[derive(Clone)]
pub struct PanicHandler(Arc<Mutex<PanicHandlerInner>>);

impl PanicHandler {
    /// Install a panic hook.
    pub fn install() -> Self {
        let handler = Self(Arc::new(Mutex::new(Default::default())));

        let handler_clone = handler.clone();
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let summary = PanicSummary::new(panic_info);

            // Log it using console.error
            error(format!(
                "{}\n\nStack:\n\n{}",
                summary.message(),
                summary.callstack()
            ));

            // Remember the summary:
            handler_clone.0.lock().summary = Some(summary);

            // Propagate panic info to the previously registered panic hook
            previous_hook(panic_info);
        }));

        handler
    }

    /// Has there been a panic?
    pub fn has_panicked(&self) -> bool {
        self.0.lock().summary.is_some()
    }

    /// What was the panic message and callstack?
    pub fn panic_summary(&self) -> Option<PanicSummary> {
        self.0.lock().summary.clone()
    }
}

#[derive(Clone, Default)]
struct PanicHandlerInner {
    summary: Option<PanicSummary>,
}

/// Contains a summary about a panics.
///
/// This is basically a human-readable version of [`std::panic::PanicInfo`]
/// with an added callstack.
#[derive(Clone, Debug)]
pub struct PanicSummary {
    message: String,
    callstack: String,
}

impl PanicSummary {
    /// Construct a summary from a panic.
    pub fn new(info: &std::panic::PanicInfo<'_>) -> Self {
        let message = info.to_string();
        let callstack = Error::new().stack();
        Self { message, callstack }
    }

    /// The panic message.
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// The backtrace.
    pub fn callstack(&self) -> String {
        self.callstack.clone()
    }
}

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
