use std::sync::Arc;

use crate::{
    emath::NumExt, mutex::RwLock, textures::TextureOptions, ImageData, ImageDelta, TextureId,
    TextureManager,
};

/// Used to paint images.
///
/// An _image_ is pixels stored in RAM, and represented using [`ImageData`].
/// Before you can paint it however, you need to convert it to a _texture_.
///
/// If you are using egui, use `egui::Context::load_texture`.
///
/// The [`TextureHandle`] can be cloned cheaply.
/// When the last [`TextureHandle`] for specific texture is dropped, the texture is freed.
///
/// See also [`TextureManager`].
#[must_use]
pub struct TextureHandle {
    tex_mngr: Arc<RwLock<TextureManager>>,
    id: TextureId,
}

impl Drop for TextureHandle {
    fn drop(&mut self) {
        self.tex_mngr.write().free(self.id);
    }
}

impl Clone for TextureHandle {
    fn clone(&self) -> Self {
        self.tex_mngr.write().retain(self.id);
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
    /// If you are using egui, use `egui::Context::load_texture` instead.
    pub fn new(tex_mngr: Arc<RwLock<TextureManager>>, id: TextureId) -> Self {
        Self { tex_mngr, id }
    }

    #[inline]
    pub fn id(&self) -> TextureId {
        self.id
    }

    /// Assign a new image to an existing texture.
    pub fn set(&mut self, image: impl Into<ImageData>, options: TextureOptions) {
        self.tex_mngr
            .write()
            .set(self.id, ImageDelta::full(image.into(), options));
    }

    /// Assign a new image to a subregion of the whole texture.
    pub fn set_partial(
        &mut self,
        pos: [usize; 2],
        image: impl Into<ImageData>,
        options: TextureOptions,
    ) {
        self.tex_mngr
            .write()
            .set(self.id, ImageDelta::partial(pos, image.into(), options));
    }

    /// width x height
    pub fn size(&self) -> [usize; 2] {
        self.tex_mngr.read().meta(self.id).unwrap().size
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
        self.tex_mngr.read().meta(self.id).unwrap().name.clone()
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
