use wasm_bindgen::JsValue;

/// Renderer for a browser canvas.
/// As of writing we're not allowing to decide on the painter at runtime,
/// therefore this trait is merely there for specifying and documenting the interface.
pub(crate) trait WebPainter {
    // Create a new web painter targeting a given canvas.
    // fn new(canvas_id: &str, options: &WebOptions) -> Result<Self, String>
    // where
    //     Self: Sized;

    /// Id of the canvas in use.
    fn canvas_id(&self) -> &str;

    /// Maximum size of a texture in one direction.
    fn max_texture_side(&self) -> usize;

    /// Update all internal textures and paint gui.
    fn paint_and_update_textures(
        &mut self,
        clear_color: [f32; 4],
        clipped_primitives: &[egui::ClippedPrimitive],
        pixels_per_point: f32,
        textures_delta: &egui::TexturesDelta,
    ) -> Result<(), JsValue>;

    /// Destroy all resources.
    fn destroy(&mut self);
}
