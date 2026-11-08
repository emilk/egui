//! The runner's state for the views below the canvas: the element they go in, and the routing
//! of the pointer input.
//!
//! The canvas is in front of the views below egui's surface
//! ([`egui::NativeViewPlacement::Below`]), so the browser's hit-testing would send all of the
//! pointer input to the canvas. The views need the input where egui doesn't take it. CSS can't
//! limit the hit-testing of an element to part of its area without also clipping its drawing:
//! `clip-path` clips both.
//!
//! So while such a view is shown, the canvas only draws and has `pointer-events: none`. Instead, an
//! invisible input element in front of the canvas and the views takes egui's pointer input, and
//! eframe listens for pointer events on the input element like on the canvas. The input element's
//! `clip-path` cuts out the areas where the views take the input
//! ([`egui::NativeViewOutput::view_input_areas`]), so hit-testing there finds the views. The canvas
//! isn't clipped, so egui paints over the views down to the last pixel, shadows included.
//!
//! The keyboard focus stays on the canvas: the input element isn't focusable, and eframe cancels
//! the default action of a `mousedown` on the input element, which would move the focus.

use core::fmt::Write as _;

use egui::{Context, NativeViewOutput, NativeViewPlacement, Rect};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use web_sys::{HtmlCanvasElement, HtmlElement};

/// The runner's state for the views below the canvas. See the module docs.
pub(crate) struct WebNativeViews {
    canvas: HtmlCanvasElement,

    /// `None` if the canvas wasn't in a document when eframe started.
    layer: Option<Layer>,

    /// The `style` attribute last set on the input element, so that an unchanged style isn't set
    /// again. `None` while the input element is hidden.
    input_style: Option<String>,
}

/// The attribute that marks the layer element, so that [`layer_of`] can find it.
const LAYER_ATTRIBUTE: &str = "data-egui-native-view-layer";

/// The element that holds the views below the canvas, inserted right before the canvas.
struct Layer {
    /// The `position: fixed` container of the views and the input element. It precedes the canvas
    /// in the document, so a positioned canvas paints in front of it.
    element: HtmlElement,

    /// Takes egui's pointer input while views below the canvas are shown. See the module docs.
    input_element: HtmlElement,
}

impl Drop for Layer {
    fn drop(&mut self) {
        self.element.remove();
    }
}

impl WebNativeViews {
    pub fn new(canvas: HtmlCanvasElement) -> Self {
        // Create the layer now, so that eframe can install its event listeners on the input element
        // at startup.
        let layer = Layer::new(&canvas)
            .inspect_err(|err| log::debug!("No native views below the canvas: {err:?}"))
            .ok();
        Self {
            canvas,
            layer,
            input_style: None,
        }
    }

    /// The element that takes egui's pointer input in front of the views. See the module docs.
    pub fn input_element(&self) -> Option<&HtmlElement> {
        self.layer.as_ref().map(|layer| &layer.input_element)
    }

    /// Whether the keyboard focus is in one of the views below the canvas, such as a text field.
    pub fn has_focus(&self) -> bool {
        let Some(layer) = &self.layer else {
            return false;
        };
        layer
            .element
            .owner_document()
            .and_then(|document| document.active_element())
            .is_some_and(|active| layer.element.contains(Some(&active)))
    }

    /// Updates the input element and the canvas for the views below the canvas in `outputs`.
    ///
    /// While such a view is shown, the canvas has `pointer-events: none`, and the input element
    /// covers the canvas except where the views take the pointer input. Otherwise the input element
    /// is hidden and the canvas takes the pointer input as usual.
    pub fn route_input(&mut self, ctx: &Context, outputs: &[NativeViewOutput]) {
        let Some(layer) = &self.layer else {
            return;
        };
        // The browser delivers the pointer input to views above the canvas without help.
        let mut below = outputs
            .iter()
            .filter(|output| output.side == NativeViewPlacement::Below)
            .peekable();
        let input_style = below.peek().is_some().then(|| {
            // While egui has a pointer button down, the input element covers the views too, so that
            // egui keeps getting the drag. eframe tracks a drag with `mousemove` events on the
            // document, and the document gets none while the pointer is over an `<iframe>`.
            let egui_press = ctx.input(|input| input.pointer.any_down());
            let view_areas: Vec<Rect> = if egui_press {
                Vec::new()
            } else {
                let to_css = to_css_pixels(ctx, &self.canvas);
                crate::native_view::disjoint_union(
                    below
                        .flat_map(NativeViewOutput::view_input_areas)
                        .map(to_css),
                )
            };
            input_element_style(&self.canvas, &layer.element, &view_areas)
        });
        if input_style == self.input_style {
            return;
        }
        let canvas_style = self.canvas.style();
        if let Some(css) = &input_style {
            canvas_style.set_property("pointer-events", "none").ok();
            set_style(&layer.input_element, css);
        } else {
            canvas_style.remove_property("pointer-events").ok();
            set_style(&layer.input_element, "display: none;");
        }
        self.input_style = input_style;
    }
}

