use crate::{ImageData, ImageDelta, TextureId};
use ahash::AHashMap;

// ----------------------------------------------------------------------------

/// Low-level manager for allocating textures.
///
/// Communicates with the painting subsystem using [`Self::take_delta`].
#[derive(Default)]
pub struct TextureManager {
    /// We allocate texture id:s linearly.
    next_id: u64,
    /// Information about currently allocated textures.
    metas: AHashMap<TextureId, TextureMeta>,
    delta: TexturesDelta,
}

impl TextureManager {
    /// Allocate a new texture.
    ///
    /// The given name can be useful for later debugging.
    ///
    /// The returned [`TextureId`] will be [`TextureId::Managed`], with an index
    /// starting from zero and increasing with each call to [`Self::alloc`].
    ///
    /// The first texture you allocate will be `TextureId::Managed(0) == TexureId::default()` and
    /// MUST have a white pixel at (0,0) ([`crate::WHITE_UV`]).
    ///
    /// The texture is given a retain-count of `1`, requiring one call to [`Self::free`] to free it.
    pub fn alloc(&mut self, name: String, image: ImageData) -> TextureId {
        let id = TextureId::Managed(self.next_id);
        self.next_id += 1;

        self.metas.entry(id).or_insert_with(|| TextureMeta {
            name,
            size: image.size(),
            bytes_per_pixel: image.bytes_per_pixel(),
            retain_count: 1,
        });

        self.delta.set.insert(id, ImageDelta::full(image));
        id
    }

    /// Assign a new image to an existing texture,
    /// or update a region of it.
    pub fn set(&mut self, id: TextureId, delta: ImageDelta) {
        if let Some(meta) = self.metas.get_mut(&id) {
            if delta.is_whole() {
                meta.size = delta.image.size();
                meta.bytes_per_pixel = delta.image.bytes_per_pixel();
            }
            self.delta.set.insert(id, delta);
        } else {
            crate::epaint_assert!(
                false,
                "Tried setting texture {:?} which is not allocated",
                id
            );
        }
    }

    /// Free an existing texture.
    pub fn free(&mut self, id: TextureId) {
        if let std::collections::hash_map::Entry::Occupied(mut entry) = self.metas.entry(id) {
            let meta = entry.get_mut();
            meta.retain_count -= 1;
            if meta.retain_count == 0 {
                entry.remove();
                self.delta.free.push(id);
            }
        } else {
            crate::epaint_assert!(
                false,
                "Tried freeing texture {:?} which is not allocated",
                id
            );
        }
    }

    /// Increase the retain-count of the given texture.
    ///
    /// For each time you call [`Self::retain`] you must call [`Self::free`] on additional time.
    pub fn retain(&mut self, id: TextureId) {
        if let Some(meta) = self.metas.get_mut(&id) {
            meta.retain_count += 1;
        } else {
            crate::epaint_assert!(
                false,
                "Tried retaining texture {:?} which is not allocated",
                id
            );
        }
    }

    /// Take and reset changes since last frame.
    ///
    /// These should be applied to the painting subsystem each frame.
    pub fn take_delta(&mut self) -> TexturesDelta {
        std::mem::take(&mut self.delta)
    }

    /// Get meta-data about a specific texture.
    pub fn meta(&self, id: TextureId) -> Option<&TextureMeta> {
        self.metas.get(&id)
    }

    /// Get meta-data about all allocated textures in some arbitrary order.
    pub fn allocated(&self) -> impl ExactSizeIterator<Item = (&TextureId, &TextureMeta)> {
        self.metas.iter()
    }

    /// Total number of allocated textures.
    pub fn num_allocated(&self) -> usize {
        self.metas.len()
    }
}

/// Meta-data about an allocated texture.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureMeta {
    /// A human-readable name useful for debugging.
    pub name: String,

    /// width x height
    pub size: [usize; 2],

    /// 4 or 1
    pub bytes_per_pixel: usize,

    /// Free when this reaches zero.
    pub retain_count: usize,
}

impl TextureMeta {
    /// Size in bytes.
    /// width x height x [`Self::bytes_per_pixel`].
    pub fn bytes_used(&self) -> usize {
        self.size[0] * self.size[1] * self.bytes_per_pixel
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
    /// New or changed textures. Apply before painting.
    pub set: AHashMap<TextureId, ImageDelta>,

    /// Textures to free after painting.
    pub free: Vec<TextureId>,
}

impl TexturesDelta {
    pub fn is_empty(&self) -> bool {
        self.set.is_empty() && self.free.is_empty()
    }

    pub fn append(&mut self, mut newer: TexturesDelta) {
        self.set.extend(newer.set.into_iter());
        self.free.append(&mut newer.free);
    }

    pub fn clear(&mut self) {
        self.set.clear();
        self.free.clear();
    }
}

impl std::fmt::Debug for TexturesDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("TexturesDelta");
        if !self.set.is_empty() {
            let mut string = String::new();
            for (tex_id, delta) in &self.set {
                let size = delta.image.size();
                if let Some(pos) = delta.pos {
                    string += &format!(
                        "{:?} partial ([{} {}] - [{} {}]), ",
                        tex_id,
                        pos[0],
                        pos[1],
                        pos[0] + size[0],
                        pos[1] + size[1]
                    );
                } else {
                    string += &format!("{:?} full {}x{}, ", tex_id, size[0], size[1]);
                }
            }
            debug_struct.field("set", &string);
        }
        if !self.free.is_empty() {
            debug_struct.field("free", &self.free);
        }
        debug_struct.finish()
    }
}
