use std::{collections::BTreeMap, sync::Arc};

use crate::{
    mutex::{Mutex, MutexGuard},
    text::{
        font::{Font, FontImpl},
        Galley, LayoutJob,
    },
    TextureAtlas,
};
use emath::NumExt as _;

// ----------------------------------------------------------------------------

/// How to select a sized font.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontId {
    /// Height in points.
    pub size: f32,

    /// What font family to use.
    pub family: FontFamily,
    // TODO(emilk): weight (bold), italics, …
}

impl Default for FontId {
    #[inline]
    fn default() -> Self {
        Self {
            size: 14.0,
            family: FontFamily::Proportional,
        }
    }
}

impl FontId {
    #[inline]
    pub const fn new(size: f32, family: FontFamily) -> Self {
        Self { size, family }
    }

    #[inline]
    pub const fn proportional(size: f32) -> Self {
        Self::new(size, FontFamily::Proportional)
    }

    #[inline]
    pub const fn monospace(size: f32) -> Self {
        Self::new(size, FontFamily::Monospace)
    }
}

#[allow(clippy::derive_hash_xor_eq)]
impl std::hash::Hash for FontId {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self { size, family } = self;
        crate::f32_hash(state, *size);
        family.hash(state);
    }
}

// ----------------------------------------------------------------------------

/// Font of unknown size.
///
/// Which style of font: [`Monospace`][`FontFamily::Monospace`], [`Proportional`][`FontFamily::Proportional`],
/// or by user-chosen name.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FontFamily {
    /// A font where some characters are wider than other (e.g. 'w' is wider than 'i').
    ///
    /// Proportional fonts are easier to read and should be the preferred choice in most situations.
    Proportional,

    /// A font where each character is the same width (`w` is the same width as `i`).
    ///
    /// Useful for code snippets, or when you need to align numbers or text.
    Monospace,

    /// One of the names in [`FontDefinitions::families`].
    ///
    /// ```
    /// # use epaint::FontFamily;
    /// // User-chosen names:
    /// FontFamily::Name("arial".into());
    /// FontFamily::Name("serif".into());
    /// ```
    Name(Arc<str>),
}

impl Default for FontFamily {
    #[inline]
    fn default() -> Self {
        FontFamily::Proportional
    }
}

impl std::fmt::Display for FontFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Monospace => "Monospace".fmt(f),
            Self::Proportional => "Proportional".fmt(f),
            Self::Name(name) => (*name).fmt(f),
        }
    }
}

// ----------------------------------------------------------------------------

/// A `.ttf` or `.otf` file and a font face index.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontData {
    /// The content of a `.ttf` or `.otf` file.
    pub font: std::borrow::Cow<'static, [u8]>,

    /// Which font face in the file to use.
    /// When in doubt, use `0`.
    pub index: u32,

    /// Extra scale and vertical tweak to apply to all text of this font.
    pub tweak: FontTweak,
}

impl FontData {
    pub fn from_static(font: &'static [u8]) -> Self {
        Self {
            font: std::borrow::Cow::Borrowed(font),
            index: 0,
            tweak: Default::default(),
        }
    }

    pub fn from_owned(font: Vec<u8>) -> Self {
        Self {
            font: std::borrow::Cow::Owned(font),
            index: 0,
            tweak: Default::default(),
        }
    }

    pub fn tweak(self, tweak: FontTweak) -> Self {
        Self { tweak, ..self }
    }
}

// ----------------------------------------------------------------------------

/// Extra scale and vertical tweak to apply to all text of a certain font.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontTweak {
    /// Scale the font by this much.
    ///
    /// Default: `1.0` (no scaling).
    pub scale: f32,

    /// Shift font downwards by this fraction of the font size (in points).
    ///
    /// A positive value shifts the text downwards.
    /// A negative value shifts it upwards.
    ///
    /// Example value: `-0.2`.
    pub y_offset_factor: f32,

    /// Shift font downwards by this amount of logical points.
    ///
    /// Example value: `2.0`.
    pub y_offset: f32,
}

