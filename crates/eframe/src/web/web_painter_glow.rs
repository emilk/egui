use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use egui_glow::glow;

use crate::{WebGlContextOption, WebOptions};

use super::web_painter::WebPainter;

pub(crate) struct WebPainterGlow {
    canvas: HtmlCanvasElement,
    canvas_id: String,
    painter: egui_glow::Painter,
}

impl WebPainterGlow {
    pub fn gl(&self) -> &std::sync::Arc<glow::Context> {
        self.painter.gl()
    }

    pub async fn new(canvas_id: &str, options: &WebOptions) -> Result<Self, String> {
        let canvas = super::canvas_element_or_die(canvas_id);

        let (gl, shader_prefix) =
            init_glow_context_from_canvas(&canvas, options.webgl_context_option)?;
        let gl = std::sync::Arc::new(gl);

        let painter = egui_glow::Painter::new(gl, shader_prefix, None)
            .map_err(|error| format!("Error starting glow painter: {}", error))?;

        Ok(Self {
            canvas,
            canvas_id: canvas_id.to_owned(),
            painter,
        })
    }
}

impl WebPainter for WebPainterGlow {
    fn max_texture_side(&self) -> usize {
        self.painter.max_texture_side()
    }

    fn canvas_id(&self) -> &str {
        &self.canvas_id
    }

    fn paint_and_update_textures(
        &mut self,
        clear_color: [f32; 4],
        clipped_primitives: &[egui::ClippedPrimitive],
        pixels_per_point: f32,
        textures_delta: &egui::TexturesDelta,
    ) -> Result<(), JsValue> {
        let canvas_dimension = [self.canvas.width(), self.canvas.height()];

        for (id, image_delta) in &textures_delta.set {
            self.painter.set_texture(*id, image_delta);
        }

        egui_glow::painter::clear(self.painter.gl(), canvas_dimension, clear_color);
        self.painter
            .paint_primitives(canvas_dimension, pixels_per_point, clipped_primitives);

        for &id in &textures_delta.free {
            self.painter.free_texture(id);
        }

        Ok(())
    }

    fn destroy(&mut self) {
        self.painter.destroy();
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

    let shader_prefix = if webgl1_requires_brightening(&gl1_ctx) {
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

fn webgl1_requires_brightening(gl: &web_sys::WebGlRenderingContext) -> bool {
    // See https://github.com/emilk/egui/issues/794

    // detect WebKitGTK

    // WebKitGTK use WebKit default unmasked vendor and renderer
    // but safari use same vendor and renderer
    // so exclude "Mac OS X" user-agent.
    let user_agent = web_sys::window().unwrap().navigator().user_agent().unwrap();
    !user_agent.contains("Mac OS X") && is_safari_and_webkit_gtk(gl)
}

/// detecting Safari and `webkitGTK`.
///
/// Safari and `webkitGTK` use unmasked renderer :Apple GPU
///
/// If we detect safari or `webkitGTKs` returns true.
///
/// This function used to avoid displaying linear color with `sRGB` supported systems.
fn is_safari_and_webkit_gtk(gl: &web_sys::WebGlRenderingContext) -> bool {
    // This call produces a warning in Firefox ("WEBGL_debug_renderer_info is deprecated in Firefox and will be removed.")
    // but unless we call it we get errors in Chrome when we call `get_parameter` below.
    // TODO(emilk): do something smart based on user agent?
    if gl
        .get_extension("WEBGL_debug_renderer_info")
        .unwrap()
        .is_some()
    {
        if let Ok(renderer) =
            gl.get_parameter(web_sys::WebglDebugRendererInfo::UNMASKED_RENDERER_WEBGL)
        {
            if let Some(renderer) = renderer.as_string() {
                if renderer.contains("Apple") {
                    return true;
                }
            }
        }
    }

    false
}
