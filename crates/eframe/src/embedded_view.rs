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

    #[cfg(target_arch = "wasm32")]
    canvas: web_sys::HtmlCanvasElement,
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
        Self { canvas }
    }

    /// Creates embedded views that aren't attached to a window, for tests.
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

    /// Returns the container that the elements of views below egui's surface
    /// ([`egui::EmbeddedViewPlacement::Below`]) go in.
    ///
    /// It is a `<div>` right before the canvas in the document, so the canvas paints over it.
    /// The `<div>` has `position: fixed` at the top left of the browser viewport and a size of
    /// zero. It forms its own stacking context, so the `z-index` of a view only orders it among
    /// the other views in it. Give each view's element `position: fixed`, a `z-index` of
    /// [`EmbeddedViewOutput::z_index`], and `pointer-events: auto`, because the `<div>` has
    /// `pointer-events: none`.
    ///
    /// When a view below egui's surface is first shown, eframe gives a canvas that has
    /// `position: static` an inline `position: relative`, and leaves it that way. A positioned
    /// element is painted in front of a static one, so this keeps the canvas in front of the
    /// `<div>`.
    ///
    /// # Errors
    /// If the canvas wasn't in the document when eframe started, or was moved to another parent
    /// element since. eframe only makes the `<div>` at startup, next to the canvas.
    #[cfg(target_arch = "wasm32")]
    pub fn container(&self) -> Result<web_sys::HtmlElement, wasm_bindgen::JsValue> {
        crate::web::embedded_views::container_of(&self.canvas)
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

/// Returns the parts of `rect` outside all of `others`, as rects that don't overlap.
///
/// It cuts `rect` into horizontal bands at the top and bottom edges of `others`, and joins the
/// parts of adjacent bands that span the same x-range, so the result is a few tall rects rather
/// than many small ones.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn subtract_all(rect: egui::Rect, others: &[egui::Rect]) -> Vec<egui::Rect> {
    let others: Vec<egui::Rect> = others
        .iter()
        .map(|other| other.intersect(rect))
        .filter(egui::Rect::is_positive)
        .collect();

    let mut ys = vec![rect.min.y, rect.max.y];
    ys.extend(others.iter().flat_map(|other| [other.min.y, other.max.y]));
    ys.sort_by(f32::total_cmp);
    ys.dedup();

    let mut parts = Vec::new();
    // The parts of the band above, which a part of this band with the same x-range extends.
    let mut open: Vec<egui::Rect> = Vec::new();
    for band in ys.windows(2) {
        let (top, bottom) = (band[0], band[1]);
        // Every one of `others` that reaches into the band spans its whole height.
        let mut cuts: Vec<egui::Rangef> = others
            .iter()
            .filter(|other| other.min.y < bottom && top < other.max.y)
            .map(|other| other.x_range())
            .collect();
        cuts.sort_by(|a, b| a.min.total_cmp(&b.min));

        let mut in_band = Vec::new();
        let mut x = rect.min.x;
        for cut in cuts {
            if x < cut.min {
                in_band.push(egui::Rangef::new(x, cut.min));
            }
            x = x.max(cut.max);
        }
        if x < rect.max.x {
            in_band.push(egui::Rangef::new(x, rect.max.x));
        }

        let mut still_open = Vec::new();
        for x_range in in_band {
            let above = open.iter().position(|part| part.x_range() == x_range);
            let part = if let Some(above) = above {
                let mut part = open.swap_remove(above);
                part.max.y = bottom;
                part
            } else {
                egui::Rect::from_x_y_ranges(x_range, top..=bottom)
            };
            still_open.push(part);
        }
        parts.append(&mut open);
        open = still_open;
    }
    parts.append(&mut open);
    parts
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use egui::{Rect, pos2, vec2};

    use super::subtract_all;

    fn assert_disjoint(parts: &[Rect]) {
        for (i, a) in parts.iter().enumerate() {
            for b in &parts[i + 1..] {
                assert!(!a.intersect(*b).is_positive(), "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn subtracting_a_rect_in_the_middle_leaves_a_frame_around_it() {
        let canvas = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0));
        let view = Rect::from_min_size(pos2(10.0, 20.0), vec2(50.0, 30.0));
        let parts = subtract_all(canvas, &[view]);
        assert_eq!(parts.len(), 4, "{parts:?}");
        assert_disjoint(&parts);
        let area: f32 = parts.iter().map(Rect::area).sum();
        assert_eq!(area, canvas.area() - view.area(), "{parts:?}");
        for part in &parts {
            assert!(
                !part.intersect(view).is_positive(),
                "{part:?} covers the view"
            );
        }
    }

    #[test]
    fn subtracting_overlapping_rects_removes_the_overlap_once() {
        let canvas = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0));
        let views = [
            Rect::from_min_size(pos2(0.0, 0.0), vec2(60.0, 60.0)),
            Rect::from_min_size(pos2(40.0, 40.0), vec2(60.0, 60.0)),
        ];
        let parts = subtract_all(canvas, &views);
        assert_disjoint(&parts);
        let area: f32 = parts.iter().map(Rect::area).sum();
        let covered = 2.0 * 60.0 * 60.0 - 20.0 * 20.0;
        assert_eq!(area, canvas.area() - covered, "{parts:?}");
    }

    #[test]
    fn a_window_over_a_page_leaves_few_rects() {
        // A page with margins around it, and a window over it whose own page is cut out too:
        // what is left is the margins and the window's frame.
        let canvas = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0));
        let page = Rect::from_min_max(pos2(8.0, 33.0), pos2(992.0, 640.0));
        let window = Rect::from_min_max(pos2(300.0, 200.0), pos2(780.0, 560.0));
        let window_page = Rect::from_min_max(pos2(306.0, 250.0), pos2(774.0, 554.0));
        // The page takes the input around the window, as four rects.
        let views = [
            Rect::from_min_max(page.min, pos2(page.max.x, window.min.y)),
            Rect::from_min_max(pos2(page.min.x, window.max.y), page.max),
            Rect::from_min_max(
                pos2(page.min.x, window.min.y),
                pos2(window.min.x, window.max.y),
            ),
            Rect::from_min_max(
                pos2(window.max.x, window.min.y),
                pos2(page.max.x, window.max.y),
            ),
            window_page,
        ];
        let parts = subtract_all(canvas, &views);
        assert_disjoint(&parts);
        let area: f32 = parts.iter().map(Rect::area).sum();
        let frame = window.area() - window_page.area();
        assert_eq!(area, canvas.area() - page.area() + frame, "{parts:?}");
        assert_eq!(
            parts.len(),
            8,
            "four margins and the four sides of the window: {parts:?}"
        );
    }

    #[test]
    fn subtracting_nothing_or_everything() {
        let canvas = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0));
        assert_eq!(subtract_all(canvas, &[]), vec![canvas]);
        assert_eq!(subtract_all(canvas, &[canvas.expand(5.0)]), Vec::new());
    }
}
