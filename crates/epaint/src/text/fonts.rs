use std::{collections::BTreeMap, sync::Arc};

use crate::{
    Color32, TextureAtlas,
    text::{
        FontDefinitions, FontFamily, FontId, Galley, GlyphRasterizer, GlyphSource,
        GlyphSourcePreference, LayoutJob, TextOptions, VariationCoords,
        face_store::{FaceStore, FontFaceKey},
        font::Font,
        galley_cache::GalleyCache,
        glyph_atlas::GlyphAtlas,
        glyph_rasterizer::default_glyph_source,
    },
};

// ----------------------------------------------------------------------------

/// Maximum width or height of a glyph rasterized into the font atlas.
///
/// Must not exceed the minimum width of the [`TextureAtlas`] (1024).
pub const MAX_GLYPH_SIZE: usize = 1024;

// ----------------------------------------------------------------------------

/// Cached data for working with a font family (e.g. doing character lookups).
#[derive(Debug)]
pub(super) struct CachedFamily {
    pub fonts: Vec<FontFaceKey>,

    /// Lazily calculated.
    pub characters: Option<BTreeMap<char, Vec<String>>>,

    /// The face used when no face in [`Self::fonts`] supports a char.
    pub replacement_face_key: FontFaceKey,

    /// The char that [`Self::replacement_face_key`] actually contains.
    ///
    /// When the user asks about a char that no fallback face supports we
    /// render this char in its place.
    pub replacement_char: char,

    /// Cache: `char → which face in the fallback chain owns this char`.
    ///
    /// Location-independent (fallback choice depends only on charmap support,
    /// not on variation coordinates).
    pub face_cache: ahash::HashMap<char, FontFaceKey>,
}

impl CachedFamily {
    fn new(fonts: Vec<FontFaceKey>, faces: &mut FaceStore) -> Self {
        const PRIMARY_REPLACEMENT_CHAR: char = '◻'; // white medium square
        const FALLBACK_REPLACEMENT_CHAR: char = '?'; // fallback for the fallback

        if fonts.is_empty() {
            return Self {
                fonts,
                characters: None,
                replacement_face_key: FontFaceKey::INVALID,
                replacement_char: PRIMARY_REPLACEMENT_CHAR,
                face_cache: Default::default(),
            };
        }

        let mut slf = Self {
            fonts,
            characters: None,
            replacement_face_key: FontFaceKey::INVALID,
            replacement_char: PRIMARY_REPLACEMENT_CHAR,
            face_cache: Default::default(),
        };

        let (replacement_face_key, replacement_char) = slf
            .find_face_for_char(PRIMARY_REPLACEMENT_CHAR, faces)
            .map(|key| (key, PRIMARY_REPLACEMENT_CHAR))
            .or_else(|| {
                slf.find_face_for_char(FALLBACK_REPLACEMENT_CHAR, faces)
                    .map(|key| (key, FALLBACK_REPLACEMENT_CHAR))
            })
            .unwrap_or_else(|| {
                log::warn!(
                    "Failed to find replacement characters {PRIMARY_REPLACEMENT_CHAR:?} or {FALLBACK_REPLACEMENT_CHAR:?}. Will use empty glyph."
                );
                (FontFaceKey::INVALID, PRIMARY_REPLACEMENT_CHAR)
            });
        slf.replacement_face_key = replacement_face_key;
        slf.replacement_char = replacement_char;

        slf
    }

    /// Walk the fallback chain and return the first face whose charmap supports `c`.
    ///
    /// Pure — does not touch any cache. Callers that want memoisation should
    /// insert into [`Self::face_cache`] themselves.
    pub(crate) fn find_face_for_char(&self, c: char, faces: &mut FaceStore) -> Option<FontFaceKey> {
        for font_key in &self.fonts {
            let font_face = faces.get_mut(*font_key).expect("Nonexistent font ID");
            if font_face.glyph_id_resolution(c).is_some() {
                return Some(*font_key);
            }
        }
        None
    }
}

// ----------------------------------------------------------------------------

// ----------------------------------------------------------------------------

/// The collection of fonts used by `epaint`.
///
/// Required in order to paint text. Create one and reuse. Cheap to clone.
///
/// Each [`Fonts`] comes with a font atlas textures that needs to be used when painting.
///
/// If you are using `egui`, use `egui::Context::set_fonts` and `egui::Context::fonts`.
///
/// You need to call [`Self::begin_pass`] and [`Self::font_image_delta`] once every frame.
pub struct Fonts {
    pub fonts: FontsImpl,
    galley_cache: GalleyCache,
}

