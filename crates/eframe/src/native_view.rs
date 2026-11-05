//! Native views: rects of the UI that the platform draws instead of egui, such as a web view, a
//! map or a video player.
//!
//! The app reserves the rect with the [`egui::NativeView`] widget, and places the view from
//! [`crate::App::output_hook`] with [`egui::PlatformOutput::native_views`]. [`NativeViews`]
//! gives the code that shows a view what it needs from the window.
//!
//! eframe also handles what all the views of a window share. On the web, that is the pointer
//! input: the canvas is in front of the views below egui's surface, so eframe lets the pointer
//! through the canvas wherever a view takes the input
//! ([`egui::NativeViewOutput::view_input_areas`]).

/// What the code that shows native views needs from the app's root window.
///
/// Get it from [`crate::CreationContext::native_views`] or [`crate::Frame::native_views`].
///
/// Natively, it implements [`raw_window_handle::HasWindowHandle`] with the root window's handle,
/// so that the views can be added to that window. On the web, it gives access to the canvas and
/// to the element that the views below the canvas go in (`NativeViews::layer`, web only).
#[derive(Clone)]
pub struct NativeViews {
    #[cfg(not(target_arch = "wasm32"))]
    raw_window_handle: Result<raw_window_handle::RawWindowHandle, raw_window_handle::HandleError>,

    #[cfg(target_arch = "wasm32")]
    canvas: web_sys::HtmlCanvasElement,
}

impl core::fmt::Debug for NativeViews {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NativeViews").finish_non_exhaustive()
    }
}

impl NativeViews {
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
        Self { canvas }
    }

    /// Creates native views that aren't attached to a window, for tests.
    pub(crate) fn detached() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::new(Err(raw_window_handle::HandleError::NotSupported))
        }
        #[cfg(target_arch = "wasm32")]
        {
            let canvas = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.create_element("canvas").ok())
                .and_then(|canvas| wasm_bindgen::JsCast::dyn_into(canvas).ok())
                .expect("A document to make a canvas in");
            Self::new(canvas)
        }
    }

    /// The canvas egui paints into.
    #[cfg(target_arch = "wasm32")]
    pub fn canvas(&self) -> &web_sys::HtmlCanvasElement {
        &self.canvas
    }

    /// Returns the element to add the elements of views below egui's surface
    /// ([`egui::NativeViewPlacement::Below`]) to.
    ///
    /// It is a `<div>` right before the canvas in the document, so the canvas paints over it.
    /// The `<div>` has `position: fixed` at the top left of the browser viewport and a size of
    /// zero. It forms its own stacking context, so the `z-index` of a view only orders it among
    /// the other views in it. Give each view's element `position: fixed`, a `z-index` of
    /// [`NativeViewOutput::z_index`] plus one, and `pointer-events: auto`, because the `<div>`
    /// has `pointer-events: none`.
    ///
    /// If the canvas has `position: static`, the first call sets it to `position: relative`.
    /// A positioned element is painted in front of a static one, so this keeps the canvas in
    /// front of the `<div>`.
    ///
    /// # Errors
    /// If the canvas wasn't in the document when eframe started.
    #[cfg(target_arch = "wasm32")]
    pub fn layer(&self) -> Result<web_sys::HtmlElement, wasm_bindgen::JsValue> {
        crate::web::native_views::layer_of(&self.canvas)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[expect(unsafe_code)]
impl raw_window_handle::HasWindowHandle for NativeViews {
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

/// Splits the union of `rects` into rects that don't overlap.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn disjoint_union(rects: impl IntoIterator<Item = egui::Rect>) -> Vec<egui::Rect> {
    let mut disjoint: Vec<egui::Rect> = Vec::new();
    for rect in rects {
        let mut parts = vec![rect];
        for &other in &disjoint {
            parts = parts
                .into_iter()
                .flat_map(|part| subtract(part, other))
                .collect();
        }
        disjoint.extend(parts);
    }
    disjoint
}

/// Returns `rect` without `other`, as up to four rects that don't overlap.
#[cfg(any(target_arch = "wasm32", test))]
fn subtract(rect: egui::Rect, other: egui::Rect) -> Vec<egui::Rect> {
    let cut = rect.intersect(other);
    if !cut.is_positive() {
        return vec![rect];
    }
    // The strips above and below the cut span the full width of `rect`, and the strips to its
    // left and right only the height of the cut.
    [
        egui::Rect::from_x_y_ranges(rect.x_range(), rect.min.y..=cut.min.y),
        egui::Rect::from_x_y_ranges(rect.x_range(), cut.max.y..=rect.max.y),
        egui::Rect::from_x_y_ranges(rect.min.x..=cut.min.x, cut.y_range()),
        egui::Rect::from_x_y_ranges(cut.max.x..=rect.max.x, cut.y_range()),
    ]
    .into_iter()
    .filter(egui::Rect::is_positive)
    .collect()
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use egui::{Rect, pos2, vec2};

    use super::disjoint_union;

    #[test]
    fn a_disjoint_union_covers_overlaps_once() {
        let rects = [
            Rect::from_min_size(pos2(0.0, 0.0), vec2(60.0, 60.0)),
            Rect::from_min_size(pos2(40.0, 40.0), vec2(60.0, 60.0)),
            Rect::from_min_size(pos2(50.0, 50.0), vec2(10.0, 10.0)),
        ];
        let parts = disjoint_union(rects);
        let area: f32 = parts.iter().map(Rect::area).sum();
        assert_eq!(area, 2.0 * 60.0 * 60.0 - 20.0 * 20.0, "{parts:?}");
        for (i, a) in parts.iter().enumerate() {
            for b in &parts[i + 1..] {
                assert!(!a.intersect(*b).is_positive(), "{a:?} overlaps {b:?}");
            }
        }
    }
}