impl Default for FontTweak {
    fn default() -> Self {
        Self {
            scale: 1.0,
            y_offset_factor: -0.2, // makes the default fonts look more centered in buttons and such
            y_offset: 0.0,
        }
    }
}

// ----------------------------------------------------------------------------

fn ab_glyph_font_from_font_data(name: &str, data: &FontData) -> ab_glyph::FontArc {
    match &data.font {
        std::borrow::Cow::Borrowed(bytes) => {
            ab_glyph::FontRef::try_from_slice_and_index(bytes, data.index)
                .map(ab_glyph::FontArc::from)
        }
        std::borrow::Cow::Owned(bytes) => {
            ab_glyph::FontVec::try_from_vec_and_index(bytes.clone(), data.index)
                .map(ab_glyph::FontArc::from)
        }
    }
    .unwrap_or_else(|err| panic!("Error parsing {:?} TTF/OTF font file: {}", name, err))
}

/// Describes the font data and the sizes to use.
///
/// Often you would start with [`FontDefinitions::default()`] and then add/change the contents.
///
/// This is how you install your own custom fonts:
/// ```
/// # use {epaint::text::{FontDefinitions, FontFamily, FontData}};
/// # struct FakeEguiCtx {};
/// # impl FakeEguiCtx { fn set_fonts(&self, _: FontDefinitions) {} }
/// # let egui_ctx = FakeEguiCtx {};
/// let mut fonts = FontDefinitions::default();
///
/// // Install my own font (maybe supporting non-latin characters):
/// fonts.font_data.insert("my_font".to_owned(),
///    FontData::from_static(include_bytes!("../../fonts/Ubuntu-Light.ttf"))); // .ttf and .otf supported
///
/// // Put my font first (highest priority):
/// fonts.families.get_mut(&FontFamily::Proportional).unwrap()
///     .insert(0, "my_font".to_owned());
///
/// // Put my font as last fallback for monospace:
/// fonts.families.get_mut(&FontFamily::Monospace).unwrap()
///     .push("my_font".to_owned());
///
/// egui_ctx.set_fonts(fonts);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct FontDefinitions {
    /// List of font names and their definitions.
    ///
    /// `epaint` has built-in-default for these, but you can override them if you like.
    pub font_data: BTreeMap<String, FontData>,

    /// Which fonts (names) to use for each [`FontFamily`].
    ///
    /// The list should be a list of keys into [`Self::font_data`].
    /// When looking for a character glyph `epaint` will start with
    /// the first font and then move to the second, and so on.
    /// So the first font is the primary, and then comes a list of fallbacks in order of priority.
    pub families: BTreeMap<FontFamily, Vec<String>>,
}

impl Default for FontDefinitions {
    /// Specifies the default fonts if the feature `default_fonts` is enabled,
    /// otherwise this is the same as [`Self::empty`].
    #[cfg(not(feature = "default_fonts"))]
    fn default() -> Self {
        Self::empty()
    }

