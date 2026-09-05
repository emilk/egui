use std::collections::BTreeMap;

use crate::text::{
    FontFamily,
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
/// first the fonts the [`FontProviders`] install up front (the configured fonts),
/// then fonts they discover on demand, then the `.notdef` glyph ("tofu") of the primary face.
#[derive(Debug)]
pub(crate) struct Family {
    name: FontFamily,

    /// Faces in priority order: the primary first, then the fallbacks.
    ///
    /// Configured fonts first, then any discovered by the [`FontProviders`].
    chain: Vec<FontFaceKey>,

    /// Cache: `char → which face in the fallback chain owns this char`.
    ///
    /// Location-independent (fallback choice depends only on charmap support,
    /// not on variation coordinates).
    face_cache: ahash::HashMap<char, FontFaceKey>,

    /// Lazily calculated: every supported char, and the names of the faces that have it.
    characters: Option<BTreeMap<char, Vec<String>>>,
}

impl Family {
    /// Install the fonts the providers want for `name` up front, in provider order.
    pub fn new(name: &FontFamily, faces: &mut FaceStore, providers: &FontProviders) -> Self {
        let mut chain: Vec<FontFaceKey> = Vec::new();
        for insert in providers.fonts_for_family(name) {
            match faces.install(&insert.name, &insert.data) {
                Ok(key) => {
                    if !chain.contains(&key) {
                        chain.push(key);
                    }
                }
                Err(err) => {
                    panic!("Error parsing {:?} TTF/OTF font file: {err}", insert.name);
                }
            }
        }
        if chain.is_empty() {
            log::error!("No font provider has any font for FontFamily::{name:?}");
        }

        Self {
            name: name.clone(),
            chain,
            face_cache: Default::default(),
            characters: None,
        }
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

    /// Find which face in the fallback chain owns `c`.
    ///
    /// Location-independent: fallback choice depends only on charmap support.
    /// Asks the [`FontProviders`] when no face has `c`,
    /// and falls back to the primary face (whose `.notdef` glyph we then render)
    /// when they have no font for it either.
    #[inline]
    pub fn resolve(
        &mut self,
        faces: &mut FaceStore,
        providers: &mut FontProviders,
        c: char,
    ) -> FontFaceKey {
        let mut utf8 = [0_u8; 4];
        let cluster = c.encode_utf8(&mut utf8);
        self.resolve_cluster(faces, providers, cluster)
    }

    /// Like [`Self::resolve`] for the first char of `cluster`,
    /// but lets the [`FontProviders`] see the whole grapheme cluster.
    #[inline]
    pub fn resolve_cluster(
        &mut self,
        faces: &mut FaceStore,
        providers: &mut FontProviders,
        cluster: &str,
    ) -> FontFaceKey {
        let Some(base_char) = cluster.chars().next() else {
            return self.notdef_face();
        };
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
            // All the installed fonts, i.e. the configured ones
            // and then the ones we have already discovered…
            .find_face_for_char(c, faces)
            // …and if none of them has the char, ask the providers for a new font:
            .or_else(|| self.discover(faces, providers, cluster))
            // No font has the char, so we will render the `.notdef` glyph ("tofu"):
            .unwrap_or_else(|| {
                log::debug!(
                    "No font for {c:?} (U+{:04X}) in {:?}: rendering the tofu glyph instead",
                    c as u32,
                    self.name
                );
                self.notdef_face()
            });
        self.face_cache.insert(c, font_key);
        font_key
    }

    /// The face whose `.notdef` glyph ("tofu") we render for chars no face has.
    #[inline]
    fn notdef_face(&self) -> FontFaceKey {
        self.primary().unwrap_or(FontFaceKey::INVALID)
    }

    /// Ask the providers for a new font with the first char of `cluster`, and append it to the chain.
    fn discover(
        &mut self,
        faces: &mut FaceStore,
        providers: &mut FontProviders,
        cluster: &str,
    ) -> Option<FontFaceKey> {
        let key = providers.discover(faces, &self.name, cluster)?;
        if !self.chain.contains(&key) {
            self.chain.push(key);
            self.characters = None;
        }
        Some(key)
    }

    /// The first face in the whole chain whose charmap supports `c`.
    fn find_face_for_char(&self, c: char, faces: &mut FaceStore) -> Option<FontFaceKey> {
        Self::find_face_for_char_in(&self.chain, c, faces)
    }

    /// The first of `chain` whose charmap supports `c`.
    ///
    /// Does not touch [`Self::face_cache`].
    fn find_face_for_char_in(
        chain: &[FontFaceKey],
        c: char,
        faces: &mut FaceStore,
    ) -> Option<FontFaceKey> {
        chain.iter().copied().find(|&key| {
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
