//! Native views: rects of the UI that the platform draws instead of egui, such as a web view, a
//! map or a video player.
//!
//! The app reserves the rect with the [`egui::NativeView`] widget. The code that shows the view
//! registers a callback with [`NativeViews::on_place`], and eframe calls it with
//! [`egui::PlatformOutput::native_views`] once per frame, before the frame is painted.
//!
//! eframe also handles what all the views of a window share. On the web, that is the pointer
//! input: the canvas is in front of the views below egui's surface, so eframe lets the pointer
//! through the canvas wherever a view takes the input
//! ([`egui::NativeViewOutput::view_input_areas`]).

use core::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use egui::{Context, Id, NativeViewOutput};

/// A callback registered with [`NativeViews::on_place`].
type PlaceFn = dyn FnMut(&Context, &[NativeViewOutput]);

/// Reports to the code that shows native views where egui placed them in the app's root window.
///
/// Get it from [`crate::CreationContext::native_views`] or [`crate::Frame::native_views`], and
/// register a callback with [`Self::on_place`] or [`Self::on_place_view`].
///
/// Natively, it implements [`raw_window_handle::HasWindowHandle`] with the root window's handle,
/// so that the views can be added to that window. On the web, it gives access to the canvas and
/// to the element that the views below the canvas go in (`NativeViews::layer`, web only).
///
/// Cloning it is cheap, and all clones share the same callbacks.
#[derive(Clone)]
pub struct NativeViews(Rc<Inner>);

struct Inner {
    /// The callbacks registered with [`NativeViews::on_place`], with the key that their
    /// [`PlaceHook`] removes them by.
    hooks: RefCell<Vec<(u64, Rc<RefCell<PlaceFn>>)>>,
    next_key: Cell<u64>,

    #[cfg(not(target_arch = "wasm32"))]
    raw_window_handle: Result<raw_window_handle::RawWindowHandle, raw_window_handle::HandleError>,

    #[cfg(target_arch = "wasm32")]
    web: crate::web::native_views::WebNativeViews,
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
        Self(Rc::new(Inner {
            hooks: RefCell::default(),
            next_key: Cell::new(0),
            raw_window_handle,
        }))
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn new(canvas: web_sys::HtmlCanvasElement) -> Self {
        Self(Rc::new(Inner {
            hooks: RefCell::default(),
            next_key: Cell::new(0),
            web: crate::web::native_views::WebNativeViews::new(canvas),
        }))
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

    /// Registers `place` to be called once per frame of the root viewport, before the frame is
    /// painted, with the outputs of the [`egui::NativeView`]s that are on screen.
    ///
    /// The outputs are those of the frame's last pass. A view that is missing from them should
    /// hide.
    ///
    /// This suits code that shows many views, like a bridge to platform text fields. For a
    /// single view, see [`Self::on_place_view`].
    ///
    /// eframe calls `place` until the returned [`PlaceHook`] is dropped.
    pub fn on_place(
        &self,
        place: impl FnMut(&Context, &[NativeViewOutput]) + 'static,
    ) -> PlaceHook {
        let key = self.0.next_key.get();
        self.0.next_key.set(key + 1);
        let place: Rc<RefCell<PlaceFn>> = Rc::new(RefCell::new(place));
        self.0.hooks.borrow_mut().push((key, place));
        PlaceHook {
            inner: Rc::downgrade(&self.0),
            key,
        }
    }

    /// Registers `place` to be called once per frame of the root viewport, before the frame is
    /// painted, with the output of the [`egui::NativeView`] `id`.
    ///
    /// The output is `None` when the view isn't on screen, and the view should then hide.
    ///
    /// eframe calls `place` until the returned [`PlaceHook`] is dropped.
    pub fn on_place_view(
        &self,
        id: Id,
        mut place: impl FnMut(&Context, Option<&NativeViewOutput>) + 'static,
    ) -> PlaceHook {
        self.on_place(move |ctx, outputs| {
            place(ctx, outputs.iter().find(|output| output.id == id));
        })
    }

    /// The canvas egui paints into.
    #[cfg(target_arch = "wasm32")]
    pub fn canvas(&self) -> &web_sys::HtmlCanvasElement {
        self.0.web.canvas()
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
        self.0.web.layer()
    }

    /// The invisible element that takes egui's pointer input while views are shown below the
    /// canvas. `None` if the canvas wasn't in the document when eframe started.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn input_element(&self) -> Option<&web_sys::HtmlElement> {
        self.0.web.input_element()
    }