    /// Specifies the default fonts if the feature `default_fonts` is enabled,
    /// otherwise this is the same as [`Self::empty`].
    #[cfg(feature = "default_fonts")]
    fn default() -> Self {
        let mut font_data: BTreeMap<String, FontData> = BTreeMap::new();

        let mut families = BTreeMap::new();

        font_data.insert(
            "Hack".to_owned(),
            FontData::from_static(include_bytes!("../../fonts/Hack-Regular.ttf")),
        );
        font_data.insert(
            "Ubuntu-Light".to_owned(),
            FontData::from_static(include_bytes!("../../fonts/Ubuntu-Light.ttf")),
        );

        // Some good looking emojis. Use as first priority:
        font_data.insert(
            "NotoEmoji-Regular".to_owned(),
            FontData::from_static(include_bytes!("../../fonts/NotoEmoji-Regular.ttf")).tweak(
                FontTweak {
                    scale: 0.81,           // make it smaller
                    y_offset_factor: -0.2, // move it up
                    y_offset: 0.0,
                },
            ),
        );

        // Bigger emojis, and more. <http://jslegers.github.io/emoji-icon-font/>:
        font_data.insert(
            "emoji-icon-font".to_owned(),
            FontData::from_static(include_bytes!("../../fonts/emoji-icon-font.ttf")).tweak(
                FontTweak {
                    scale: 0.88,           // make it smaller
                    y_offset_factor: 0.07, // move it down slightly
                    y_offset: 0.0,
                },
            ),
        );

        families.insert(
            FontFamily::Monospace,
            vec![
                "Hack".to_owned(),
                "Ubuntu-Light".to_owned(), // fallback for √ etc
                "NotoEmoji-Regular".to_owned(),
                "emoji-icon-font".to_owned(),
            ],
        );
        families.insert(
            FontFamily::Proportional,
            vec![
                "Ubuntu-Light".to_owned(),
                "NotoEmoji-Regular".to_owned(),
                "emoji-icon-font".to_owned(),
            ],
        );

        Self {
            font_data,
            families,
        }
    }
}

impl FontDefinitions {
    /// No fonts.
    pub fn empty() -> Self {
        let mut families = BTreeMap::new();
        families.insert(FontFamily::Monospace, vec![]);
        families.insert(FontFamily::Proportional, vec![]);

        Self {
            font_data: Default::default(),
            families,
        }
    }
}

// ----------------------------------------------------------------------------

/// The collection of fonts used by `epaint`.
///
/// Required in order to paint text. Create one and reuse. Cheap to clone.
///
/// Each [`Fonts`] comes with a font atlas textures that needs to be used when painting.
///
/// If you are using `egui`, use `egui::Context::set_fonts` and `egui::Context::fonts`.
///
/// You need to call [`Self::begin_frame`] and [`Self::font_image_delta`] once every frame.
pub struct Fonts(Arc<Mutex<FontsAndCache>>);

impl Fonts {
    /// Create a new [`Fonts`] for text layout.
    /// This call is expensive, so only create one [`Fonts`] and then reuse it.
    ///
    /// * `pixels_per_point`: how many physical pixels per logical "point".
    /// * `max_texture_side`: largest supported texture size (one side).
    pub fn new(
        pixels_per_point: f32,
        max_texture_side: usize,
        definitions: FontDefinitions,
    ) -> Self {
        let fonts_and_cache = FontsAndCache {
            fonts: FontsImpl::new(pixels_per_point, max_texture_side, definitions),
            galley_cache: Default::default(),
        };
        Self(Arc::new(Mutex::new(fonts_and_cache)))
    }

    /// Call at the start of each frame with the latest known
    /// `pixels_per_point` and `max_texture_side`.
    ///
    /// Call after painting the previous frame, but before using [`Fonts`] for the new frame.
    ///
    /// This function will react to changes in `pixels_per_point` and `max_texture_side`,
    /// as well as notice when the font atlas is getting full, and handle that.
    pub fn begin_frame(&self, pixels_per_point: f32, max_texture_side: usize) {
        let mut fonts_and_cache = self.0.lock();

        let pixels_per_point_changed =
            (fonts_and_cache.fonts.pixels_per_point - pixels_per_point).abs() > 1e-3;
        let max_texture_side_changed = fonts_and_cache.fonts.max_texture_side != max_texture_side;
        let font_atlas_almost_full = fonts_and_cache.fonts.atlas.lock().fill_ratio() > 0.8;
        let needs_recreate =
            pixels_per_point_changed || max_texture_side_changed || font_atlas_almost_full;

        if needs_recreate {
            let definitions = fonts_and_cache.fonts.definitions.clone();

            *fonts_and_cache = FontsAndCache {
                fonts: FontsImpl::new(pixels_per_point, max_texture_side, definitions),
                galley_cache: Default::default(),
            };
        }

        fonts_and_cache.galley_cache.flush_cache();
    }