impl Fonts {
    /// Create a new [`Fonts`] for text layout.
    ///
    /// This call is expensive, so only create one [`Fonts`] and then reuse it.
    pub fn new(options: TextOptions, definitions: FontDefinitions) -> Self {
        Self {
            fonts: FontsImpl::new(options, definitions),
            galley_cache: Default::default(),
        }
    }

    /// Use this platform glyph rasterizer, e.g. the browser on web.
    ///
    /// See [`GlyphRasterizer`].
    #[inline]
    pub fn with_glyph_rasterizer(mut self, glyph_rasterizer: GlyphRasterizer) -> Self {
        self.set_glyph_rasterizer(Some(glyph_rasterizer));
        self
    }

    /// Use this platform glyph rasterizer, e.g. the browser on web.
    ///
    /// Pass `None` to only use the installed fonts.
    ///
    /// See [`GlyphRasterizer`].
    pub fn set_glyph_rasterizer(&mut self, glyph_rasterizer: Option<GlyphRasterizer>) {
        self.fonts.set_glyph_rasterizer(glyph_rasterizer);
        self.galley_cache = Default::default();
    }

    /// Decide where to look first for the glyphs of each grapheme cluster.
    ///
    /// See [`GlyphSourcePreference`].
    #[inline]
    pub fn with_glyph_source_preference(
        mut self,
        prefer: impl Fn(&str) -> GlyphSource + Send + Sync + 'static,
    ) -> Self {
        self.fonts.set_glyph_source_preference(prefer);
        self
    }

    /// Call at the start of each frame with the latest known [`TextOptions`].
    ///
    /// Call after painting the previous frame, but before using [`Fonts`] for the new frame.
    ///
    /// This function will react to changes in [`TextOptions`],
    /// as well as notice when the font atlas is getting full, and handle that.
    pub fn begin_pass(&mut self, options: TextOptions) {
        if self.fonts.options() != &options {
            // Hinting and other options are baked into each parsed face, so start over:
            let definitions = self.fonts.definitions.clone();
            let mut fonts = FontsImpl::new(options, definitions);
            fonts.set_glyph_rasterizer(self.fonts.glyph_rasterizer.take());
            fonts.glyph_source_preference = Arc::clone(&self.fonts.glyph_source_preference);
            self.fonts = fonts;
            self.galley_cache = Default::default();
        } else if 0.8 < self.fonts.glyphs.fill_ratio() {
            // The parsed faces are still fine; only the bitmaps need to go.
            self.fonts.glyphs.clear();
            self.galley_cache = Default::default(); // Galleys point into the old atlas.
        }

        self.galley_cache.flush_cache();
    }

    /// Call at the end of each frame (before painting) to get the change to the font texture since last call.
    pub fn font_image_delta(&mut self) -> Option<crate::ImageDelta> {
        self.fonts.glyphs.take_delta()
    }

    #[inline]
    pub fn options(&self) -> &TextOptions {
        self.texture_atlas().options()
    }

    #[inline]
    pub fn definitions(&self) -> &FontDefinitions {
        &self.fonts.definitions
    }

    /// The font atlas.
    /// Pass this to [`crate::Tessellator`].
    pub fn texture_atlas(&self) -> &TextureAtlas {
        self.fonts.glyphs.atlas()
    }

    /// The full font atlas image.
    #[inline]
    pub fn image(&self) -> crate::ColorImage {
        self.fonts.glyphs.atlas().image().clone()
    }

    /// Current size of the font image.
    /// Pass this to [`crate::Tessellator`].
    pub fn font_image_size(&self) -> [usize; 2] {
        self.fonts.glyphs.atlas().size()
    }

    /// Do the installed fonts have this glyph?
    ///
    /// This does not consult the [`GlyphRasterizer`], so it can return `false`
    /// for a character that would still render via the rasterizer (e.g. the browser on web).
    pub fn has_glyph(&mut self, font_id: &FontId, c: char) -> bool {
        self.fonts.font(&font_id.family).has_glyph(c)
    }

    /// Do the installed fonts have all the glyphs in this text?
    ///
    /// See [`Self::has_glyph`] for the caveat about the [`GlyphRasterizer`].
    pub fn has_glyphs(&mut self, font_id: &FontId, s: &str) -> bool {
        self.fonts.font(&font_id.family).has_glyphs(s)
    }

    pub fn num_galleys_in_cache(&self) -> usize {
        self.galley_cache.num_galleys_in_cache()
    }

