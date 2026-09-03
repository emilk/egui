use std::collections::BTreeMap;

use crate::text::{
    FontDefinitions, FontFamily,
    face_store::{FaceStore, FontFaceKey},
    font_provider::FontProviders,
};

/// Index of a [`Family`] in `FontsImpl`.
///
/// Cheaper to pass around and look up than a [`FontFamily`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FamilyKey(pub(crate) usize);

/// The fallback chain of one [`FontFamily`], and the caches for resolving chars against it.
///
/// This is the only place that decides which face renders a given char:
/// first the fonts from [`FontDefinitions`], then fonts found by the [`FontProviders`],
/// then the replacement glyph.
#[derive(Debug)]
pub(crate) struct Family {
    name: FontFamily,

    /// Faces in priority order: the primary first, then the fallbacks.
    ///
    /// Fonts from [`FontDefinitions`] first, then any found by the [`FontProviders`].
    chain: Vec<FontFaceKey>,

    /// The face used when no face in [`Self::chain`] supports a char.
    replacement_face_key: FontFaceKey,

    /// The char that [`Self::replacement_face_key`] actually contains.
    ///
    /// When the user asks about a char that no fallback face supports we
    /// render this char in its place.
    replacement_char: char,

    /// Cache: `char → which face in the fallback chain owns this char`.
    ///
    /// Location-independent (fallback choice depends only on charmap support,
    /// not on variation coordinates).
    face_cache: ahash::HashMap<char, FontFaceKey>,

    /// Lazily calculated: every supported char, and the names of the faces that have it.
    characters: Option<BTreeMap<char, Vec<String>>>,
}

impl Family {
    const PRIMARY_REPLACEMENT_CHAR: char = '◻'; // white medium square
    const FALLBACK_REPLACEMENT_CHAR: char = '?'; // fallback for the fallback

    /// Look up the fallback chain of `name` in the definitions,
    /// followed by the fonts the providers have already found for it.
    ///
    /// Panics if the family or one of its fonts is missing from the definitions.
    pub fn new(
        name: &FontFamily,
        definitions: &FontDefinitions,
        faces: &mut FaceStore,
        providers: &FontProviders,
    ) -> Self {
        let font_names = definitions
            .families
            .get(name)
            .unwrap_or_else(|| panic!("FontFamily::{name:?} is not bound to any fonts"));

        let mut chain: Vec<FontFaceKey> = font_names
            .iter()
            .map(|font_name| {
                faces.key_by_name(font_name).unwrap_or_else(|| {
                    let available: Vec<&str> = faces.iter().map(|(_, face)| face.name()).collect();
                    panic!("No font data found for {font_name:?}. Installed fonts: {available:?}")
                })
            })
            .collect();

        // Fonts found by the providers come after the ones in the definitions:
        for key in providers.provided_keys_for(name, faces) {
            if !chain.contains(&key) {
                chain.push(key);
            }
        }

        let mut slf = Self {
            name: name.clone(),
            chain,
            replacement_face_key: FontFaceKey::INVALID,
            replacement_char: Self::PRIMARY_REPLACEMENT_CHAR,
            face_cache: Default::default(),
            characters: None,
        };

        if !slf.chain.is_empty() {
            let (replacement_face_key, replacement_char) = slf
                .find_face_for_char(Self::PRIMARY_REPLACEMENT_CHAR, faces)
                .map(|key| (key, Self::PRIMARY_REPLACEMENT_CHAR))
                .or_else(|| {
                    slf.find_face_for_char(Self::FALLBACK_REPLACEMENT_CHAR, faces)
                        .map(|key| (key, Self::FALLBACK_REPLACEMENT_CHAR))
                })
                .unwrap_or_else(|| {
                    log::warn!(
                        "Failed to find replacement characters {:?} or {:?}. Will use empty glyph.",
                        Self::PRIMARY_REPLACEMENT_CHAR,
                        Self::FALLBACK_REPLACEMENT_CHAR
                    );
                    (FontFaceKey::INVALID, Self::PRIMARY_REPLACEMENT_CHAR)
                });
            slf.replacement_face_key = replacement_face_key;
            slf.replacement_char = replacement_char;
        }

        slf
    }

    #[inline]
    pub fn name(&self) -> &FontFamily {
        &self.name
    }

    /// The primary face, if any.
    #[inline]
    pub fn primary(&self) -> Option<FontFaceKey> {
        self.chain.first().copied()
    }

    /// The char rendered in place of chars no face in the chain has.
    #[inline]
    pub fn replacement_char(&self) -> char {
        self.replacement_char
    }

    /// Find which face in the fallback chain owns `c`.
    ///
    /// Location-independent: fallback choice depends only on charmap support.
    /// Asks the [`FontProviders`] when no face has `c`,
    /// and falls back to the replacement-glyph face when they have no font for it either.
    #[inline]
    pub fn resolve(
        &mut self,
        faces: &mut FaceStore,
        providers: &mut FontProviders,
        c: char,
    ) -> FontFaceKey {
        if let Some(font_key) = self.face_cache.get(&c) {
            return *font_key;
        }
        let mut utf8 = [0_u8; 4];
        let cluster = c.encode_utf8(&mut utf8);
        self.resolve_slow(faces, providers, cluster, c)
    }

    /// Like [`Self::resolve`] for the first char of `cluster`,
    /// but lets the [`FontProviders`] see the whole grapheme cluster.
    #[inline]
    pub fn resolve_cluster(
        &mut self,
        faces: &mut FaceStore,
        providers: &mut FontProviders,
        cluster: &str,
        base_char: char,
    ) -> FontFaceKey {
        if let Some(font_key) = self.face_cache.get(&base_char) {
            return *font_key;
        }
        self.resolve_slow(faces, providers, cluster, base_char)
    }

    #[cold]
    fn resolve_slow(
        &mut self,
        faces: &mut FaceStore,
        providers: &mut FontProviders,
        cluster: &str,
        c: char,
    ) -> FontFaceKey {
        let font_key = self
            .find_face_for_char(c, faces)
            .or_else(|| {
                let key = providers.provide(faces, &self.name, cluster, c)?;
                if !self.chain.contains(&key) {
                    self.chain.push(key);
                    self.characters = None;
                }
                Some(key)
            })
            .unwrap_or(self.replacement_face_key);
        self.face_cache.insert(c, font_key);
        font_key
    }

    /// Walk the fallback chain and return the first face whose charmap supports `c`.
    ///
    /// Does not touch [`Self::face_cache`].
    fn find_face_for_char(&self, c: char, faces: &mut FaceStore) -> Option<FontFaceKey> {
        self.chain.iter().copied().find(|&key| {
            faces
                .get_mut(key)
                .is_some_and(|face| face.glyph_id_resolution(c).is_some())
        })
    }

    /// All supported characters, and in which faces they are available.
    pub fn characters(&mut self, faces: &FaceStore) -> &BTreeMap<char, Vec<String>> {
        self.characters.get_or_insert_with(|| {
            let mut characters: BTreeMap<char, Vec<String>> = Default::default();
            for key in &self.chain {
                let Some(face) = faces.get(*key) else {
                    continue;
                };
                for chr in face.characters() {
                    characters
                        .entry(chr)
                        .or_default()
                        .push(face.name().to_owned());
                }
            }
            characters
        })
    }
}
