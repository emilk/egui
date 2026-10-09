//! A canvas that eframe can render to.
//!
//! Two flavors exist:
//! * `Html`: the classic DOM canvas, with focus, CSS geometry and text input.
//! * `Offscreen`: an `OffscreenCanvas` owned by a web worker: it has pixels and
//!   a rendering context, but no DOM. Every DOM-only operation must go through
//!   [`WebCanvas::as_html`], which returns `None` in worker mode.

#[cfg(feature = "glow")]
use wasm_bindgen::{JsCast as _, JsValue};

/// The canvas an eframe web app renders to.
#[derive(Clone, Debug)]
pub(crate) enum WebCanvas {
    /// A `<canvas>` element in the document.
    Html(web_sys::HtmlCanvasElement),

    /// An `OffscreenCanvas` transferred into a worker.
    Offscreen(web_sys::OffscreenCanvas),
}

impl WebCanvas {
    pub fn as_html(&self) -> Option<&web_sys::HtmlCanvasElement> {
        match self {
            Self::Html(canvas) => Some(canvas),
            Self::Offscreen(_) => None,
        }
    }

    /// The DOM canvas. Panics in worker mode: only call from DOM-only code paths.
    ///
    /// # Panics
    /// Panics if this is an `OffscreenCanvas`.
    pub fn expect_html(&self) -> &web_sys::HtmlCanvasElement {
        self.as_html().expect(
            "tried to use DOM-only canvas functionality in a worker (OffscreenCanvas has no DOM)",
        )
    }

    /// The offscreen canvas, if this is one.
    pub fn as_offscreen(&self) -> Option<&web_sys::OffscreenCanvas> {
        match self {
            Self::Html(_) => None,
            Self::Offscreen(canvas) => Some(canvas),
        }
    }

    /// Canvas width in physical pixels.
    pub fn width(&self) -> u32 {
        match self {
            Self::Html(canvas) => canvas.width(),
            Self::Offscreen(canvas) => canvas.width(),
        }
    }

    /// Canvas height in physical pixels.
    pub fn height(&self) -> u32 {
        match self {
            Self::Html(canvas) => canvas.height(),
            Self::Offscreen(canvas) => canvas.height(),
        }
    }

    /// Resize the canvas, in physical pixels.
    pub fn set_size(&self, width: u32, height: u32) {
        match self {
            Self::Html(canvas) => {
                canvas.set_width(width);
                canvas.set_height(height);
            }
            Self::Offscreen(canvas) => {
                canvas.set_width(width);
                canvas.set_height(height);
            }
        }
    }

    /// A stable identifier, for logging only.
    pub fn id(&self) -> String {
        match self {
            Self::Html(canvas) => format!("{canvas:?}"),
            Self::Offscreen(canvas) => format!("{canvas:?}"),
        }
    }

    /// Get a rendering context (e.g. `"webgl2"`) from either canvas flavor.
    #[cfg(feature = "glow")]
    pub fn get_context(&self, context_id: &str) -> Result<Option<JsValue>, JsValue> {
        let context = match self {
            Self::Html(canvas) => canvas.get_context(context_id)?,
            Self::Offscreen(canvas) => canvas.get_context(context_id)?,
        };
        Ok(context.map(|context| context.unchecked_into::<JsValue>()))
    }
}