    /// Call at the end of each frame (before painting) to get the change to the font texture since last call.
    pub fn font_image_delta(&self) -> Option<crate::ImageDelta> {
        self.lock().fonts.atlas.lock().take_delta()
    }

    /// Access the underlying [`FontsAndCache`].
    #[doc(hidden)]
    #[inline]
    pub fn lock(&self) -> MutexGuard<'_, FontsAndCache> {
        self.0.lock()
    }

    #[inline]
    pub fn pixels_per_point(&self) -> f32 {
        self.lock().fonts.pixels_per_point
    }

    #[inline]
    pub fn max_texture_side(&self) -> usize {
        self.lock().fonts.max_texture_side
    }

    /// The font atlas.
    /// Pass this to [`crate::Tessellator`].
    pub fn texture_atlas(&self) -> Arc<Mutex<TextureAtlas>> {
        self.lock().fonts.atlas.clone()
    }

    /// Current size of the font image.
    /// Pass this to [`crate::Tessellator`].
    pub fn font_image_size(&self) -> [usize; 2] {
        self.lock().fonts.atlas.lock().size()
    }

    /// Width of this character in points.
    #[inline]
    pub fn glyph_width(&self, font_id: &FontId, c: char) -> f32 {
        self.lock().fonts.glyph_width(font_id, c)
    }

    /// Can we display this glyph?
    #[inline]
    pub fn has_glyph(&self, font_id: &FontId, c: char) -> bool {
        self.lock().fonts.has_glyph(font_id, c)
    }

    /// Can we display all the glyphs in this text?
    pub fn has_glyphs(&self, font_id: &FontId, s: &str) -> bool {
        self.lock().fonts.has_glyphs(font_id, s)
    }

    /// Height of one row of text in points
    #[inline]
    pub fn row_height(&self, font_id: &FontId) -> f32 {
        self.lock().fonts.row_height(font_id)
    }

    /// List of all known font families.
    pub fn families(&self) -> Vec<FontFamily> {
        self.lock()
            .fonts
            .definitions
            .families
            .keys()
            .cloned()
            .collect()
    }

    /// Layout some text.
    ///
    /// This is the most advanced layout function.
    /// See also [`Self::layout`], [`Self::layout_no_wrap`] and
    /// [`Self::layout_delayed_color`].
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    #[inline]
    pub fn layout_job(&self, job: LayoutJob) -> Arc<Galley> {
        self.lock().layout_job(job)
    }

    pub fn num_galleys_in_cache(&self) -> usize {
        self.lock().galley_cache.num_galleys_in_cache()
    }

    /// How full is the font atlas?
    ///
    /// This increases as new fonts and/or glyphs are used,
    /// but can also decrease in a call to [`Self::begin_frame`].
    pub fn font_atlas_fill_ratio(&self) -> f32 {
        self.lock().fonts.atlas.lock().fill_ratio()
    }

    /// Will wrap text at the given width and line break at `\n`.
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    pub fn layout(
        &self,
        text: String,
        font_id: FontId,
        color: crate::Color32,
        wrap_width: f32,
    ) -> Arc<Galley> {
        let job = LayoutJob::simple(text, font_id, color, wrap_width);
        self.layout_job(job)
    }

    /// Will line break at `\n`.
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    pub fn layout_no_wrap(
        &self,
        text: String,
        font_id: FontId,
        color: crate::Color32,
    ) -> Arc<Galley> {
        let job = LayoutJob::simple(text, font_id, color, f32::INFINITY);
        self.layout_job(job)
    }

    /// Like [`Self::layout`], made for when you want to pick a color for the text later.
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    pub fn layout_delayed_color(
        &self,
        text: String,
        font_id: FontId,
        wrap_width: f32,
    ) -> Arc<Galley> {
        self.layout_job(LayoutJob::simple(
            text,
            font_id,
            crate::Color32::TEMPORARY_COLOR,
            wrap_width,
        ))
    }
}

