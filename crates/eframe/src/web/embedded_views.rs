//! The runner's state for the views below the canvas: the element they go in, and the routing
//! of the pointer input.
//!
//! The canvas is in front of the views below egui's surface
//! ([`egui::EmbeddedViewPlacement::Below`]), so the browser's hit-testing would send all of the
//! pointer input to the canvas. The views need the input where egui doesn't take it. CSS can't
//! limit the hit-testing of an element to part of its area without also clipping its drawing:
//! `clip-path` clips both.
//!
//! So while such a view is shown, the canvas is only used for display and has
//! `pointer-events: none`. Instead, an invisible input element in front of the canvas and the
//! views takes egui's pointer input, and eframe listens for pointer events on the input element
//! like on the canvas. The input element takes the input through rectangles that together cover
//! the canvas except where the views take the input
//! ([`egui::EmbeddedViewOutput::view_input_areas`]), so hit-testing there finds the views. The
//! canvas isn't clipped, so egui paints over the views down to the last pixel, shadows included.
//!
//! The rectangles are elements of their own, rather than one element with the views cut out of
//! it by a `clip-path`. A page from another origin runs in a process of its own, and Firefox
//! picks the process that gets a click by the bounding box of a `clip-path`, which a hole doesn't
//! shrink. The click would go to the app's process and never reach the page.
//!
//! The keyboard focus stays on the canvas: the input element isn't focusable, and eframe cancels
//! the default action of a `mousedown` on the input element, which would move the focus.

use egui::{Context, EmbeddedViewOutput, EmbeddedViewPlacement, Pos2, Rect, vec2};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use web_sys::{HtmlCanvasElement, HtmlElement};

/// The runner's state for the views below the canvas. See the module docs.
pub(crate) struct WebEmbeddedViews {
    canvas: HtmlCanvasElement,

    /// `None` if the canvas wasn't in a document when eframe started. Then there are no views
    /// below the canvas for the life of the runner, and [`crate::EmbeddedViews::container`] says
    /// why.
    elements: Option<Elements>,

    /// How the input element is laid out now, as of the last pass, so that an unchanged layout
    /// isn't set again. `None` while the input element is hidden.
    current_input_style: Option<InputStyle>,
}

/// The layout of the input element: where it is, and where in it egui takes the pointer input.
#[derive(PartialEq)]
struct InputStyle {
    /// The `style` attribute of the input element, which places it over the canvas.
    element: String,

    /// The rects that take egui's pointer input, in CSS pixels relative to the input element.
    /// They don't overlap.
    rects: Vec<Rect>,
}

/// The attribute that marks the container, so that [`container_of`] can find it.
const CONTAINER_ATTRIBUTE: &str = "data-egui-embedded-view-container";

/// The elements that eframe adds to the document for the views below the canvas.
struct Elements {
    /// The `position: fixed` container of the views and the input element, inserted right before
    /// the canvas. It precedes the canvas in the document, so a positioned canvas paints in front
    /// of it. See [`WebEmbeddedViews::route_input`] for a canvas that isn't positioned.
    container: HtmlElement,

    /// Takes egui's pointer input while views below the canvas are shown. See the module docs.
    ///
    /// It covers the canvas, but takes the input only through [`Self::input_rects`], so eframe
    /// listens on it for the events that bubble up from them.
    input: HtmlElement,

    /// The children of [`Self::input`] that take the pointer input, one per rect of
    /// [`InputStyle::rects`]. The ones that aren't needed are hidden.
    ///
    /// None is ever removed from the document: a touch keeps reporting to the element it started
    /// on, and the events of a removed element no longer reach the document, where eframe
    /// listens for the end of a drag.
    input_rects: Vec<HtmlElement>,
}

impl Drop for Elements {
    fn drop(&mut self) {
        self.container.remove();
    }
}

