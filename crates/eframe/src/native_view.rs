//! Native views: UI that the platform draws, not egui, like a web view, a map or a video.
//!
//! egui leaves a rect to a native view with [`egui::NativeView`]. Whatever shows the view
//! hooks into [`NativeViews`], and eframe hands it where egui left the views after every pass,
//! before the pass is painted.
//!
//! eframe also does what all views of a window share. On the web, that's the input: the canvas
//! is above the views below egui's surface, so eframe lets the pointer through it to where a
//! view takes the input ([`egui::NativeViewOutput::view_input_areas`]).

use core::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use egui::{Context, Id, NativeViewOutput};

/// What [`NativeViews::on_place`] calls.
type PlaceFn = dyn FnMut(&Context, &[NativeViewOutput]);

/// The native views of the app's window, which eframe places. Get it from
/// [`crate::CreationContext::native_views`] or [`crate::Frame::native_views`].
///
/// Natively, it's the root window's handle, for the views to go in. On the web, it holds what
/// the views of the canvas share.
///
/// It's cheap to clone.
#[derive(Clone)]
pub struct NativeViews(Rc<Inner>);

struct Inner {
    /// The hooks of [`NativeViews::on_place`], by key.
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

    /// Native views without a window, for tests.
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

    /// Call `place` after every pass of the root viewport, before the pass is painted, with
    /// where the [`egui::NativeView`]s on screen in it go. A view that isn't in there should
    /// hide.
    ///
    /// For something that shows many views, like a bridge to platform text fields. For one view,
    /// see [`Self::on_place_view`].
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

    /// Call `place` after every pass of the root viewport, before the pass is painted, with
    /// where the [`egui::NativeView`] `id` goes: `None` when it isn't on screen, and should hide.
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

    /// The element to put the elements of views below egui's surface in
    /// ([`egui::NativeViewPlacement::Below`]): a `<div>` just before the canvas, so the canvas paints
    /// over them. It's `position: fixed` at the top left of the page, with no
    /// size, so give each view `position: fixed`, and a `z-index` from
    /// [`NativeViewOutput::z_index`] (plus one).
    ///
    /// # Errors
    /// If the canvas wasn't in the document when eframe started.
    #[cfg(target_arch = "wasm32")]
    pub fn layer(&self) -> Result<web_sys::HtmlElement, wasm_bindgen::JsValue> {
        self.0.web.layer()
    }

    /// The element that takes egui's pointer input over the views below the canvas, if the
    /// canvas was in the document when eframe started.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn shield(&self) -> Option<&web_sys::HtmlElement> {
        self.0.web.shield()
    }

    /// Place the views after a pass of the root viewport, with its `outputs`.
    #[cfg_attr(
        not(any(feature = "glow", feature = "wgpu_no_default_features")),
        expect(
            dead_code,
            reason = "Only the runners place views, and they need a renderer"
        )
    )]
    pub(crate) fn place(&self, ctx: &Context, outputs: &[NativeViewOutput]) {
        #[cfg(target_arch = "wasm32")]
        self.0.web.route_input(ctx, outputs);

        // Not borrowed while the hooks run, so they can add and drop hooks.
        let hooks = self
            .0
            .hooks
            .borrow()
            .iter()
            .map(|(_, hook)| Rc::clone(hook))
            .collect::<Vec<_>>();
        for hook in hooks {
            // A hook that places the views from inside itself, through a pass of its own, is
            // skipped there.
            if let Ok(mut place) = hook.try_borrow_mut() {
                place(ctx, outputs);
            }
        }
    }
}

/// Keeps a hook of [`NativeViews::on_place`] called. Drop it to stop.
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
        // Safety: the window outlives the app, and so `self`.
        unsafe {
            Ok(raw_window_handle::WindowHandle::borrow_raw(
                self.0.raw_window_handle.clone()?,
            ))
        }
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use core::cell::RefCell;
    use std::rc::Rc;

    use egui::{Context, Id, NativeViewOutput, NativeViewPlacement, Rect};

    use super::NativeViews;

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
}