/// Returns the layer element of `canvas`: the element that the views below the canvas go in.
/// See [`crate::NativeViews::layer`].
///
/// # Errors
/// If the canvas wasn't in the document when eframe started, so that it has no layer.
pub(crate) fn layer_of(canvas: &HtmlCanvasElement) -> Result<HtmlElement, JsValue> {
    let mut sibling = canvas.previous_element_sibling();
    let layer = loop {
        let Some(element) = sibling else {
            return Err(JsValue::from_str(
                "The canvas wasn't in the document when eframe started",
            ));
        };
        if element.has_attribute(LAYER_ATTRIBUTE) {
            break element.unchecked_into::<HtmlElement>();
        }
        sibling = element.previous_element_sibling();
    };
    // The layer is positioned, and a positioned element paints in front of a static one. The
    // canvas needs a non-static `position` too, to stay in front of the views.
    let is_static = web_sys::window()
        .and_then(|window| window.get_computed_style(canvas).ok().flatten())
        .and_then(|style| style.get_property_value("position").ok())
        .is_some_and(|position| position == "static");
    if is_static {
        canvas.style().set_property("position", "relative")?;
    }
    Ok(layer)
}

/// Returns a function that converts rects from ui points to CSS pixels relative to the top left
/// of `canvas`.
///
/// One ui point is usually [`Context::zoom_factor`] CSS pixels. While the canvas is resized,
/// egui's viewport rect can lag behind the canvas's size on the page, so the scale also includes
/// the ratio between the two.
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

/// Returns the `style` attribute of the input element: it covers `canvas`, with `view_areas` (in
/// CSS pixels relative to the canvas, and not overlapping) cut out.
fn input_element_style(
    canvas: &HtmlCanvasElement,
    layer: &HtmlElement,
    view_areas: &[Rect],
) -> String {
    let rect = canvas.get_bounding_client_rect();
    // The input element is placed in the layer's frame, which is not always the viewport's:
    // with the keyboard up, iOS Safari scrolls the page and reports fixed elements, the layer
    // included, off by the scroll. Placing the element by the canvas's viewport position alone
    // would then move it by the scroll twice.
    let origin = layer.get_bounding_client_rect();
    // The maximum `z-index` puts the input element in front of the views, whose `z-index` is at
    // least 1.
    let mut css = format!(
        "position: fixed; left: {}px; top: {}px; width: {}px; height: {}px; \
         z-index: 2147483647; pointer-events: auto;",
        rect.left() - origin.left(),
        rect.top() - origin.top(),
        rect.width(),
        rect.height(),
    );
    if !view_areas.is_empty() {
        // The path is the input element's rect followed by the view areas. The `evenodd` fill rule
        // cuts out each area, as long as no two areas overlap: it would fill an overlap again.
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
        element.set_attribute(LAYER_ATTRIBUTE, "")?;
        // `position: fixed` makes the element a stacking context, so the `z-index` of the views
        // and the input element only orders them within it. Its children inherit
        // `pointer-events: none`, so the input element and the views have to set
        // `pointer-events: auto`.
        set_style(
            &element,
            "position: fixed; left: 0; top: 0; width: 0; height: 0; pointer-events: none;",
        );
        let input_element: HtmlElement = document.create_element("div")?.unchecked_into();
        set_style(&input_element, "display: none;");
        element.append_child(&input_element)?;
        parent.insert_before(&element, Some(canvas))?;

        Ok(Self {
            element,
            input_element,
        })
    }
}

fn set_style(element: &web_sys::Element, css: &str) {
    element.set_attribute("style", css).ok();
}
