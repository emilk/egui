use epaint::{
    mutex::{Arc, Mutex},
    ImageData, TextureId,
};

/// Used to show images in egui.
///
/// To show an image in egui, allocate a texture with
/// [`crate::Context::alloc_texture`] and store the [`TextureHandle`].
/// You can then pass it to e.g. [`Ui::image`].
///
/// The [`TextureHandle`] can be cloned cheaply.
/// When the last [`TextureHandle`] for specific texture is dropped, the texture is freed.
#[must_use]
pub struct TextureHandle {
    tex_mngr: Arc<Mutex<epaint::TextureManager>>,
    id: TextureId,
}

impl Drop for TextureHandle {
    fn drop(&mut self) {
        self.tex_mngr.lock().free(self.id);
    }
}

impl Clone for TextureHandle {
    fn clone(&self) -> Self {
        self.tex_mngr.lock().retain(self.id);
        Self {
            tex_mngr: self.tex_mngr.clone(),
            id: self.id,
        }
    }
}

impl TextureHandle {
    pub(crate) fn new(tex_mngr: Arc<Mutex<epaint::TextureManager>>, id: TextureId) -> Self {
        Self { tex_mngr, id }
    }

    pub fn id(&self) -> TextureId {
        self.id
    }

    /// Assign a new image to an existing texture.
    pub fn set(&mut self, image: impl Into<ImageData>) {
        self.tex_mngr.lock().set(self.id, image.into());
    }

    /// width x height
    pub fn size(&self) -> [usize; 2] {
        self.tex_mngr.lock().meta(self.id).unwrap().size
    }

    /// width x height
    pub fn size_vec2(&self) -> crate::Vec2 {
        let [w, h] = self.size();
        crate::Vec2::new(w as f32, h as f32)
    }

    /// Debug-name.
    pub fn name(&self) -> String {
        self.tex_mngr.lock().meta(self.id).unwrap().name.clone()
    }
}

impl From<&TextureHandle> for TextureId {
    #[inline(always)]
    fn from(handle: &TextureHandle) -> Self {
        handle.id()
    }
}

impl From<&mut TextureHandle> for TextureId {
    #[inline(always)]
    fn from(handle: &mut TextureHandle) -> Self {
        handle.id()
    }
}