impl WebEmbeddedViews {
    pub fn new(canvas: HtmlCanvasElement) -> Self {
        // Create the elements now, so that eframe can install its event listeners on the input element
        // at startup.
        let elements = Elements::new(&canvas)
            .inspect_err(|err| log::debug!("No embedded views below the canvas: {err:?}"))
            .ok();
        Self {
            canvas,
            elements,
            current_input_style: None,
        }
    }

    /// The element that takes egui's pointer input in front of the views. See the module docs.
    pub fn input_element(&self) -> Option<&HtmlElement> {
        self.elements.as_ref().map(|elements| &elements.input)
    }

    /// Whether the keyboard focus is in one of the views below the canvas, such as a text field.
    pub fn has_focus(&self) -> bool {
        let Some(elements) = &self.elements else {
            return false;
        };
        elements
            .container
            .owner_document()
            .and_then(|document| document.active_element())
            .is_some_and(|active| elements.container.contains(Some(&active)))
    }

    /// Updates the input element and the canvas for the views below the canvas in `outputs`.
    ///
    /// While such a view is shown, the canvas has `pointer-events: none`, and the input element
    /// takes the pointer input over the canvas except where the views take it. Otherwise the input
    /// element is hidden and the canvas takes the pointer input as usual.
    ///
    /// When such a view is first shown, a canvas with `position: static` also gets
    /// `position: relative`, and keeps it. The container is positioned, and a positioned element
    /// paints in front of a static one, so the canvas needs a non-static `position` too, to stay
    /// in front of the views.
    pub fn route_input(&mut self, ctx: &Context, outputs: &[EmbeddedViewOutput]) {
        let Self {
            canvas,
            elements: Some(elements),
            current_input_style,
        } = self
        else {
            return;
        };
        // The browser delivers the pointer input to views above the canvas without help.
        let mut below = outputs
            .iter()
            .filter(|output| output.side == EmbeddedViewPlacement::Below)
            .peekable();
        let input_style = below.peek().is_some().then(|| {
            // While egui has a pointer button down, the input element covers the views too, so that
            // egui keeps getting the drag. eframe tracks a drag with `mousemove` events on the
            // document, and the document gets none while the pointer is over an `<iframe>`.
            let egui_press = ctx.input(|input| input.pointer.any_down());
            let view_areas: Vec<Rect> = if egui_press {
                Vec::new()
            } else {
                let to_css = to_css_pixels(ctx, canvas);
                below
                    .flat_map(EmbeddedViewOutput::view_input_areas)
                    .map(to_css)
                    .collect()
            };
            input_style(canvas, &elements.container, &view_areas)
        });
        if input_style == *current_input_style {
            return;
        }
        let canvas_style = canvas.style();
        if let Some(style) = &input_style {
            canvas_style.set_property("pointer-events", "none").ok();
            // Only when the views come on screen, not on every change of the input element.
            if current_input_style.is_none() && is_static(canvas) {
                canvas_style.set_property("position", "relative").ok();
            }
            set_style(&elements.input, &style.element);
            if let Err(err) = elements.set_input_rects(&style.rects) {
                log::warn!(
                    "Failed to lay out the input element: {}",
                    super::string_from_js_value(&err)
                );
            }
        } else {
            canvas_style.remove_property("pointer-events").ok();
            set_style(&elements.input, "display: none;");
        }
        *current_input_style = input_style;
    }
}

/// Returns the container of `canvas`: the element that the views below the canvas go in.
/// See [`crate::EmbeddedViews::container`].
///
/// # Errors
/// If the container isn't before the canvas under the same parent: the canvas wasn't in the
/// document when eframe started, so that eframe made no container, or it was moved to another
/// parent since.
pub(crate) fn container_of(canvas: &HtmlCanvasElement) -> Result<HtmlElement, JsValue> {
    let mut sibling = canvas.previous_element_sibling();
    loop {
        let Some(element) = sibling else {
            return Err(JsValue::from_str(
                "eframe has no element for embedded views next to its canvas. Add the canvas to \
                 the document before starting eframe, and keep it under the same parent element.",
            ));
        };
        if element.has_attribute(CONTAINER_ATTRIBUTE) {
            return Ok(element.unchecked_into::<HtmlElement>());
        }
        sibling = element.previous_element_sibling();
    }
}

