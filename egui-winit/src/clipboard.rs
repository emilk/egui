/// Handles interfacing either with the OS clipboard.
/// If the "clipboard" feature is off it will instead simulate the clipboard locally.

/// Install additional copy pasta implementation for web.
#[cfg(not(feature = "web-sys"))]
type ClipboardContext = copypasta::ClipboardContext;
#[cfg(feature = "web-sys")]
type ClipboardContext = WebSysClipboardContext;
pub struct Clipboard {
    #[cfg(feature = "copypasta")]
    copypasta: Option<ClipboardContext>,

    /// Fallback manual clipboard.
    #[cfg(not(feature = "copypasta"))]
    clipboard: String,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            #[cfg(feature = "copypasta")]
            copypasta: init_copypasta(),

            #[cfg(not(feature = "copypasta"))]
            clipboard: String::default(),
        }
    }
}

impl Clipboard {
    pub fn get(&mut self) -> Option<String> {
        #[cfg(feature = "copypasta")]
        if let Some(clipboard) = &mut self.copypasta {
            use copypasta::ClipboardProvider as _;
            match clipboard.get_contents() {
                Ok(contents) => Some(contents),
                Err(err) => {
                    eprintln!("Paste error: {}", err);
                    None
                }
            }
        } else {
            None
        }

        #[cfg(not(feature = "copypasta"))]
        Some(self.clipboard.clone())
    }

    pub fn set(&mut self, text: String) {
        #[cfg(feature = "copypasta")]
        if let Some(clipboard) = &mut self.copypasta {
            use copypasta::ClipboardProvider as _;
            if let Err(err) = clipboard.set_contents(text) {
                eprintln!("Copy/Cut error: {}", err);
            }
        }

        #[cfg(not(feature = "copypasta"))]
        {
            self.clipboard = text;
        }
    }
}

#[cfg(all(feature = "copypasta", not(feature = "web-sys")))]
fn init_copypasta() -> Option<ClipboardContext> {
    match copypasta::ClipboardContext::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            eprintln!("Failed to initialize clipboard: {}", err);
            None
        }
    }
}
#[cfg(all(feature = "web-sys", feature = "copypasta"))]
fn init_copypasta() -> Option<ClipboardContext> {
    Some({
        let cc = ClipboardContext::new();
        cc.install_event_handler();
        cc
    })
}

#[cfg(feature = "web-sys")]
pub struct WebSysClipboardContext {
    buffer: Arc<RwLock<String>>,
}
#[cfg(feature = "web-sys")]
impl WebSysClipboardContext {
    pub fn new() -> WebSysClipboardContext {
        Self {
            buffer: Arc::new(RwLock::new(String::new())),
        }
    }
    fn install_event_handler(&self) {
        web_sys::console::log_1(
            &"installing copy/paste event handlers"
                .into_js_result()
                .unwrap(),
        );
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let buffer_ref = self.buffer.clone();
        #[cfg(web_sys_unstable_apis)]
        {
            web_sys::console::log_1(&"installing paste event handler".into_js_result().unwrap());
            // paste
            let closure = Closure::wrap(Box::new(move |event: web_sys::ClipboardEvent| {
                if let Some(data) = event.clipboard_data() {
                    if let Ok(text) = data.get_data("text") {
                        let mut lock = buffer_ref.write();
                        let text = String::from(text.replace("\r\n", "\n"));
                        web_sys::console::debug_1(
                            &format!("paste by copypasta on websys : {}", text)
                                .into_js_result()
                                .unwrap(),
                        );
                        *lock = text;
                        event.stop_propagation();
                        event.prevent_default();
                    }
                }
            }) as Box<dyn FnMut(_)>);
            document
                .add_event_listener_with_callback("paste", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }
        let buffer_ref = self.buffer.clone();
        #[cfg(web_sys_unstable_apis)]
        {
            web_sys::console::log_1(&"installing cut event handler".into_js_result().unwrap());
            // cut
            let closure = Closure::wrap(Box::new(move |_: web_sys::ClipboardEvent| {
                Self::set_clipboard_text(&buffer_ref.read())
            }) as Box<dyn FnMut(_)>);
            document
                .add_event_listener_with_callback("cut", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }
        let buffer_ref = self.buffer.clone();
        #[cfg(web_sys_unstable_apis)]
        {
            web_sys::console::log_1(&"installing copy event handler".into_js_result().unwrap());
            // copy
            let closure = Closure::wrap(Box::new(move |_: web_sys::ClipboardEvent| {
                Self::set_clipboard_text(&buffer_ref.read())
            }) as Box<dyn FnMut(_)>);
            document
                .add_event_listener_with_callback("copy", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }
    }
    fn set_clipboard_text(s: &str) {
        let clipboard = web_sys::window().unwrap().navigator().clipboard().unwrap();
        let promise = clipboard.write_text(s);
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        let future = async move { if let Err(err) = future.await {} };
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[cfg(feature = "web-sys")]
use egui::mutex::RwLock;
#[cfg(feature = "web-sys")]
use std::error::Error;
#[cfg(feature = "web-sys")]
use std::sync::Arc;
#[cfg(feature = "web-sys")]
use wasm_bindgen::__rt::IntoJsResult;
#[cfg(feature = "web-sys")]
pub type CPResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync + 'static>>;
#[cfg(feature = "web-sys")]
impl copypasta::ClipboardProvider for WebSysClipboardContext {
    fn get_contents(&mut self) -> CPResult<String> {
        let lock = self.buffer.read();
        Ok(lock.to_string())
    }

    fn set_contents(&mut self, text: String) -> CPResult<()> {
        Ok({
            //Self::set_clipboard_text(&text)
            web_sys::console::log_1(&format!("set_content {}",text).into_js_result().unwrap());
            *self.buffer.write() = text;
        })
    }
}
