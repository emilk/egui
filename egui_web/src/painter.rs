use wasm_bindgen::prelude::JsValue;

/// What is needed to paint egui.
pub trait WebPainter {
    fn name(&self) -> &'static str;

    /// Max size of one side of a texture.
    fn max_texture_side(&self) -> usize;

    /// id of the canvas html element containing the rendering
    fn canvas_id(&self) -> &str;

    fn set_texture(&mut self, tex_id: egui::TextureId, delta: &egui::epaint::ImageDelta);

    fn free_texture(&mut self, tex_id: egui::TextureId);

    fn clear(&mut self, clear_color: egui::Rgba);

    fn paint_primitives(
        &mut self,
        clipped_primitives: Vec<egui::ClippedPrimitive>,
        pixels_per_point: f32,
    ) -> Result<(), JsValue>;

    fn paint_and_update_textures(
        &mut self,
        clipped_primitives: Vec<egui::ClippedPrimitive>,
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
}
