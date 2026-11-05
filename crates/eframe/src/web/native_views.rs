//! What the native views of a canvas share: the layer they go in, and the input.
//!
//! The canvas is above the views below egui's surface ([`egui::NativeViewPlacement::Below`]), so
//! it would get all of the pointer. A view's input has to get through it, but an element can't
//! take the pointer in only parts of itself without also being drawn in only those parts.
//!
//! So while such a view is shown, the canvas only paints, with `pointer-events: none`, and an
//! invisible shield over it takes egui's pointer input. eframe listens for the pointer on the
//! shield like on the canvas. The shield is clipped to leave out where the views take the input
//! ([`egui::NativeViewOutput::view_input_areas`]), so the pointer there goes to the views. The
//! canvas is drawn whole, so egui paints over the views up to the last pixel, shadows included.
//!
//! The keyboard focus stays on the canvas: the shield isn't focusable, and a press on it
//! doesn't move the focus.

use core::cell::RefCell;
use core::fmt::Write as _;

use egui::{Context, NativeViewOutput, NativeViewPlacement, Rect};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use web_sys::{HtmlCanvasElement, HtmlElement};

pub(crate) struct WebNativeViews {
    canvas: HtmlCanvasElement,

    /// `None` if the canvas wasn't in a document when eframe started.
    layer: Option<Layer>,

    state: RefCell<State>,
}

#[derive(Default)]
struct State {
    /// Whether the canvas was made positioned, to stay over the views.
    positioned: bool,

    /// The style of the shield last set, to skip setting it again. `None` while it's hidden.
    shield_style: Option<String>,
}

/// What goes with the canvas, just before it.
struct Layer {
    /// Holds the views, and the shield. It comes before the canvas, so the canvas is painted
    /// over it.
    element: HtmlElement,

    /// Takes egui's pointer input while views are shown. See the module docs.
    shield: HtmlElement,
}

impl Drop for Layer {
    fn drop(&mut self) {
        self.element.remove();
    }
}

impl WebNativeViews {
    pub fn new(canvas: HtmlCanvasElement) -> Self {
        // Made up front, so eframe can listen on the shield from the start.
        let layer = Layer::new(&canvas)
            .inspect_err(|err| log::debug!("No native views below the canvas: {err:?}"))
            .ok();
        Self {
            canvas,
            layer,
            state: RefCell::default(),
        }
    }

    pub fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }

    /// The element that takes egui's pointer input over the views. See the module docs.
    pub fn shield(&self) -> Option<&HtmlElement> {
        self.layer.as_ref().map(|layer| &layer.shield)
    }

    pub fn layer(&self) -> Result<HtmlElement, JsValue> {
        let layer = self.layer.as_ref().ok_or_else(|| {
            JsValue::from_str("The canvas wasn't in the document when eframe started")
        })?;
        let mut state = self.state.borrow_mut();
        if !state.positioned {
            // A positioned element is painted over one that isn't, so the canvas has to be
            // positioned too, to stay over the views.
            let is_static = web_sys::window()
                .and_then(|window| window.get_computed_style(&self.canvas).ok().flatten())
                .and_then(|style| style.get_property_value("position").ok())
                .is_some_and(|position| position == "static");
            if is_static {
                self.canvas.style().set_property("position", "relative")?;
            }
            state.positioned = true;
        }
        Ok(layer.element.clone())
    }

    /// Have the shield take egui's pointer input while views below the canvas are shown, and
    /// let the pointer through to where they take the input this pass.
    pub fn route_input(&self, ctx: &Context, outputs: &[NativeViewOutput]) {
        let Some(layer) = &self.layer else {
            return;
        };
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return;
        };
        // A view above the canvas gets the pointer anyway.
        let mut below = outputs
            .iter()
            .filter(|output| output.side == NativeViewPlacement::Below)
            .peekable();
        let shield_style = below.peek().is_some().then(|| {
            // A press that started in egui keeps the pointer until it is released: eframe
            // follows a drag with the `mousemove` events of the document, which don't come
            // while the pointer is over a view in an `<iframe>`.
            let egui_press = ctx.input(|input| input.pointer.any_down());
            let view_areas: Vec<Rect> = if egui_press {
                Vec::new()
            } else {
                let to_css = to_css_pixels(ctx, &self.canvas);
                below
                    .flat_map(NativeViewOutput::view_input_areas)
                    .map(to_css)
                    .collect()
            };
            shield_style(&self.canvas, &view_areas)
        });
        if shield_style == state.shield_style {
            return;
        }
        let canvas_style = self.canvas.style();
        if let Some(css) = &shield_style {
            canvas_style.set_property("pointer-events", "none").ok();
            set_style(&layer.shield, css);
        } else {
            canvas_style.remove_property("pointer-events").ok();
            set_style(&layer.shield, "display: none;");
        }
        state.shield_style = shield_style;
    }
}

/// Converts ui points to CSS pixels from the top left of `canvas`.
///
/// Those are usually window points (ui points times the zoom factor), but not while the size of
/// the canvas that egui knows lags behind its size on the page.
fn to_css_pixels(
    ctx: &Context,
    canvas: &HtmlCanvasElement,
) -> impl Fn(Rect) -> Rect + Copy + use<> {
    let zoom = ctx.zoom_factor();
    let css_width = canvas.get_bounding_client_rect().width() as f32;
    let lag = css_width / (ctx.viewport_rect().width() * zoom);
    let scale = if lag.is_finite() && 0.0 < lag {
        zoom * lag
    } else {
        zoom
    };
    move |rect| rect * scale
}

/// The style of the shield: over `canvas`, without `view_areas`.
fn shield_style(canvas: &HtmlCanvasElement, view_areas: &[Rect]) -> String {
    let rect = canvas.get_bounding_client_rect();
    // Over the views, which have a `z-index` of at least 1.
    let mut css = format!(
        "position: fixed; left: {}px; top: {}px; width: {}px; height: {}px; \
         z-index: 2147483647; pointer-events: auto;",
        rect.left(),
        rect.top(),
        rect.width(),
        rect.height(),
    );
    if !view_areas.is_empty() {
        // With `evenodd`, the rects are cut out. Those of one view don't overlap, and those of
        // different ones only if the views do.
        let mut path = format!("M0 0H{}V{}H0Z", rect.width(), rect.height());
        for Rect { min, max } in view_areas {
            write!(path, "M{} {}H{}V{}H{}Z", min.x, min.y, max.x, max.y, min.x).ok();
        }
        write!(css, " clip-path: path(evenodd, '{path}');").ok();
    }
    css
}

impl Layer {
    fn new(canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        let document = canvas
            .owner_document()
            .ok_or_else(|| JsValue::from_str("The canvas has no document"))?;
        let parent = canvas
            .parent_node()
            .ok_or_else(|| JsValue::from_str("The canvas isn't in the document"))?;

        let element: HtmlElement = document.create_element("div")?.unchecked_into();
        // `position: fixed` makes it a stacking context, so the `z-index` of the views and the
        // shield only orders them among each other.
        set_style(
            &element,
            "position: fixed; left: 0; top: 0; width: 0; height: 0; pointer-events: none;",
        );
        let shield: HtmlElement = document.create_element("div")?.unchecked_into();
        set_style(&shield, "display: none;");
        element.append_child(&shield)?;
        parent.insert_before(&element, Some(canvas))?;

        Ok(Self { element, shield })
    }
}

fn set_style(element: &web_sys::Element, css: &str) {
    element.set_attribute("style", css).ok();
}
