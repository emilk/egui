//! The channel from the worker back to the host page.

use wasm_bindgen::{JsCast as _, JsValue};

/// Sends messages to the main thread from a worker.
///
/// Constructed with [`WebHost::new`], which returns `None` when the global
/// object has no `postMessage` function.
#[derive(Clone)]
pub(crate) struct WebHost {
    post_message: js_sys::Function,

    /// The `{ type: "request_frame" }` message. It never changes and is posted
    /// on every paint, so it is built once.
    request_frame_message: JsValue,
}

impl core::fmt::Debug for WebHost {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("WebHost")
    }
}

impl WebHost {
    pub fn new() -> Option<Self> {
        let post_message =
            js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("postMessage"))
                .ok()?
                .dyn_into::<js_sys::Function>()
                .ok()?;

        let request_frame_message = js_sys::Object::new();
        set(&request_frame_message, "type", "request_frame");

        Some(Self {
            post_message,
            request_frame_message: request_frame_message.into(),
        })
    }

    /// Post a message object to the main thread.
    pub fn send(&self, message: &JsValue) {
        if let Err(error) = self.post_message.call1(&js_sys::global(), message) {
            log::error!(
                "failed to postMessage to the host page: {}",
                super::string_from_js_value(&error)
            );
        }
    }

    /// Ask the host page to change the CSS cursor of the visible canvas.
    pub fn send_cursor_icon(&self, icon: egui::CursorIcon) {
        let message = js_sys::Object::new();
        set(&message, "type", "cursor");
        set(&message, "icon", super::cursor_web_name(icon));
        self.send(&message);
    }

    /// Ask the host page to send one `{type: "frame"}` message on its next
    /// animation frame. Used in worker mode, where there is no `window` to call
    /// `requestAnimationFrame` on.
    pub fn request_frame(&self) {
        self.send(&self.request_frame_message);
    }
}

fn set(object: &js_sys::Object, key: &str, value: &str) {
    let _ = js_sys::Reflect::set(object, &JsValue::from_str(key), &JsValue::from_str(value));
}
