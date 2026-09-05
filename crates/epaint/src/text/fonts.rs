use std::{collections::BTreeMap, sync::Arc};

use crate::{
    Color32, TextureAtlas,
    text::{
        FontDefinitions, FontFamily, FontId, Galley, GlyphRasterizer, GlyphSource,
        GlyphSourcePreference, LayoutJob, TextOptions, VariationCoords,
        face_store::{FaceStore, FontFaceKey},
        family::{Family, FamilyKey},
        font_face::{FontFace, GlyphInfo, ShapedGlyph},
        galley_cache::GalleyCache,
        glyph_atlas::{GlyphAtlas, OutlineGlyph, RasterGlyphAllocation},
        glyph_rasterizer::default_glyph_source,
        styled_metrics::StyledMetrics,
    },
};

// ----------------------------------------------------------------------------

/// Maximum width or height of a glyph rasterized into the font atlas.
///
/// Must not exceed the minimum width of the [`TextureAtlas`] (1024).
pub const MAX_GLYPH_SIZE: usize = 1024;

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
        self.fonts.has_glyph(&font_id.family, c)
    }

    /// Do the installed fonts have all the glyphs in this text?
    ///
    /// See [`Self::has_glyph`] for the caveat about the [`GlyphRasterizer`].
    pub fn has_glyphs(&mut self, font_id: &FontId, s: &str) -> bool {
        self.fonts.has_glyphs(&font_id.family, s)
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
        self.fonts.glyph_width(&font_id.family, c, font_id.size)
    }

    /// Do the installed fonts have this glyph?
    ///
    /// This does not consult the [`GlyphRasterizer`], so it can return `false`
    /// for a character that would still render via the rasterizer (e.g. the browser on web).
    pub fn has_glyph(&mut self, font_id: &FontId, c: char) -> bool {
        self.fonts.has_glyph(&font_id.family, c)
    }

    /// Do the installed fonts have all the glyphs in this text?
    ///
    /// See [`Self::has_glyph`] for the caveat about the [`GlyphRasterizer`].
    pub fn has_glyphs(&mut self, font_id: &FontId, s: &str) -> bool {
        self.fonts.has_glyphs(&font_id.family, s)
    }

    /// All characters the fonts of this family support, and the names of the fonts that have each.
    ///
    /// This does not consult the [`GlyphRasterizer`].
    pub fn characters(&mut self, family: &FontFamily) -> &BTreeMap<char, Vec<String>> {
        self.fonts.characters(family)
    }

    /// Height of one row of text in points.
    ///
    /// Returns a value rounded to [`emath::GUI_ROUNDING`].
    #[inline]
    pub fn row_height(&mut self, font_id: &FontId) -> f32 {
        let family = self.fonts.family_key(&font_id.family);
        self.fonts
            .family_metrics(
                family,
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

    /// Indexed by [`FamilyKey`].
    families: Vec<Family>,
    family_keys: ahash::HashMap<FontFamily, FamilyKey>,

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
            families: Default::default(),
            family_keys: Default::default(),
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

    // ------------------------------------------------------------------------
    // Families

    /// Look up (or build) the fallback chain of a family.
    ///
    /// Panics if the family is not in the [`FontDefinitions`].
    pub(crate) fn family_key(&mut self, family: &FontFamily) -> FamilyKey {
        if let Some(key) = self.family_keys.get(family) {
            return *key;
        }
        let key = FamilyKey(self.families.len());
        self.families
            .push(Family::new(family, &self.definitions, &mut self.faces));
        self.family_keys.insert(family.clone(), key);
        key
    }

    #[inline]
    fn family(&self, key: FamilyKey) -> &Family {
        &self.families[key.0]
    }

    /// Find which face in the fallback chain owns `c`.
    ///
    /// See [`Family::resolve`].
    #[inline]
    pub(crate) fn resolve_face(&mut self, family: FamilyKey, c: char) -> FontFaceKey {
        self.families[family.0].resolve(&mut self.faces, c)
    }

    /// Resolve `c` to its (face, [`GlyphInfo`]) at the given face's location.
    ///
    /// `\n` will (intentionally) show up as the replacement character.
    ///
    /// `metrics` must be the resolved [`StyledMetrics`] for the face that ends
    /// up owning `c`. Most callers pass the metrics of their text run's primary
    /// face, which is correct as long as `c` is in that face. For correct
    /// fallback-face advances, resolve the face first with [`Self::resolve_face`]
    /// and build metrics for that face.
    pub(crate) fn glyph_info(
        &mut self,
        family: FamilyKey,
        c: char,
        metrics: &StyledMetrics,
    ) -> (FontFaceKey, GlyphInfo) {
        let face_key = self.resolve_face(family, c);
        let replacement_char = self.family(family).replacement_char();
        let Some(face) = self.faces.get_mut(face_key) else {
            return (face_key, GlyphInfo::INVISIBLE);
        };
        let glyph_info = face.glyph_info(c, metrics).unwrap_or_else(|| {
            // `c` is in no face: render the replacement character instead.
            face.glyph_info(replacement_char, metrics)
                .unwrap_or(GlyphInfo::INVISIBLE)
        });
        (face_key, glyph_info)
    }

    /// Metrics of the primary face of the family.
    pub(crate) fn family_metrics(
        &self,
        family: FamilyKey,
        pixels_per_point: f32,
        font_size: f32,
        coords: &VariationCoords,
    ) -> StyledMetrics {
        self.family(family)
            .primary()
            .and_then(|key| self.faces.get(key))
            .map(|face| face.styled_metrics(pixels_per_point, font_size, coords))
            .unwrap_or_default()
    }

    /// Where to look first for the glyphs of this grapheme cluster.
    #[inline]
    pub(crate) fn glyph_source(&self, cluster: &str) -> GlyphSource {
        (self.glyph_source_preference)(cluster)
    }

    /// Width of this character in points, at the font's default variation location.
    ///
    /// Returns `0.0` if no font has the character.
    pub fn glyph_width(&mut self, family: &FontFamily, c: char, font_size: f32) -> f32 {
        let family = self.family_key(family);
        let face_key = self.resolve_face(family, c);
        let Some(face) = self.faces.get_mut(face_key) else {
            return 0.0;
        };
        let metrics = face.styled_metrics(1.0, font_size, &VariationCoords::default());
        let Some(glyph_info) = face.glyph_info(c, &metrics) else {
            return 0.0;
        };
        glyph_info.advance_width_unscaled.0 * face.px_scale_factor(font_size)
    }

    /// Do the installed fonts have this glyph?
    ///
    /// This does not consult the [`GlyphRasterizer`], so it can return `false`
    /// for a character that would still render via the rasterizer (e.g. the browser on web).
    pub fn has_glyph(&mut self, family: &FontFamily, c: char) -> bool {
        let family = self.family_key(family);
        // TODO(emilk): this is a false negative if the user asks about the replacement character itself 🤦‍♂️
        self.resolve_face(family, c) != self.family(family).replacement_face_key()
    }

    /// Do the installed fonts have all the glyphs in this text?
    ///
    /// See [`Self::has_glyph`] for the caveat about the glyph rasterizer.
    pub fn has_glyphs(&mut self, family: &FontFamily, s: &str) -> bool {
        s.chars().all(|c| self.has_glyph(family, c))
    }

    /// All characters the fonts of this family support, and the names of the fonts that have each.
    pub fn characters(&mut self, family: &FontFamily) -> &BTreeMap<char, Vec<String>> {
        let family = self.family_key(family);
        self.families[family.0].characters(&self.faces)
    }

    // ------------------------------------------------------------------------
    // Faces and glyphs

    #[inline]
    pub(crate) fn face(&self, key: FontFaceKey) -> Option<&FontFace> {
        self.faces.get(key)
    }

    /// Get or render the glyph of a face, and put it in the atlas.
    ///
    /// See [`GlyphAtlas::allocate_outline`].
    pub(crate) fn allocate_glyph(
        &mut self,
        face_key: FontFaceKey,
        metrics: &StyledMetrics,
        shaped: &ShapedGlyph,
    ) -> OutlineGlyph {
        let Some(face) = self.faces.get_mut(face_key) else {
            return Default::default();
        };
        self.glyphs
            .allocate_outline(face_key, face, metrics, shaped)
    }

    #[inline]
    pub(crate) fn has_glyph_rasterizer(&self) -> bool {
        self.glyph_rasterizer.is_some()
    }

    /// Rasterize a grapheme cluster using the platform [`GlyphRasterizer`].
    ///
    /// Returns `None` if there is no rasterizer, or it could not handle the cluster.
    pub(crate) fn rasterize_cluster(
        &mut self,
        family: FamilyKey,
        cluster: &str,
        pixels_per_point: f32,
        font_size: f32,
    ) -> Option<RasterGlyphAllocation> {
        let rasterizer = self.glyph_rasterizer.as_ref()?;
        let family_name = self.families[family.0].name();
        self.glyphs.allocate_raster(
            rasterizer,
            cluster,
            family_name,
            pixels_per_point,
            font_size,
        )
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
