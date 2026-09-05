use core::sync::atomic::{AtomicUsize, Ordering};

use nohash_hasher::IntMap;

use crate::text::{FontData, FontDefinitions, TextOptions, font::FontFace};

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
/// This is the one place fonts get parsed, whether they come from
/// [`FontDefinitions`] or are found later (e.g. by a font provider).
pub(crate) struct FaceStore {
    options: TextOptions,
    by_key: IntMap<FontFaceKey, FontFace>,
    by_name: ahash::HashMap<String, FontFaceKey>,
}

impl FaceStore {
    /// Parse every font in the definitions.
    ///
    /// Panics if a font fails to parse.
    pub fn new(options: TextOptions, definitions: &FontDefinitions) -> Self {
        let mut slf = Self {
            options,
            by_key: Default::default(),
            by_name: Default::default(),
        };
        for (name, font_data) in &definitions.font_data {
            slf.install(name, font_data)
                .unwrap_or_else(|err| panic!("Error parsing {name:?} TTF/OTF font file: {err}"));
        }
        slf
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
            font_data.blob(),
            font_data.index,
            font_data.tweak.clone(),
        )?;
        let key = FontFaceKey::new();
        self.by_key.insert(key, font_face);
        self.by_name.insert(name.to_owned(), key);
        Ok(key)
    }

    #[inline]
    pub fn key_by_name(&self, name: &str) -> Option<FontFaceKey> {
        self.by_name.get(name).copied()
    }

    #[inline]
    pub fn get(&self, key: FontFaceKey) -> Option<&FontFace> {
        self.by_key.get(&key)
    }

    #[inline]
    pub fn get_mut(&mut self, key: FontFaceKey) -> Option<&mut FontFace> {
        self.by_key.get_mut(&key)
    }

    /// All installed faces, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = (FontFaceKey, &FontFace)> {
        self.by_key.iter().map(|(key, face)| (*key, face))
    }
}
