use wasm_bindgen::prelude::JsValue;

pub trait Painter {
    fn as_tex_allocator(&mut self) -> &mut dyn epi::TextureAllocator;

    fn debug_info(&self) -> String;

    /// id of the canvas html element containing the rendering
    fn canvas_id(&self) -> &str;

    fn upload_egui_texture(&mut self, texture: &egui::Texture);

    fn clear(&mut self, lear_color: egui::Rgba);

    fn paint_jobs(&mut self, jobs: egui::PaintJobs, pixels_per_point: f32) -> Result<(), JsValue>;
}
