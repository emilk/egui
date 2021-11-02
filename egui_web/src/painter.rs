use wasm_bindgen::prelude::JsValue;

pub trait Painter {
    fn as_tex_allocator(&mut self) -> &mut dyn epi::TextureAllocator;

    fn debug_info(&self) -> String;

    /// id of the canvas html element containing the rendering
    fn canvas_id(&self) -> &str;

    fn upload_egui_texture(&mut self, texture: &egui::Texture);

    fn clear(&mut self, clear_color: egui::Rgba);

    fn paint_meshes(
        &mut self,
        clipped_meshes: Vec<egui::ClippedMesh>,
        pixels_per_point: f32,
    ) -> Result<(), JsValue>;

    fn name(&self) -> &'static str;
}
