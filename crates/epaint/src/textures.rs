use crate::{ImageData, ImageDelta, TextureId};

// ----------------------------------------------------------------------------

/// Low-level manager for allocating textures.
///
/// Communicates with the painting subsystem using [`Self::take_delta`].
#[derive(Default)]
pub struct TextureManager {
    /// We allocate texture id:s linearly.
    next_id: u64,

    /// Information about currently allocated textures.
    metas: ahash::HashMap<TextureId, TextureMeta>,

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
    /// The first texture you allocate will be `TextureId::Managed(0) == TextureId::default()` and
    /// MUST have a white pixel at (0,0) ([`crate::WHITE_UV`]).
    ///
    /// The texture is given a retain-count of `1`, requiring one call to [`Self::free`] to free it.
    pub fn alloc(&mut self, name: String, image: ImageData, options: TextureOptions) -> TextureId {
        let id = TextureId::Managed(self.next_id);
        self.next_id += 1;

        self.metas.entry(id).or_insert_with(|| TextureMeta {
            name,
            size: image.size(),
            bytes_per_pixel: image.bytes_per_pixel(),
            retain_count: 1,
            options,
        });

        self.delta.set.push((id, ImageDelta::full(image, options)));
        id
    }

    /// Assign a new image to an existing texture,
    /// or update a region of it.
    pub fn set(&mut self, id: TextureId, delta: ImageDelta) {
        if let Some(meta) = self.metas.get_mut(&id) {
            if let Some(pos) = delta.pos {
                debug_assert!(
                    pos[0] + delta.image.width() <= meta.size[0]
                        && pos[1] + delta.image.height() <= meta.size[1],
                    "Partial texture update is outside the bounds of texture {id:?}",
                );
            } else {
                // whole update
                meta.size = delta.image.size();
                meta.bytes_per_pixel = delta.image.bytes_per_pixel();
                // since we update the whole image, we can discard all old enqueued deltas
                self.delta.set.retain(|(x, _)| x != &id);
            }
            self.delta.set.push((id, delta));
        } else {
            debug_assert!(false, "Tried setting texture {id:?} which is not allocated");
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
            debug_assert!(false, "Tried freeing texture {id:?} which is not allocated");
        }
    }

    /// Increase the retain-count of the given texture.
    ///
    /// For each time you call [`Self::retain`] you must call [`Self::free`] on additional time.
    pub fn retain(&mut self, id: TextureId) {
        if let Some(meta) = self.metas.get_mut(&id) {
            meta.retain_count += 1;
        } else {
            debug_assert!(
                false,
                "Tried retaining texture {id:?} which is not allocated",
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureMeta {
    /// A human-readable name useful for debugging.
    pub name: String,

    /// width x height
    pub size: [usize; 2],

    /// 4 or 1
    pub bytes_per_pixel: usize,

    /// Free when this reaches zero.
    pub retain_count: usize,

    /// The texture filtering mode to use when rendering.
    pub options: TextureOptions,
}

impl TextureMeta {
    /// Size in bytes.
    /// width x height x [`Self::bytes_per_pixel`].
    pub fn bytes_used(&self) -> usize {
        self.size[0] * self.size[1] * self.bytes_per_pixel
    }
}

// ----------------------------------------------------------------------------

/// How the texture texels are filtered.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextureOptions {
    /// How to filter when magnifying (when texels are larger than pixels).
    pub magnification: TextureFilter,

    /// How to filter when minifying (when texels are smaller than pixels).
    pub minification: TextureFilter,

    /// How to wrap the texture when the texture coordinates are outside the [0, 1] range.
    pub wrap_mode: TextureWrapMode,
}

impl TextureOptions {
    /// Linear magnification and minification.
    pub const LINEAR: Self = Self {
        magnification: TextureFilter::Linear,
        minification: TextureFilter::Linear,
        wrap_mode: TextureWrapMode::ClampToEdge,
    };

    /// Nearest magnification and minification.
    pub const NEAREST: Self = Self {
        magnification: TextureFilter::Nearest,
        minification: TextureFilter::Nearest,
        wrap_mode: TextureWrapMode::ClampToEdge,
    };

    /// Linear magnification and minification, but with the texture repeated.
    pub const LINEAR_REPEAT: Self = Self {
        magnification: TextureFilter::Linear,
        minification: TextureFilter::Linear,
        wrap_mode: TextureWrapMode::Repeat,
    };

    /// Linear magnification and minification, but with the texture mirrored and repeated.
    pub const LINEAR_MIRRORED_REPEAT: Self = Self {
        magnification: TextureFilter::Linear,
        minification: TextureFilter::Linear,
        wrap_mode: TextureWrapMode::MirroredRepeat,
    };

    /// Nearest magnification and minification, but with the texture repeated.
    pub const NEAREST_REPEAT: Self = Self {
        magnification: TextureFilter::Nearest,
        minification: TextureFilter::Nearest,
        wrap_mode: TextureWrapMode::Repeat,
    };

    /// Nearest magnification and minification, but with the texture mirrored and repeated.
    pub const NEAREST_MIRRORED_REPEAT: Self = Self {
        magnification: TextureFilter::Nearest,
        minification: TextureFilter::Nearest,
        wrap_mode: TextureWrapMode::MirroredRepeat,
    };
}

impl Default for TextureOptions {
    /// The default is linear for both magnification and minification.
    fn default() -> Self {
        Self::LINEAR
    }
}

/// How the texture texels are filtered.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TextureFilter {
    /// Show the nearest pixel value.
    ///
    /// When zooming in you will get sharp, square pixels/texels.
    /// When zooming out you will get a very crisp (and aliased) look.
    Nearest,

    /// Linearly interpolate the nearest neighbors, creating a smoother look when zooming in and out.
    Linear,
}

/// Defines how textures are wrapped around objects when texture coordinates fall outside the [0, 1] range.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TextureWrapMode {
    /// Stretches the edge pixels to fill beyond the texture's bounds.
    ///
    /// This is what you want to use for a normal image in a GUI.
    #[default]
    ClampToEdge,

    /// Tiles the texture across the surface, repeating it horizontally and vertically.
    Repeat,

    /// Mirrors the texture with each repetition, creating symmetrical tiling.
    MirroredRepeat,
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
    pub set: Vec<(TextureId, ImageDelta)>,

    /// Textures to free after painting.
    pub free: Vec<TextureId>,
}

impl TexturesDelta {
    pub fn is_empty(&self) -> bool {
        self.set.is_empty() && self.free.is_empty()
    }

    pub fn append(&mut self, mut newer: Self) {
        self.set.extend(newer.set);
        self.free.append(&mut newer.free);
    }

    pub fn clear(&mut self) {
        self.set.clear();
        self.free.clear();
    }
}

impl std::fmt::Debug for TexturesDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write as _;

        let mut debug_struct = f.debug_struct("TexturesDelta");
        if !self.set.is_empty() {
            let mut string = String::new();
            for (tex_id, delta) in &self.set {
                let size = delta.image.size();
                if let Some(pos) = delta.pos {
                    write!(
                        string,
                        "{:?} partial ([{} {}] - [{} {}]), ",
                        tex_id,
                        pos[0],
                        pos[1],
                        pos[0] + size[0],
                        pos[1] + size[1]
                    )
                    .ok();
                } else {
                    write!(string, "{:?} full {}x{}, ", tex_id, size[0], size[1]).ok();
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
