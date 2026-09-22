use core::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use nohash_hasher::IntMap;

use crate::text::{FontData, TextOptions, font_face::FontFace};

/// Unique ID for looking up a single font face/file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FontFaceKey(u64);

impl FontFaceKey {
    pub const INVALID: Self = Self(0);

    fn new() -> Self {
        static KEY_COUNTER: AtomicUsize = AtomicUsize::new(1);
        Self(crate::util::hash(
            KEY_COUNTER.fetch_add(1, Ordering::Relaxed),
        ))
    }
}

// Safe, because we hash the value in the constructor.
impl nohash_hasher::IsEnabled for FontFaceKey {}

// ----------------------------------------------------------------------------

/// Every parsed [`FontFace`], by key and by name.
///
/// This is the one place fonts get parsed, whether they are configured up front
/// or discovered later by a font provider.
pub(crate) struct FaceStore {
    options: TextOptions,
    by_key: IntMap<FontFaceKey, FontFace>,
    by_name: ahash::HashMap<String, FontFaceKey>,
}

impl FaceStore {
    pub fn new(options: TextOptions) -> Self {
        Self {
            options,
            by_key: Default::default(),
            by_name: Default::default(),
        }
    }

    /// Apply new [`TextOptions`] (hinting, sub-pixel binning) to every face,
    /// and to faces installed later.
    pub fn set_options(&mut self, options: TextOptions) {
        self.options = options;
        #[expect(clippy::iter_over_hash_type, reason = "Order does not matter here")]
        for face in self.by_key.values_mut() {
            face.set_options(options);
        }
    }

    /// Parse a font and add it, unless a font with the same name is already installed.
    pub fn install(
        &mut self,
        name: &str,
        font_data: &FontData,
    ) -> Result<FontFaceKey, Box<dyn core::error::Error>> {
        if let Some(key) = self.by_name.get(name) {
            return Ok(*key);
        }
        let font_face = FontFace::new(
            self.options,
            name.to_owned(),
            Arc::clone(&font_data.font),
            font_data.index,
            font_data.tweak.clone(),
        )?;
        let key = FontFaceKey::new();
        self.by_key.insert(key, font_face);
        self.by_name.insert(name.to_owned(), key);
        Ok(key)
    }

    #[inline]
    pub fn get(&self, key: FontFaceKey) -> Option<&FontFace> {
        self.by_key.get(&key)
    }

    #[inline]
    pub fn get_mut(&mut self, key: FontFaceKey) -> Option<&mut FontFace> {
        self.by_key.get_mut(&key)
    }
}