    /// How full is the font atlas?
    ///
    /// This increases as new fonts and/or glyphs are used,
    /// but can also decrease in a call to [`Self::begin_pass`].
    pub fn font_atlas_fill_ratio(&self) -> f32 {
        self.fonts.glyphs.fill_ratio()
    }

    /// Returns a [`FontsView`] with the given `pixels_per_point` that can be used to do text layout.
    pub fn with_pixels_per_point(&mut self, pixels_per_point: f32) -> FontsView<'_> {
        FontsView {
            fonts: &mut self.fonts,
            galley_cache: &mut self.galley_cache,
            pixels_per_point,
        }
    }
}

// ----------------------------------------------------------------------------

/// The context's collection of fonts, with this context's `pixels_per_point`. This is what you use to do text layout.
pub struct FontsView<'a> {
    pub fonts: &'a mut FontsImpl,
    galley_cache: &'a mut GalleyCache,
    pixels_per_point: f32,
}

impl FontsView<'_> {
    #[inline]
    pub fn options(&self) -> &TextOptions {
        self.fonts.options()
    }

    #[inline]
    pub fn definitions(&self) -> &FontDefinitions {
        &self.fonts.definitions
    }

    /// The full font atlas image.
    #[inline]
    pub fn image(&self) -> crate::ColorImage {
        self.fonts.glyphs.atlas().image().clone()
    }

    /// Current size of the font image.
    /// Pass this to [`crate::Tessellator`].
    pub fn font_image_size(&self) -> [usize; 2] {
        self.fonts.glyphs.atlas().size()
    }

    /// Width of this character in points.
    ///
    /// If the font doesn't exist, this will return `0.0`.
    pub fn glyph_width(&mut self, font_id: &FontId, c: char) -> f32 {
        self.fonts
            .font(&font_id.family)
            .glyph_width(c, font_id.size)
    }

    /// Do the installed fonts have this glyph?
    ///
    /// This does not consult the [`GlyphRasterizer`], so it can return `false`
    /// for a character that would still render via the rasterizer (e.g. the browser on web).
    pub fn has_glyph(&mut self, font_id: &FontId, c: char) -> bool {
        self.fonts.font(&font_id.family).has_glyph(c)
    }

    /// Do the installed fonts have all the glyphs in this text?
    ///
    /// See [`Self::has_glyph`] for the caveat about the [`GlyphRasterizer`].
    pub fn has_glyphs(&mut self, font_id: &FontId, s: &str) -> bool {
        self.fonts.font(&font_id.family).has_glyphs(s)
    }

    /// Height of one row of text in points.
    ///
    /// Returns a value rounded to [`emath::GUI_ROUNDING`].
    #[inline]
    pub fn row_height(&mut self, font_id: &FontId) -> f32 {
        self.fonts
            .font(&font_id.family)
            .styled_metrics(
                self.pixels_per_point,
                font_id.size,
                // TODO(valadaptive): use font variation coords when calculating row height
                &VariationCoords::default(),
            )
            .row_height
    }

    /// List of all known font families.
    pub fn families(&self) -> Vec<FontFamily> {
        self.fonts.definitions.families.keys().cloned().collect()
    }

    /// Layout some text.
    ///
    /// This is the most advanced layout function.
    /// See also [`Self::layout`], [`Self::layout_no_wrap`] and
    /// [`Self::layout_delayed_color`].
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    #[inline]
    pub fn layout_job(&mut self, job: LayoutJob) -> Arc<Galley> {
        let allow_split_paragraphs = true; // Optimization for editing text with many paragraphs.
        self.galley_cache.layout(
            self.fonts,
            self.pixels_per_point,
            job,
            allow_split_paragraphs,
        )
    }

    pub fn num_galleys_in_cache(&self) -> usize {
        self.galley_cache.num_galleys_in_cache()
    }

    /// How full is the font atlas?
    ///
    /// This increases as new fonts and/or glyphs are used,
    /// but can also decrease in a call to [`Fonts::begin_pass`].
    pub fn font_atlas_fill_ratio(&self) -> f32 {
        self.fonts.glyphs.fill_ratio()
    }

    /// Will wrap text at the given width and line break at `\n`.
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    #[inline]
    pub fn layout(
        &mut self,
        text: String,
        font_id: FontId,
        color: Color32,
        wrap_width: f32,
    ) -> Arc<Galley> {
        let job = LayoutJob::simple(text, font_id, color, wrap_width);
        self.layout_job(job)
    }

    /// Will line break at `\n`.
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    #[inline]
    pub fn layout_no_wrap(&mut self, text: String, font_id: FontId, color: Color32) -> Arc<Galley> {
        let job = LayoutJob::simple(text, font_id, color, f32::INFINITY);
        self.layout_job(job)
    }

    /// Like [`Self::layout`], made for when you want to pick a color for the text later.
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    #[inline]
    pub fn layout_delayed_color(
        &mut self,
        text: String,
        font_id: FontId,
        wrap_width: f32,
    ) -> Arc<Galley> {
        self.layout(text, font_id, Color32::PLACEHOLDER, wrap_width)
    }
}

