use crate::{canvas_element_or_die, console_error};
use egui::{ClippedMesh, FontImage, Rgba};
use egui_glow::glow;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;
#[cfg(not(target_arch = "wasm32"))]
use web_sys::WebGl2RenderingContext;
use web_sys::WebGlRenderingContext;

pub(crate) struct WrappedGlowPainter {
    pub(crate) gl_ctx: glow::Context,
    pub(crate) canvas: HtmlCanvasElement,
    pub(crate) canvas_id: String,
    pub(crate) painter: egui_glow::Painter,
}

impl WrappedGlowPainter {
    pub fn new(canvas_id: &str) -> Self {
        let canvas = canvas_element_or_die(canvas_id);

        let shader_prefix = if requires_brightening(&canvas) {
            crate::console_log("Enabling webkitGTK brightening workaround");
            "#define APPLY_BRIGHTENING_GAMMA"
        } else {
            ""
        };

        let gl_ctx = init_glow_context_from_canvas(&canvas);
        let dimension = [canvas.width() as i32, canvas.height() as i32];
        let painter = egui_glow::Painter::new(&gl_ctx, Some(dimension), shader_prefix)
            .map_err(|error| {
                console_error(format!(
                    "some error occurred in initializing glow painter\n {}",
                    error
                ))
            })
            .unwrap();

        Self {
            gl_ctx,
            canvas,
            canvas_id: canvas_id.to_owned(),
            painter,
        }
    }
}

fn requires_brightening(canvas: &web_sys::HtmlCanvasElement) -> bool {
    // See https://github.com/emilk/egui/issues/794

    // detect WebKitGTK

    // WebKitGTK currently support only webgl,so request webgl context.
    // WebKitGTK use WebKit default unmasked vendor and renderer
    // but safari use same vendor and renderer
    // so exclude "Mac OS X" user-agent.
    let gl = canvas
        .get_context("webgl")
        .unwrap()
        .unwrap()
        .dyn_into::<WebGlRenderingContext>()
        .unwrap();
    let user_agent = web_sys::window().unwrap().navigator().user_agent().unwrap();
    crate::is_safari_and_webkit_gtk(&gl) && !user_agent.contains("Mac OS X")
}

impl crate::Painter for WrappedGlowPainter {
    fn set_texture(&mut self, tex_id: u64, image: epi::Image) {
        self.painter.set_texture(&self.gl_ctx, tex_id, &image);
    }

    fn free_texture(&mut self, tex_id: u64) {
        self.painter.free_texture(tex_id);
    }

    fn debug_info(&self) -> String {
        format!(
            "Stored canvas size: {} x {}",
            self.canvas.width(),
            self.canvas.height(),
        )
    }

    fn canvas_id(&self) -> &str {
        &self.canvas_id
    }

    fn upload_egui_texture(&mut self, font_image: &FontImage) {
        self.painter.upload_egui_texture(&self.gl_ctx, font_image)
    }

    fn clear(&mut self, clear_color: Rgba) {
        let canvas_dimension = [self.canvas.width(), self.canvas.height()];
        egui_glow::painter::clear(&self.gl_ctx, canvas_dimension, clear_color)
    }

    fn paint_meshes(
        &mut self,
        clipped_meshes: Vec<ClippedMesh>,
        pixels_per_point: f32,
    ) -> Result<(), JsValue> {
        let canvas_dimension = [self.canvas.width(), self.canvas.height()];
        self.painter.paint_meshes(
            &self.gl_ctx,
            canvas_dimension,
            pixels_per_point,
            clipped_meshes,
        );
        Ok(())
    }

    fn name(&self) -> &'static str {
        "egui_web (glow)"
    }
}

pub fn init_glow_context_from_canvas(canvas: &HtmlCanvasElement) -> glow::Context {
    let gl2_ctx = canvas
        .get_context("webgl2")
        .expect("Failed to query about WebGL2 context");

    if let Some(gl2_ctx) = gl2_ctx {
        crate::console_log("WebGL2 found");
        let gl2_ctx = gl2_ctx
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .unwrap();
        glow::Context::from_webgl2_context(gl2_ctx)
    } else {
        let gl1 = canvas
            .get_context("webgl")
            .expect("Failed to query about WebGL1 context");

        if let Some(gl1) = gl1 {
            crate::console_log("WebGL2 not available - falling back to WebGL2");
            let gl1_ctx = gl1.dyn_into::<web_sys::WebGlRenderingContext>().unwrap();
            glow::Context::from_webgl1_context(gl1_ctx)
        } else {
            panic!("Failed to get WebGL context.");
        }
    }
}

trait DummyWebGLConstructor {
    fn from_webgl1_context(context: web_sys::WebGlRenderingContext) -> Self;

    fn from_webgl2_context(context: web_sys::WebGl2RenderingContext) -> Self;
}

#[cfg(not(target_arch = "wasm32"))]
impl DummyWebGLConstructor for glow::Context {
    fn from_webgl1_context(_context: WebGlRenderingContext) -> Self {
        panic!("you cant use egui_web(glow) on native")
    }

    fn from_webgl2_context(_context: WebGl2RenderingContext) -> Self {
        panic!("you cant use egui_web(glow) on native")
    }
}
