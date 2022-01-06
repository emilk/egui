// epi related implementations here.
use std::borrow::Borrow;
use std::sync::atomic::Ordering::SeqCst;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

/*
    repaint signal for web.
*/

pub struct NeedRepaint(std::sync::atomic::AtomicBool);

impl Default for NeedRepaint {
    fn default() -> Self {
        Self(true.into())
    }
}

impl NeedRepaint {
    #[allow(dead_code)]
    pub fn fetch_and_clear(&self) -> bool {
        self.0.swap(false, SeqCst)
    }
    #[allow(dead_code)]
    pub fn set_true(&self) {
        self.0.store(true, SeqCst);
    }
}

impl epi::backend::RepaintSignal for NeedRepaint {
    fn request_repaint(&self) {
        self.0.store(true, SeqCst);
    }
}

#[allow(dead_code)]
pub(crate) struct WebGLWindowedContextLike {
    canvas: HtmlCanvasElement,
    window: egui_winit::winit::window::Window,
}

impl WebGLWindowedContextLike {
    pub(crate) fn window(&self) -> &egui_winit::winit::window::Window {
        self.window.borrow()
    }
}

pub(crate) fn create_gl_context(
    window_builder: egui_winit::winit::window::WindowBuilder,
    event_loop: &egui_winit::winit::event_loop::EventLoop<()>,
) -> Result<(WebGLWindowedContextLike, (glow::Context, bool)), wasm_bindgen::JsValue> {
    fn is_safari_and_webkit_gtk(gl: &web_sys::WebGlRenderingContext) -> bool {
        if let Ok(renderer) =
            gl.get_parameter(web_sys::WebglDebugRendererInfo::UNMASKED_RENDERER_WEBGL)
        {
            if let Some(renderer) = renderer.as_string() {
                if renderer.contains("Apple") {
                    return true;
                }
            }
        }

        false
    }
    // and detect WebKitGTK.
    fn init_glow_context_from_canvas(canvas: &HtmlCanvasElement) -> (glow::Context, bool) {
        let gl2_ctx = canvas
            .get_context("webgl2")
            .expect("Failed to query about WebGL2 context");

        if let Some(gl2_ctx) = gl2_ctx {
            crate::misc_util::glow_debug_print("WebGL2 found");
            let gl2_ctx = gl2_ctx
                .dyn_into::<web_sys::WebGl2RenderingContext>()
                .unwrap();
            (glow::Context::from_webgl2_context(gl2_ctx), false)
        } else {
            let gl1 = canvas
                .get_context("webgl")
                .expect("Failed to query about WebGL1 context");

            if let Some(gl1) = gl1 {
                crate::misc_util::glow_debug_print("WebGL2 not available - falling back to WebGL1");
                let gl1_ctx = gl1.dyn_into::<web_sys::WebGlRenderingContext>().unwrap();
                let user_agent = web_sys::window().unwrap().navigator().user_agent().unwrap();
                let needs_gamma_collection =
                    is_safari_and_webkit_gtk(&gl1_ctx) && !user_agent.contains("Mac OS X");
                (
                    glow::Context::from_webgl1_context(gl1_ctx),
                    needs_gamma_collection,
                )
            } else {
                panic!("Failed to get WebGL context.");
            }
        }
    }
    use egui_winit::winit::platform::web::WindowExtWebSys;
    let window = window_builder.build(event_loop).unwrap();
    let canvas: HtmlCanvasElement = window.canvas();
    {
        // By default, right-clicks open a context menu.
        // We don't want to do that (right clicks is handled by egui):
        let event_name = "contextmenu";
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    let web_window = web_sys::window().unwrap();
    let document = web_window.document().unwrap();
    let body = document.body().unwrap();
    let glow_ctx = init_glow_context_from_canvas(&canvas);
    body.append_child(&canvas)
        .expect("Append canvas to HTML body");
    Ok((WebGLWindowedContextLike { canvas, window }, glow_ctx))
}

/*
use wasm_bindgen::JsValue;
use std::cell::Cell;
use std::rc::Rc;
///
static AGENT_ID: &str = "egui_text_agent";
///
/// Text event handler,
fn install_text_agent(runner_ref: &AppRunnerRef) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().expect("document should have a body");
    let input = document
        .create_element("input")?
        .dyn_into::<web_sys::HtmlInputElement>()?;
    let input = std::rc::Rc::new(input);
    input.set_id(AGENT_ID);
    let is_composing = Rc::new(Cell::new(false));
    {
        let style = input.style();
        // Transparent
        style.set_property("opacity", "0").unwrap();
        // Hide under canvas
        style.set_property("z-index", "-1").unwrap();
    }
    // Set size as small as possible, in case user may click on it.
    input.set_size(1);
    input.set_autofocus(true);
    input.set_hidden(true);
    {
        // When IME is off
        let input_clone = input.clone();
        let runner_ref = runner_ref.clone();
        let is_composing = is_composing.clone();
        let on_input = Closure::wrap(Box::new(move |_event: web_sys::InputEvent| {
            let text = input_clone.value();
            if !text.is_empty() && !is_composing.get() {
                input_clone.set_value("");
                let mut runner_lock = runner_ref.0.lock();
                runner_lock.input.raw.events.push(egui::Event::Text(text));
                runner_lock.needs_repaint.set_true();
            }
        }) as Box<dyn FnMut(_)>);
        input.add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref())?;
        on_input.forget();
    }
    {
        // When IME is on, handle composition event
        let input_clone = input.clone();
        let runner_ref = runner_ref.clone();
        let on_compositionend = Closure::wrap(Box::new(move |event: web_sys::CompositionEvent| {
            let mut runner_lock = runner_ref.0.lock();
            let opt_event = match event.type_().as_ref() {
                "compositionstart" => {
                    is_composing.set(true);
                    input_clone.set_value("");
                    Some(egui::Event::CompositionStart)
                }
                "compositionend" => {
                    is_composing.set(false);
                    input_clone.set_value("");
                    event.data().map(egui::Event::CompositionEnd)
                }
                "compositionupdate" => event.data().map(egui::Event::CompositionUpdate),
                s => {
                   web_sys::console::error_1(&format!("Unknown composition event type: {:?}", s).into());
                    None
                }
            };
            if let Some(event) = opt_event {
                runner_lock.input.raw.events.push(event);
                runner_lock.needs_repaint.set_true();
            }
        }) as Box<dyn FnMut(_)>);
        let f = on_compositionend.as_ref().unchecked_ref();
        input.add_event_listener_with_callback("compositionstart", f)?;
        input.add_event_listener_with_callback("compositionupdate", f)?;
        input.add_event_listener_with_callback("compositionend", f)?;
        on_compositionend.forget();
    }
    {
        // When input lost focus, focus on it again.
        // It is useful when user click somewhere outside canvas.
        let on_focusout = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            // Delay 10 ms, and focus again.
            let func = js_sys::Function::new_no_args(&format!(
                "document.getElementById('{}').focus()",
                AGENT_ID
            ));
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(&func, 10)
                .unwrap();
        }) as Box<dyn FnMut(_)>);
        input.add_event_listener_with_callback("focusout", on_focusout.as_ref().unchecked_ref())?;
        on_focusout.forget();
    }
    body.append_child(&input)?;
    Ok(())
}
*/