// ----------------------------------------------------------------------------

/// The collection of fonts used by `epaint`.
///
/// Required in order to paint text.
pub struct FontsImpl {
    definitions: FontDefinitions,
    glyphs: GlyphAtlas,
    faces: FaceStore,
    family_cache: ahash::HashMap<FontFamily, CachedFamily>,

    /// Recycled `harfrust` shaping buffer to avoid per-layout allocations.
    shape_buffer: Option<harfrust::UnicodeBuffer>,
    glyph_rasterizer: Option<GlyphRasterizer>,
    glyph_source_preference: GlyphSourcePreference,
}

impl FontsImpl {
    /// Create a new [`FontsImpl`] for text layout.
    /// This call is expensive, so only create one [`FontsImpl`] and then reuse it.
    pub fn new(options: TextOptions, definitions: FontDefinitions) -> Self {
        let faces = FaceStore::new(options, &definitions);

        Self {
            definitions,
            glyphs: GlyphAtlas::new(options),
            faces,
            family_cache: Default::default(),
            shape_buffer: Some(harfrust::UnicodeBuffer::new()),
            glyph_rasterizer: None,
            glyph_source_preference: Arc::new(default_glyph_source),
        }
    }

    /// Use this platform glyph rasterizer, e.g. the browser on web.
    ///
    /// See [`GlyphRasterizer`].
    #[inline]
    pub fn with_glyph_rasterizer(mut self, glyph_rasterizer: GlyphRasterizer) -> Self {
        self.set_glyph_rasterizer(Some(glyph_rasterizer));
        self
    }

    /// Use this platform glyph rasterizer, e.g. the browser on web.
    ///
    /// Pass `None` to only use the installed fonts.
    ///
    /// See [`GlyphRasterizer`].
    pub fn set_glyph_rasterizer(&mut self, glyph_rasterizer: Option<GlyphRasterizer>) {
        self.glyph_rasterizer = glyph_rasterizer;
        self.glyphs.clear_raster_glyphs();
    }

    /// Decide where to look first for the glyphs of each grapheme cluster.
    ///
    /// See [`GlyphSourcePreference`].
    pub fn set_glyph_source_preference(
        &mut self,
        prefer: impl Fn(&str) -> GlyphSource + Send + Sync + 'static,
    ) {
        self.glyph_source_preference = Arc::new(prefer);
    }

    pub fn options(&self) -> &TextOptions {
        self.glyphs.options()
    }

    /// Take the recycled shaping buffer (or create a new one if already taken).
    pub fn take_shape_buffer(&mut self) -> harfrust::UnicodeBuffer {
        self.shape_buffer.take().unwrap_or_default()
    }

    /// Return a shaping buffer for reuse.
    pub fn return_shape_buffer(&mut self, buffer: harfrust::UnicodeBuffer) {
        self.shape_buffer = Some(buffer);
    }

    /// Get the right font implementation from [`FontFamily`].
    pub fn font(&mut self, family: &FontFamily) -> Font<'_> {
        let cached_family = self.family_cache.entry(family.clone()).or_insert_with(|| {
            let fonts = &self.definitions.families.get(family);
            let fonts =
                fonts.unwrap_or_else(|| panic!("FontFamily::{family:?} is not bound to any fonts"));

            let fonts: Vec<FontFaceKey> = fonts
                .iter()
                .map(|font_name| {
                    self.faces
                        .key_by_name(font_name)
                        .unwrap_or_else(|| panic!("No font data found for {font_name:?}"))
                })
                .collect();

            CachedFamily::new(fonts, &mut self.faces)
        });
        Font {
            faces: &mut self.faces,
            cached_family,
            glyphs: &mut self.glyphs,
            family: family.clone(),
            glyph_rasterizer: self.glyph_rasterizer.as_ref(),
            glyph_source_preference: &self.glyph_source_preference,
        }
    }
}

#[cfg(feature = "default_fonts")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_glyph_width() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::empty());
        let mut view = fonts.with_pixels_per_point(1.0);

        let width = view.glyph_width(&FontId::new(12.0, FontFamily::Proportional), ' ');
        assert_eq!(width, 0.0);
    }
}
