use egui::{Event, UserData};
use wasm_bindgen::JsValue;

/// Renderer for a browser canvas.
/// As of writing we're not allowing to decide on the painter at runtime,
/// therefore this trait is merely there for specifying and documenting the interface.
pub(crate) trait WebPainter {
    // Create a new web painter targeting a given canvas.
    // fn new(canvas: HtmlCanvasElement, options: &WebOptions) -> Result<Self, String>
    // where
    //     Self: Sized;

    /// Reference to the canvas in use.
    fn canvas(&self) -> &web_sys::HtmlCanvasElement;

    /// Maximum size of a texture in one direction.
    fn max_texture_side(&self) -> usize;

    /// Update all internal textures and paint gui.
    /// When `capture` isn't empty, the rendered screen should be captured.
    /// Once the screenshot is ready, the screenshot should be returned via [`Self::handle_screenshots`].
    fn paint_and_update_textures(
        &mut self,
        clear_color: [f32; 4],
        clipped_primitives: &[egui::ClippedPrimitive],
        pixels_per_point: f32,
        textures_delta: &egui::TexturesDelta,
        capture: Vec<UserData>,
    ) -> Result<(), JsValue>;

    fn handle_screenshots(&mut self, events: &mut Vec<Event>);

    /// Destroy all resources.
    fn destroy(&mut self);
}