    /// Routes the pointer input on the web and calls the registered callbacks with the
    /// `outputs` of a frame of the root viewport.
    #[cfg_attr(
        not(any(feature = "glow", feature = "wgpu_no_default_features")),
        expect(
            dead_code,
            reason = "Only the runners call this, and they exist only with a renderer feature"
        )
    )]
    pub(crate) fn place(&self, ctx: &Context, outputs: &[NativeViewOutput]) {
        #[cfg(target_arch = "wasm32")]
        self.0.web.route_input(ctx, outputs);

        // Clone the callbacks out, so that a callback can register and drop hooks while it runs.
        let hooks = self
            .0
            .hooks
            .borrow()
            .iter()
            .map(|(_, hook)| Rc::clone(hook))
            .collect::<Vec<_>>();
        for hook in hooks {
            // If a callback runs a frame of its own that calls `place` again, the callback is
            // still borrowed, and the nested call skips it.
            if let Ok(mut place) = hook.try_borrow_mut() {
                place(ctx, outputs);
            }
        }
    }
}

/// Keeps a callback of [`NativeViews::on_place`] or [`NativeViews::on_place_view`]
/// registered. Dropping it unregisters the callback.
#[must_use = "The hook stops when this is dropped"]
pub struct PlaceHook {
    inner: Weak<Inner>,
    key: u64,
}

impl core::fmt::Debug for PlaceHook {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PlaceHook").finish_non_exhaustive()
    }
}

impl Drop for PlaceHook {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.upgrade() {
            inner.hooks.borrow_mut().retain(|(key, _)| *key != self.key);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[expect(unsafe_code)]
impl raw_window_handle::HasWindowHandle for NativeViews {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        // SAFETY: eframe keeps the root window alive until after it has dropped the app, which
        // holds the clones of `self`.
        unsafe {
            Ok(raw_window_handle::WindowHandle::borrow_raw(
                self.0.raw_window_handle.clone()?,
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
    use core::cell::RefCell;
    use std::rc::Rc;

    use egui::{Context, Id, NativeViewOutput, NativeViewPlacement, Rect, pos2, vec2};

    use super::{NativeViews, disjoint_union};

    fn output(id: Id) -> NativeViewOutput {
        NativeViewOutput {
            id,
            side: NativeViewPlacement::Below,
            rect: Rect::ZERO,
            visible: Rect::ZERO,
            egui_paint_areas: Vec::new(),
            egui_input_areas: Vec::new(),
            z_index: 0,
        }
    }

    #[test]
    fn hooks_get_their_views_until_dropped() {
        let ctx = Context::default();
        let views = NativeViews::detached();
        let (shown, hidden) = (Id::unique("shown"), Id::unique("hidden"));

        let all = Rc::new(RefCell::new(Vec::new()));
        let all_hook = views.on_place({
            let all = Rc::clone(&all);
            move |_, outputs| all.borrow_mut().push(outputs.len())
        });
        let one = Rc::new(RefCell::new(Vec::new()));
        let one_hook = views.on_place_view(hidden, {
            let one = Rc::clone(&one);
            move |_, output| one.borrow_mut().push(output.map(|output| output.id))
        });

        views.place(&ctx, &[output(shown)]);
        views.place(&ctx, &[output(shown), output(hidden)]);
        drop(one_hook);
        views.place(&ctx, &[output(hidden)]);
        drop(all_hook);
        views.place(&ctx, &[]);

        assert_eq!(*all.borrow(), vec![1, 2, 1]);
        assert_eq!(*one.borrow(), vec![None, Some(hidden)]);
    }

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
