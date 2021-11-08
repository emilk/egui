#[cfg(not(target_arch = "wasm32"))]
use crate::web_sys::{WebGl2RenderingContext, WebGlRenderingContext};
use crate::{canvas_element_or_die, console_error, console_log};
use egui::{ClippedMesh, Rgba, Texture};
use egui_glow::glow;
use epi::TextureAllocator;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

pub(crate) struct WrappedGlowPainter {
    pub(crate) gl_ctx: glow::Context,
    pub(crate) canvas: HtmlCanvasElement,
    pub(crate) canvas_id: String,
    pub(crate) painter: egui_glow::Painter,
}

impl WrappedGlowPainter {
    pub fn new(canvas_id: &str) -> Self {
        let user_agent = web_sys::window().unwrap().navigator().user_agent().unwrap();
        let epiphany_wr = if user_agent.contains("Epiphany") {
            console_log("Enabling epiphany workaround");
            vec!["#define EPIPHANY_WORKAROUND".to_owned()]
        } else {
            vec![]
        };
        let canvas = canvas_element_or_die(canvas_id);
        let gl_ctx = init_glow_context_from_canvas(&canvas);
        let dimension = [canvas.width() as i32, canvas.height() as i32];
        let painter = egui_glow::Painter::new(&gl_ctx, Some(dimension), &epiphany_wr)
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
impl crate::Painter for WrappedGlowPainter {
    fn as_tex_allocator(&mut self) -> &mut dyn TextureAllocator {
        &mut self.painter
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

    fn upload_egui_texture(&mut self, texture: &Texture) {
        self.painter.upload_egui_texture(&self.gl_ctx, texture)
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
            canvas_dimension,
            &self.gl_ctx,
            pixels_per_point,
            clipped_meshes,
        );
        Ok(())
    }

    fn name(&self) -> &'static str {
        "egui_web(glow)"
    }
}

pub fn init_glow_context_from_canvas(canvas: &HtmlCanvasElement) -> glow::Context {
    use wasm_bindgen::JsCast;
    let ctx = canvas.get_context("webgl2");
    if let Ok(ctx) = ctx {
        crate::console_log("webgl found");
        if let Some(ctx) = ctx {
            crate::console_log("webgl 2 selected");
            let gl_ctx = ctx.dyn_into::<web_sys::WebGl2RenderingContext>().unwrap();
            glow::Context::from_webgl2_context(gl_ctx)
        } else {
            let ctx = canvas.get_context("webgl");
            if let Ok(ctx) = ctx {
                crate::console_log("falling back to webgl1");
                if let Some(ctx) = ctx {
                    crate::console_log("webgl1 selected");

                    let gl_ctx = ctx.dyn_into::<web_sys::WebGlRenderingContext>().unwrap();
                    crate::console_log("success");
                    glow::Context::from_webgl1_context(gl_ctx)
                } else {
                    panic!("tried webgl1 but can't get context");
                }
            } else {
                panic!("tried webgl1 but can't get context");
            }
        }
    } else {
        panic!("tried webgl2 but something went wrong");
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