/// Whether the computed `position` of `canvas` is `static`.
fn is_static(canvas: &HtmlCanvasElement) -> bool {
    web_sys::window()
        .and_then(|window| window.get_computed_style(canvas).ok().flatten())
        .and_then(|style| style.get_property_value("position").ok())
        .is_some_and(|position| position == "static")
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

/// Returns the layout of the input element: it covers `canvas`, and takes the pointer input
/// everywhere but in `view_areas` (in CSS pixels relative to the canvas).
fn input_style(
    canvas: &HtmlCanvasElement,
    container: &HtmlElement,
    view_areas: &[Rect],
) -> InputStyle {
    let rect = canvas.get_bounding_client_rect();
    // The input element is placed in the container's frame, which is not always the viewport's:
    // with the keyboard up, iOS Safari scrolls the page and reports fixed elements, the container
    // included, off by the scroll. Placing the element by the canvas's viewport position alone
    // would then move it by the scroll twice.
    let origin = container.get_bounding_client_rect();
    // The maximum `z-index` puts the input element in front of the views. It takes no pointer
    // input itself, only its rects do.
    let element = format!(
        "position: fixed; left: {}px; top: {}px; width: {}px; height: {}px; \
         z-index: 2147483647; pointer-events: none;",
        rect.left() - origin.left(),
        rect.top() - origin.top(),
        rect.width(),
        rect.height(),
    );
    let canvas_rect =
        Rect::from_min_size(Pos2::ZERO, vec2(rect.width() as f32, rect.height() as f32));
    InputStyle {
        element,
        rects: crate::embedded_view::subtract_all(canvas_rect, view_areas),
    }
}

impl Elements {
    fn new(canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        let document = canvas
            .owner_document()
            .ok_or_else(|| JsValue::from_str("The canvas has no document"))?;
        let parent = canvas
            .parent_node()
            .ok_or_else(|| JsValue::from_str("The canvas isn't in the document"))?;

        let container: HtmlElement = document.create_element("div")?.unchecked_into();
        container.set_attribute(CONTAINER_ATTRIBUTE, "")?;
        // `position: fixed` makes the container a stacking context, so the `z-index` of the views
        // and the input element only orders them within it. Its children inherit
        // `pointer-events: none`, so the input element and the views have to set
        // `pointer-events: auto`.
        set_style(
            &container,
            "position: fixed; left: 0; top: 0; width: 0; height: 0; pointer-events: none;",
        );
        let input: HtmlElement = document.create_element("div")?.unchecked_into();
        set_style(&input, "display: none;");
        container.append_child(&input)?;
        parent.insert_before(&container, Some(canvas))?;

        Ok(Self {
            container,
            input,
            input_rects: Vec::new(),
        })
    }

    /// Lays out [`Self::input_rects`] as `rects`, in CSS pixels relative to the input element,
    /// and hides the ones that are left over.
    fn set_input_rects(&mut self, rects: &[Rect]) -> Result<(), JsValue> {
        while self.input_rects.len() < rects.len() {
            let document = self
                .input
                .owner_document()
                .ok_or_else(|| JsValue::from_str("The input element has no document"))?;
            let element: HtmlElement = document.create_element("div")?.unchecked_into();
            self.input.append_child(&element)?;
            self.input_rects.push(element);
        }
        for (i, element) in self.input_rects.iter().enumerate() {
            if let Some(rect) = rects.get(i) {
                set_style(
                    element,
                    &format!(
                        "position: absolute; left: {}px; top: {}px; width: {}px; height: {}px; \
                         pointer-events: auto;",
                        rect.min.x,
                        rect.min.y,
                        rect.width(),
                        rect.height(),
                    ),
                );
            } else {
                set_style(element, "display: none;");
            }
        }
        Ok(())
    }
}

fn set_style(element: &web_sys::Element, css: &str) {
    element.set_attribute("style", css).ok();
}
