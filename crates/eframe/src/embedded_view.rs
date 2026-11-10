//! Embedded views: parts of the UI that the platform draws instead of egui.
//!
//! A web view, a map or a video player can't be painted by egui. The app reserves a rect for
//! it in the UI, and the platform shows the view in that rect.
//!
//! It takes three steps:
//!
//! 1. The app adds an [`egui::EmbeddedView`] widget. It reserves the rect and draws nothing.
//! 2. After each pass, egui reports where every view ended up, in
//!    [`egui::PlatformOutput::embedded_views`].
//! 3. The app moves the platform's view there, from [`crate::App::output_hook`].
//!
//! To create the platform's view in the first place, the app needs the window it goes in.
//! [`EmbeddedViews`] gives access to it.
//!
//! On the web, eframe also routes the pointer input. The canvas is in front of the views that
//! are below egui's surface, so on its own it would take every click. eframe lets the pointer
//! through to a view wherever the view takes the input
//! ([`egui::EmbeddedViewOutput::view_input_areas`]).

/// Access to the app's root window, for the code that creates embedded views in it.
///
/// Get it from [`crate::CreationContext::embedded_views`] or [`crate::Frame::embedded_views`].
///
/// - Natively, it implements [`raw_window_handle::HasWindowHandle`] with the root window's
///   handle, so a view can be added to that window.
/// - On the web, it has the canvas, and the element that views below the canvas go in
///   (`EmbeddedViews::container`).
#[derive(Clone)]
pub struct EmbeddedViews {
    #[cfg(not(target_arch = "wasm32"))]
    raw_window_handle: Result<raw_window_handle::RawWindowHandle, raw_window_handle::HandleError>,

    /// `None` when not attached to a canvas, as in tests.
    #[cfg(target_arch = "wasm32")]
    canvas: Option<web_sys::HtmlCanvasElement>,
}

impl core::fmt::Debug for EmbeddedViews {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EmbeddedViews").finish_non_exhaustive()
    }
}

impl EmbeddedViews {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn new(
        raw_window_handle: Result<
            raw_window_handle::RawWindowHandle,
            raw_window_handle::HandleError,
        >,
    ) -> Self {
        Self { raw_window_handle }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn new(canvas: web_sys::HtmlCanvasElement) -> Self {
        Self {
            canvas: Some(canvas),
        }
    }

    /// Creates embedded views that aren't attached to a window, for tests.
    pub(crate) fn detached() -> Self {
        cfg_select! {
            target_arch = "wasm32" => {
                Self { canvas: None }
            }
            _ => {
                Self::new(Err(raw_window_handle::HandleError::NotSupported))
            }
        }
    }

    /// The canvas egui paints into.
    ///
    /// `None` for a [`crate::Frame`] that isn't attached to a canvas, as in tests.
    #[cfg(target_arch = "wasm32")]
    pub fn canvas(&self) -> Option<&web_sys::HtmlCanvasElement> {
        self.canvas.as_ref()
    }

    /// Returns the container that the elements of views below egui's surface
    /// ([`egui::EmbeddedViewPlacement::Below`]) go in.
    ///
    /// It is a `<div>` right before the canvas in the document, so the canvas paints over it.
    /// eframe keeps its top left corner on the top left corner of the canvas, and it has a size
    /// of zero. Give each view's element `position: absolute`, with its `left` and `top` in CSS
    /// pixels relative to the canvas. The browser then scrolls and clips the view together with
    /// the canvas.
    ///
    /// The `<div>` forms its own stacking context, so the `z-index` of a view only orders it
    /// among the other views in it. Give each view's element a `z-index` of
    /// [`egui::EmbeddedViewOutput::z_index`], and `pointer-events: auto`, because the `<div>` has
    /// `pointer-events: none`.
    ///
    /// When a view below egui's surface is first shown, eframe gives a canvas that has
    /// `position: static` an inline `position: relative`, and leaves it that way. A positioned
    /// element is painted in front of a static one, so this keeps the canvas in front of the
    /// `<div>`.
    ///
    /// # Errors
    /// If the canvas wasn't in the document when eframe started, or was moved to another parent
    /// element since. eframe only makes the `<div>` at startup, next to the canvas. Also if
    /// there is no canvas, see [`Self::canvas`].
    #[cfg(target_arch = "wasm32")]
    pub fn container(&self) -> Result<web_sys::HtmlElement, wasm_bindgen::JsValue> {
        let canvas = self.canvas().ok_or_else(|| {
            wasm_bindgen::JsValue::from_str("These embedded views aren't attached to a canvas")
        })?;
        crate::web::embedded_views::container_of(canvas)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[expect(unsafe_code)]
impl raw_window_handle::HasWindowHandle for EmbeddedViews {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        // SAFETY: eframe keeps the root window alive until after it has dropped the app, which
        // holds any clones of `self`.
        unsafe {
            Ok(raw_window_handle::WindowHandle::borrow_raw(
                self.raw_window_handle.clone()?,
            ))
        }
    }
}
