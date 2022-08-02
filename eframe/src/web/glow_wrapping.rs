use crate::WebGlContextOption;
use egui::{ClippedPrimitive, Rgba};
use egui_glow::glow;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;
#[cfg(not(target_arch = "wasm32"))]
use web_sys::{WebGl2RenderingContext, WebGlRenderingContext};

pub(crate) struct WrappedGlowPainter {
    pub(crate) canvas: HtmlCanvasElement,
    pub(crate) canvas_id: String,
    pub(crate) painter: egui_glow::Painter,
}

impl WrappedGlowPainter {
    pub fn new(canvas_id: &str, options: WebGlContextOption) -> Result<Self, String> {
        let canvas = super::canvas_element_or_die(canvas_id);

        let (gl, shader_prefix) = init_glow_context_from_canvas(&canvas, options)?;
        let gl = std::sync::Arc::new(gl);

        let dimension = [canvas.width() as i32, canvas.height() as i32];
        let painter = egui_glow::Painter::new(gl, Some(dimension), shader_prefix)
            .map_err(|error| format!("Error starting glow painter: {}", error))?;

        Ok(Self {
            canvas,
            canvas_id: canvas_id.to_owned(),
            painter,
        })
    }
}

impl WrappedGlowPainter {
    pub fn gl(&self) -> &std::sync::Arc<glow::Context> {
        self.painter.gl()
    }

    pub fn max_texture_side(&self) -> usize {
        self.painter.max_texture_side()
    }

    pub fn canvas_id(&self) -> &str {
        &self.canvas_id
    }

    pub fn set_texture(&mut self, tex_id: egui::TextureId, delta: &egui::epaint::ImageDelta) {
        self.painter.set_texture(tex_id, delta);
    }

    pub fn free_texture(&mut self, tex_id: egui::TextureId) {
        self.painter.free_texture(tex_id);
    }

    pub fn clear(&self, clear_color: Rgba) {
        let canvas_dimension = [self.canvas.width(), self.canvas.height()];
        egui_glow::painter::clear(self.painter.gl(), canvas_dimension, clear_color);
    }

    pub fn paint_primitives(
        &mut self,
        clipped_primitives: &[ClippedPrimitive],
        pixels_per_point: f32,
    ) -> Result<(), JsValue> {
        let canvas_dimension = [self.canvas.width(), self.canvas.height()];
        self.painter
            .paint_primitives(canvas_dimension, pixels_per_point, clipped_primitives);
        Ok(())
    }

    pub fn paint_and_update_textures(
        &mut self,
        clipped_primitives: &[egui::ClippedPrimitive],
        pixels_per_point: f32,
        textures_delta: &egui::TexturesDelta,
    ) -> Result<(), JsValue> {
        for (id, image_delta) in &textures_delta.set {
            self.set_texture(*id, image_delta);
        }

        self.paint_primitives(clipped_primitives, pixels_per_point)?;

        for &id in &textures_delta.free {
            self.free_texture(id);
        }

        Ok(())
    }

    pub fn destroy(&mut self) {
        self.painter.destroy()
    }
}

/// Returns glow context and shader prefix.
fn init_glow_context_from_canvas(
    canvas: &HtmlCanvasElement,
    options: WebGlContextOption,
) -> Result<(glow::Context, &'static str), String> {
    let result = match options {
        // Force use WebGl1
        WebGlContextOption::WebGl1 => init_webgl1(canvas),
        // Force use WebGl2
        WebGlContextOption::WebGl2 => init_webgl2(canvas),
        // Trying WebGl2 first
        WebGlContextOption::BestFirst => init_webgl2(canvas).or_else(|| init_webgl1(canvas)),
        // Trying WebGl1 first (useful for testing).
        WebGlContextOption::CompatibilityFirst => {
            init_webgl1(canvas).or_else(|| init_webgl2(canvas))
        }
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

    let shader_prefix = if super::webgl1_requires_brightening(&gl1_ctx) {
        tracing::debug!("Enabling webkitGTK brightening workaround.");
        "#define APPLY_BRIGHTENING_GAMMA"
    } else {
        ""
    };

    let gl = glow::Context::from_webgl1_context(gl1_ctx);

    Some((gl, shader_prefix))
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
    let gl = glow::Context::from_webgl2_context(gl2_ctx);
    let shader_prefix = "";

    Some((gl, shader_prefix))
}

trait DummyWebGLConstructor {
    fn from_webgl1_context(context: web_sys::WebGlRenderingContext) -> Self;

    fn from_webgl2_context(context: web_sys::WebGl2RenderingContext) -> Self;
}
