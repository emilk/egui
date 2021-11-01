use egui::{ClippedMesh, Rgba, Texture};
use epi::TextureAllocator;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

pub(crate) struct WrappedGlowPainter {
    pub(crate) gl_ctx: egui_glow::Context,
    pub(crate) canvas: HtmlCanvasElement,
    pub(crate) canvas_id: String,
    pub(crate) painter: egui_glow::Painter,
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
}
