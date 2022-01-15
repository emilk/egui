use epaint::{
    emath::NumExt,
    mutex::{Arc, Mutex},
    ImageData, TextureId,
};

/// Used to show images in egui.
///
/// To show an image in egui, allocate a texture with
/// [`crate::Context::load_texture`] and store the [`TextureHandle`].
/// You can then pass it to e.g. [`crate::Ui::image`].
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

impl PartialEq for TextureHandle {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TextureHandle {}

impl std::hash::Hash for TextureHandle {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl TextureHandle {
    pub(crate) fn new(tex_mngr: Arc<Mutex<epaint::TextureManager>>, id: TextureId) -> Self {
        Self { tex_mngr, id }
    }

    #[inline]
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

    /// width / height
    pub fn aspect_ratio(&self) -> f32 {
        let [w, h] = self.size();
        w as f32 / h.at_least(1) as f32
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