// ----------------------------------------------------------------------------

pub struct FontsAndCache {
    pub fonts: FontsImpl,
    galley_cache: GalleyCache,
}

impl FontsAndCache {
    fn layout_job(&mut self, job: LayoutJob) -> Arc<Galley> {
        self.galley_cache.layout(&mut self.fonts, job)
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
struct HashableF32(f32);

#[allow(clippy::derive_hash_xor_eq)]
impl std::hash::Hash for HashableF32 {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        crate::f32_hash(state, self.0);
    }
}

impl Eq for HashableF32 {}

// ----------------------------------------------------------------------------

/// The collection of fonts used by `epaint`.
///
/// Required in order to paint text.
pub struct FontsImpl {
    pixels_per_point: f32,
    max_texture_side: usize,
    definitions: FontDefinitions,
    atlas: Arc<Mutex<TextureAtlas>>,
    font_impl_cache: FontImplCache,
    sized_family: ahash::HashMap<(HashableF32, FontFamily), Font>,
}

impl FontsImpl {
    /// Create a new [`FontsImpl`] for text layout.
    /// This call is expensive, so only create one [`FontsImpl`] and then reuse it.
    pub fn new(
        pixels_per_point: f32,
        max_texture_side: usize,
        definitions: FontDefinitions,
    ) -> Self {
        assert!(
            0.0 < pixels_per_point && pixels_per_point < 100.0,
            "pixels_per_point out of range: {}",
            pixels_per_point
        );

        let texture_width = max_texture_side.at_most(8 * 1024);
        let initial_height = 64;
        let atlas = TextureAtlas::new([texture_width, initial_height]);

        let atlas = Arc::new(Mutex::new(atlas));

        let font_impl_cache =
            FontImplCache::new(atlas.clone(), pixels_per_point, &definitions.font_data);

        Self {
            pixels_per_point,
            max_texture_side,
            definitions,
            atlas,
            font_impl_cache,
            sized_family: Default::default(),
        }
    }

    #[inline(always)]
    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }

    #[inline]
    pub fn definitions(&self) -> &FontDefinitions {
        &self.definitions
    }

    /// Get the right font implementation from size and [`FontFamily`].
    pub fn font(&mut self, font_id: &FontId) -> &mut Font {
        let FontId { size, family } = font_id;

        self.sized_family
            .entry((HashableF32(*size), family.clone()))
            .or_insert_with(|| {
                let fonts = &self.definitions.families.get(family);
                let fonts = fonts.unwrap_or_else(|| {
                    panic!("FontFamily::{:?} is not bound to any fonts", family)
                });

                let fonts: Vec<Arc<FontImpl>> = fonts
                    .iter()
                    .map(|font_name| self.font_impl_cache.font_impl(*size, font_name))
                    .collect();

                Font::new(fonts)
            })
    }

    /// Width of this character in points.
    fn glyph_width(&mut self, font_id: &FontId, c: char) -> f32 {
        self.font(font_id).glyph_width(c)
    }

    /// Can we display this glyph?
    pub fn has_glyph(&mut self, font_id: &FontId, c: char) -> bool {
        self.font(font_id).has_glyph(c)
    }

    /// Can we display all the glyphs in this text?
    pub fn has_glyphs(&mut self, font_id: &FontId, s: &str) -> bool {
        self.font(font_id).has_glyphs(s)
    }

    /// Height of one row of text. In points
    fn row_height(&mut self, font_id: &FontId) -> f32 {
        self.font(font_id).row_height()
    }
}

// ----------------------------------------------------------------------------

struct CachedGalley {
    /// When it was last used
    last_used: u32,
    galley: Arc<Galley>,
}

