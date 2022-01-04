// epi related implementations here.
#[cfg(target_arch = "wasm32")]
use std::borrow::Borrow;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

/*
    repaint signal for web.
*/
#[cfg(target_arch = "wasm32")]
use std::sync::atomic::Ordering::SeqCst;
#[cfg(target_arch = "wasm32")]
pub struct NeedRepaint(std::sync::atomic::AtomicBool);
#[cfg(target_arch = "wasm32")]
impl Default for NeedRepaint {
    fn default() -> Self {
        Self(true.into())
    }
}
#[cfg(target_arch = "wasm32")]
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
#[cfg(target_arch = "wasm32")]
impl epi::backend::RepaintSignal for NeedRepaint {
    fn request_repaint(&self) {
        self.0.store(true, SeqCst);
    }
}

/* minimally emulates glutin::WindowedContext.
*/
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub(crate) struct WebGLWindowedContextLike {
    canvas: HtmlCanvasElement,
    window: egui_winit::winit::window::Window,
}
#[cfg(target_arch = "wasm32")]
impl WebGLWindowedContextLike {
    pub(crate) fn window(&self) -> &egui_winit::winit::window::Window {
        self.window.borrow()
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn create_gl_context(
    window_builder: egui_winit::winit::window::WindowBuilder,
    event_loop: &egui_winit::winit::event_loop::EventLoop<()>,
) -> Result<(WebGLWindowedContextLike, (glow::Context, bool)), wasm_bindgen::JsValue> {
    pub(crate) fn is_safari_and_webkit_gtk(gl: &web_sys::WebGlRenderingContext) -> bool {
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
        use wasm_bindgen::closure::Closure;
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
    Ok((
        WebGLWindowedContextLike {
            canvas,
            window: window,
        },
        glow_ctx,
    ))
}
