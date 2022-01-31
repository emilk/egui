use egui::{ClippedMesh, Rgba};
use egui_glow::glow;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;
#[cfg(not(target_arch = "wasm32"))]
use web_sys::{WebGl2RenderingContext, WebGlRenderingContext};

pub(crate) struct WrappedGlowPainter {
    pub(crate) glow_ctx: glow::Context,
    pub(crate) canvas: HtmlCanvasElement,
    pub(crate) canvas_id: String,
    pub(crate) painter: egui_glow::Painter,
}

impl WrappedGlowPainter {
    pub fn new(canvas_id: &str) -> Result<Self, String> {
        let canvas = crate::canvas_element_or_die(canvas_id);

        let (glow_ctx, shader_prefix) = init_glow_context_from_canvas(&canvas)?;

        let dimension = [canvas.width() as i32, canvas.height() as i32];
        let painter = egui_glow::Painter::new(&glow_ctx, Some(dimension), shader_prefix)
            .map_err(|error| format!("Error starting glow painter: {}", error))?;

        Ok(Self {
            glow_ctx,
            canvas,
            canvas_id: canvas_id.to_owned(),
            painter,
        })
    }
}

impl crate::Painter for WrappedGlowPainter {
    fn max_texture_side(&self) -> usize {
        self.painter.max_texture_side()
    }

    fn set_texture(&mut self, tex_id: egui::TextureId, delta: &egui::epaint::ImageDelta) {
        self.painter.set_texture(&self.glow_ctx, tex_id, delta);
    }

    fn free_texture(&mut self, tex_id: egui::TextureId) {
        self.painter.free_texture(&self.glow_ctx, tex_id);
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

    fn clear(&mut self, clear_color: Rgba) {
        let canvas_dimension = [self.canvas.width(), self.canvas.height()];
        egui_glow::painter::clear(&self.glow_ctx, canvas_dimension, clear_color)
    }

    fn paint_meshes(
        &mut self,
        clipped_meshes: Vec<ClippedMesh>,
        pixels_per_point: f32,
    ) -> Result<(), JsValue> {
        let canvas_dimension = [self.canvas.width(), self.canvas.height()];
        self.painter.paint_meshes(
            &self.glow_ctx,
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

/// Returns glow context and shader prefix.
fn init_glow_context_from_canvas(
    canvas: &HtmlCanvasElement,
) -> Result<(glow::Context, &'static str), String> {
    const BEST_FIRST: bool = true;

    let result = if BEST_FIRST {
        // Trying WebGl2 first
        init_webgl2(canvas).or_else(|| init_webgl1(canvas))
    } else {
        // Trying WebGl1 first (useful for testing).
        tracing::warn!("Looking for WebGL1 first");
        init_webgl1(canvas).or_else(|| init_webgl2(canvas))
    };

    if let Some(result) = result {
        Ok(result)
    } else {
        Err("WebGL isn't supported".into())
    }
}

fn init_webgl1(canvas: &HtmlCanvasElement) -> Option<(glow::Context, &'static str)> {
    let gl1_ctx = canvas
        .get_context("webgl")
        .expect("Failed to query about WebGL2 context");

    let gl1_ctx = gl1_ctx?;
    tracing::debug!("WebGL1 selected.");

    let gl1_ctx = gl1_ctx
        .dyn_into::<web_sys::WebGlRenderingContext>()
        .unwrap();

    let shader_prefix = if crate::webgl1_requires_brightening(&gl1_ctx) {
        tracing::debug!("Enabling webkitGTK brightening workaround.");
        "#define APPLY_BRIGHTENING_GAMMA"
    } else {
        ""
    };

    let glow_ctx = glow::Context::from_webgl1_context(gl1_ctx);

    Some((glow_ctx, shader_prefix))
}

fn init_webgl2(canvas: &HtmlCanvasElement) -> Option<(glow::Context, &'static str)> {
    let gl2_ctx = canvas
        .get_context("webgl2")
        .expect("Failed to query about WebGL2 context");

    let gl2_ctx = gl2_ctx?;
    tracing::debug!("WebGL2 selected.");

    let gl2_ctx = gl2_ctx
        .dyn_into::<web_sys::WebGl2RenderingContext>()
        .unwrap();
    let glow_ctx = glow::Context::from_webgl2_context(gl2_ctx);
    let shader_prefix = "";

    Some((glow_ctx, shader_prefix))
}

trait DummyWebGLConstructor {
    fn from_webgl1_context(context: web_sys::WebGlRenderingContext) -> Self;

    fn from_webgl2_context(context: web_sys::WebGl2RenderingContext) -> Self;
}

#[cfg(not(target_arch = "wasm32"))]
impl DummyWebGLConstructor for glow::Context {
    fn from_webgl1_context(_context: WebGlRenderingContext) -> Self {
        panic!("you can't use egui_web(glow) on native")
    }

    fn from_webgl2_context(_context: WebGl2RenderingContext) -> Self {
        panic!("you can't use egui_web(glow) on native")
    }
}