#[derive(Default)]
struct GalleyCache {
    /// Frame counter used to do garbage collection on the cache
    generation: u32,
    cache: nohash_hasher::IntMap<u64, CachedGalley>,
}

impl GalleyCache {
    fn layout(&mut self, fonts: &mut FontsImpl, job: LayoutJob) -> Arc<Galley> {
        let hash = crate::util::hash(&job); // TODO(emilk): even faster hasher?

        match self.cache.entry(hash) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                let cached = entry.into_mut();
                cached.last_used = self.generation;
                cached.galley.clone()
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let galley = super::layout(fonts, job.into());
                let galley = Arc::new(galley);
                entry.insert(CachedGalley {
                    last_used: self.generation,
                    galley: galley.clone(),
                });
                galley
            }
        }
    }

    pub fn num_galleys_in_cache(&self) -> usize {
        self.cache.len()
    }

    /// Must be called once per frame to clear the [`Galley`] cache.
    pub fn flush_cache(&mut self) {
        let current_generation = self.generation;
        self.cache.retain(|_key, cached| {
            cached.last_used == current_generation // only keep those that were used this frame
        });
        self.generation = self.generation.wrapping_add(1);
    }
}

// ----------------------------------------------------------------------------

struct FontImplCache {
    atlas: Arc<Mutex<TextureAtlas>>,
    pixels_per_point: f32,
    ab_glyph_fonts: BTreeMap<String, (FontTweak, ab_glyph::FontArc)>,

    /// Map font pixel sizes and names to the cached [`FontImpl`].
    cache: ahash::HashMap<(u32, String), Arc<FontImpl>>,
}

impl FontImplCache {
    pub fn new(
        atlas: Arc<Mutex<TextureAtlas>>,
        pixels_per_point: f32,
        font_data: &BTreeMap<String, FontData>,
    ) -> Self {
        let ab_glyph_fonts = font_data
            .iter()
            .map(|(name, font_data)| {
                let tweak = font_data.tweak;
                let ab_glyph = ab_glyph_font_from_font_data(name, font_data);
                (name.clone(), (tweak, ab_glyph))
            })
            .collect();

        Self {
            atlas,
            pixels_per_point,
            ab_glyph_fonts,
            cache: Default::default(),
        }
    }

    pub fn font_impl(&mut self, scale_in_points: f32, font_name: &str) -> Arc<FontImpl> {
        use ab_glyph::Font as _;

        let (tweak, ab_glyph_font) = self
            .ab_glyph_fonts
            .get(font_name)
            .unwrap_or_else(|| panic!("No font data found for {:?}", font_name))
            .clone();

        let scale_in_pixels = self.pixels_per_point * scale_in_points;

        // Scale the font properly (see https://github.com/emilk/egui/issues/2068).
        let units_per_em = ab_glyph_font.units_per_em().unwrap_or_else(|| {
            panic!(
                "The font unit size of {:?} exceeds the expected range (16..=16384)",
                font_name
            )
        });
        let font_scaling = ab_glyph_font.height_unscaled() / units_per_em;
        let scale_in_pixels = scale_in_pixels * font_scaling;

        // Tweak the scale as the user desired:
        let scale_in_pixels = scale_in_pixels * tweak.scale;

        // Round to an even number of physical pixels to get even kerning.
        // See https://github.com/emilk/egui/issues/382
        let scale_in_pixels = scale_in_pixels.round() as u32;

        let y_offset_points = {
            let scale_in_points = scale_in_pixels as f32 / self.pixels_per_point;
            scale_in_points * tweak.y_offset_factor
        } + tweak.y_offset;

        self.cache
            .entry((scale_in_pixels, font_name.to_owned()))
            .or_insert_with(|| {
                Arc::new(FontImpl::new(
                    self.atlas.clone(),
                    self.pixels_per_point,
                    font_name.to_owned(),
                    ab_glyph_font,
                    scale_in_pixels,
                    y_offset_points,
                ))
            })
            .clone()
    }
}
