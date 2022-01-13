use crate::{image::ImageData, TextureId};
use ahash::AHashMap;

// ----------------------------------------------------------------------------

/// The data needed in order to allocate and free textures/images.
pub struct TextureManager {
    /// We allocate texture id:s linearly.
    next_id: u64,
    delta: TexturesDelta,
}

impl Default for TextureManager {
    fn default() -> Self {
        Self {
            next_id: 1, // reserve 0 for the font texture
            delta: Default::default(),
        }
    }
}

impl TextureManager {
    /// Allocate a new texture.
    pub fn alloc(&mut self, image: impl Into<ImageData>) -> TextureId {
        let id = TextureId::Epaint(self.next_id);
        self.next_id += 1;
        self.delta.set.insert(id, image.into());
        id
    }

    /// Assign a new image to an existing texture.
    pub fn set(&mut self, id: TextureId, image: impl Into<ImageData>) {
        self.delta.set.insert(id, image.into());
    }

    /// Free an existing texture.
    pub fn free(&mut self, id: TextureId) {
        self.delta.free.push(id);
    }

    /// Get changes since last frame, and reset it.
    pub fn take_delta(&mut self) -> TexturesDelta {
        std::mem::take(&mut self.delta)
    }
}

// ----------------------------------------------------------------------------

/// What has been allocated and freed during the last period.
///
/// These are commands given to the integration painter.
#[derive(Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[must_use = "The painter must take care of this"]
pub struct TexturesDelta {
    /// New or changed textures. Apply before rendering.
    pub set: AHashMap<TextureId, ImageData>,

    /// Texture ID:s to free after rendering.
    pub free: Vec<TextureId>,
}

impl TexturesDelta {
    pub fn append(&mut self, mut newer: TexturesDelta) {
        self.set.extend(newer.set.into_iter());
        self.free.append(&mut newer.free);
    }
}
