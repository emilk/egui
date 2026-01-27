use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{
    TextureAtlas,
    text::{
        Galley, LayoutJob, LayoutSection, TextOptions,
        font::{Font, FontFace, GlyphInfo},
    },
};
use emath::{NumExt as _, OrderedFloat};

#[cfg(feature = "default_fonts")]
use epaint_default_fonts::{EMOJI_ICON, HACK_REGULAR, NOTO_EMOJI_REGULAR, UBUNTU_LIGHT};

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

impl std::hash::Hash for FontId {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self { size, family } = self;
        emath::OrderedFloat(*size).hash(state);
        family.hash(state);
    }
}

// ----------------------------------------------------------------------------

/// Font of unknown size.
///
/// Which style of font: [`Monospace`][`FontFamily::Monospace`], [`Proportional`][`FontFamily::Proportional`],
/// or by user-chosen name.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FontFamily {
    /// A font where some characters are wider than other (e.g. 'w' is wider than 'i').
    ///
    /// Proportional fonts are easier to read and should be the preferred choice in most situations.
    #[default]
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
    pub font: Cow<'static, [u8]>,

    /// Which font face in the file to use.
    /// When in doubt, use `0`.
    pub index: u32,

    /// Extra scale and vertical tweak to apply to all text of this font.
    pub tweak: FontTweak,

    /// The font weight (100-900), if available.
    /// Standard values: 100 (Thin), 200 (Extra Light), 300 (Light), 400 (Regular),
    /// 500 (Medium), 600 (Semi Bold), 700 (Bold), 800 (Extra Bold), 900 (Black).
    /// `None` if the weight could not be determined.
    pub weight: Option<u16>,
}

impl FontData {
    pub fn from_static(font: &'static [u8]) -> Self {
        Self {
            font: Cow::Borrowed(font),
            index: 0,
            tweak: Default::default(),
            weight: None,
        }
    }

    pub fn from_owned(font: Vec<u8>) -> Self {
        Self {
            font: Cow::Owned(font),
            index: 0,
            tweak: Default::default(),
            weight: None,
        }
    }

    pub fn tweak(self, tweak: FontTweak) -> Self {
        Self { tweak, ..self }
    }

    /// Set the font weight (100-900).
    ///
    /// This is typically read automatically from the font file when loaded,
    /// but can be overridden manually if needed.
    ///
    /// Standard weight values:
    /// - 100: Thin
    /// - 200: Extra Light
    /// - 300: Light
    /// - 400: Regular/Normal
    /// - 500: Medium
    /// - 600: Semi Bold
    /// - 700: Bold
    /// - 800: Extra Bold
    /// - 900: Black
    ///
    /// # Example
    /// ```
    /// # use epaint::text::FontData;
    /// let font_data = FontData::from_static(include_bytes!("../../../epaint_default_fonts/fonts/Ubuntu-Light.ttf"))
    ///     .weight(300); // Override to Light weight
    /// assert_eq!(font_data.weight, Some(300));
    /// ```
    pub fn weight(self, weight: u16) -> Self {
        Self {
            weight: Some(weight),
            ..self
        }
    }
}

impl AsRef<[u8]> for FontData {
    fn as_ref(&self) -> &[u8] {
        self.font.as_ref()
    }
}

// ----------------------------------------------------------------------------

/// Extra scale and vertical tweak to apply to all text of a certain font.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontTweak {
    /// Scale the font's glyphs by this much.
    /// this is only a visual effect and does not affect the text layout.
    ///
    /// Default: `1.0` (no scaling).
    pub scale: f32,

    /// Shift font's glyphs downwards by this fraction of the font size (in points).
    /// this is only a visual effect and does not affect the text layout.
    ///
    /// Affects larger font sizes more.
    ///
    /// A positive value shifts the text downwards.
    /// A negative value shifts it upwards.
    ///
    /// Example value: `-0.2`.
    pub y_offset_factor: f32,

    /// Shift font's glyphs downwards by this amount of logical points.
    /// this is only a visual effect and does not affect the text layout.
    ///
    /// Affects all font sizes equally.
    ///
    /// Example value: `2.0`.
    pub y_offset: f32,

    /// Override the global font hinting setting for this specific font.
    ///
    /// `None` means use the global setting.
    pub hinting_override: Option<bool>,
}

impl Default for FontTweak {
    fn default() -> Self {
        Self {
            scale: 1.0,
            y_offset_factor: 0.0,
            y_offset: 0.0,
            hinting_override: None,
        }
    }
}

// ----------------------------------------------------------------------------

pub type Blob = Arc<dyn AsRef<[u8]> + Send + Sync>;

fn blob_from_font_data(data: &FontData) -> Blob {
    match data.clone().font {
        Cow::Borrowed(bytes) => Arc::new(bytes) as Blob,
        Cow::Owned(bytes) => Arc::new(bytes) as Blob,
    }
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
///    std::sync::Arc::new(
///        // .ttf and .otf supported
///        FontData::from_static(include_bytes!("../../../epaint_default_fonts/fonts/Ubuntu-Light.ttf"))
///    )
/// );
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
    pub font_data: BTreeMap<String, Arc<FontData>>,

    /// Which fonts (names) to use for each [`FontFamily`].
    ///
    /// The list should be a list of keys into [`Self::font_data`].
    /// When looking for a character glyph `epaint` will start with
    /// the first font and then move to the second, and so on.
    /// So the first font is the primary, and then comes a list of fallbacks in order of priority.
    pub families: BTreeMap<FontFamily, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct FontInsert {
    /// Font name
    pub name: String,

    /// A `.ttf` or `.otf` file and a font face index.
    pub data: FontData,

    /// Sets the font family and priority
    pub families: Vec<InsertFontFamily>,
}

#[derive(Debug, Clone)]
pub struct InsertFontFamily {
    /// Font family
    pub family: FontFamily,

    /// Fallback or Primary font
    pub priority: FontPriority,
}

#[derive(Debug, Clone)]
pub enum FontPriority {
    /// Prefer this font before all existing ones.
    ///
    /// If a desired glyph exists in this font, it will be used.
    Highest,

    /// Use this font as a fallback, after all existing ones.
    ///
    /// This font will only be used if the glyph is not found in any of the previously installed fonts.
    Lowest,
}

impl FontInsert {
    pub fn new(name: &str, data: FontData, families: Vec<InsertFontFamily>) -> Self {
        Self {
            name: name.to_owned(),
            data,
            families,
        }
    }
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
        let mut font_data: BTreeMap<String, Arc<FontData>> = BTreeMap::new();

        let mut families = BTreeMap::new();

        font_data.insert(
            "Hack".to_owned(),
            Arc::new(FontData::from_static(HACK_REGULAR)),
        );

        // Some good looking emojis. Use as first priority:
        font_data.insert(
            "NotoEmoji-Regular".to_owned(),
            Arc::new(FontData::from_static(NOTO_EMOJI_REGULAR).tweak(FontTweak {
                scale: 0.81, // Make smaller
                ..Default::default()
            })),
        );

        font_data.insert(
            "Ubuntu-Light".to_owned(),
            Arc::new(FontData::from_static(UBUNTU_LIGHT)),
        );

        // Bigger emojis, and more. <http://jslegers.github.io/emoji-icon-font/>:
        font_data.insert(
            "emoji-icon-font".to_owned(),
            Arc::new(FontData::from_static(EMOJI_ICON).tweak(FontTweak {
                scale: 0.90, // Make smaller
                ..Default::default()
            })),
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

    /// List of all the builtin font names used by `epaint`.
    #[cfg(feature = "default_fonts")]
    pub fn builtin_font_names() -> &'static [&'static str] {
        &[
            "Ubuntu-Light",
            "NotoEmoji-Regular",
            "emoji-icon-font",
            "Hack",
        ]
    }

    /// List of all the builtin font names used by `epaint`.
    #[cfg(not(feature = "default_fonts"))]
    pub fn builtin_font_names() -> &'static [&'static str] {
        &[]
    }
}

/// Unique ID for looking up a single font face/file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FontFaceKey(u64);

impl FontFaceKey {
    pub const INVALID: Self = Self(0);

    fn new() -> Self {
        static KEY_COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(crate::util::hash(
            KEY_COUNTER.fetch_add(1, Ordering::Relaxed),
        ))
    }
}

// Safe, because we hash the value in the constructor.
impl nohash_hasher::IsEnabled for FontFaceKey {}

/// Cached data for working with a font family (e.g. doing character lookups).
#[derive(Debug)]
pub(super) struct CachedFamily {
    pub fonts: Vec<FontFaceKey>,

    /// Lazily calculated.
    pub characters: Option<BTreeMap<char, Vec<String>>>,

    pub replacement_glyph: (FontFaceKey, GlyphInfo),

    pub glyph_info_cache: ahash::HashMap<char, (FontFaceKey, GlyphInfo)>,
}

impl CachedFamily {
    fn new(
        fonts: Vec<FontFaceKey>,
        fonts_by_id: &mut nohash_hasher::IntMap<FontFaceKey, FontFace>,
    ) -> Self {
        if fonts.is_empty() {
            return Self {
                fonts,
                characters: None,
                replacement_glyph: (FontFaceKey::INVALID, GlyphInfo::INVISIBLE),
                glyph_info_cache: Default::default(),
            };
        }

        let mut slf = Self {
            fonts,
            characters: None,
            replacement_glyph: (FontFaceKey::INVALID, GlyphInfo::INVISIBLE),
            glyph_info_cache: Default::default(),
        };

        const PRIMARY_REPLACEMENT_CHAR: char = '◻'; // white medium square
        const FALLBACK_REPLACEMENT_CHAR: char = '?'; // fallback for the fallback

        let replacement_glyph = slf
            .glyph_info_no_cache_or_fallback(PRIMARY_REPLACEMENT_CHAR, fonts_by_id)
            .or_else(|| slf.glyph_info_no_cache_or_fallback(FALLBACK_REPLACEMENT_CHAR, fonts_by_id))
            .unwrap_or_else(|| {
                log::warn!(
                    "Failed to find replacement characters {PRIMARY_REPLACEMENT_CHAR:?} or {FALLBACK_REPLACEMENT_CHAR:?}. Will use empty glyph."
                );
                (FontFaceKey::INVALID, GlyphInfo::INVISIBLE)
            });
        slf.replacement_glyph = replacement_glyph;

        slf
    }

    pub(crate) fn glyph_info_no_cache_or_fallback(
        &mut self,
        c: char,
        fonts_by_id: &mut nohash_hasher::IntMap<FontFaceKey, FontFace>,
    ) -> Option<(FontFaceKey, GlyphInfo)> {
        for font_key in &self.fonts {
            let font_face = fonts_by_id.get_mut(font_key).expect("Nonexistent font ID");
            if let Some(glyph_info) = font_face.glyph_info(c) {
                self.glyph_info_cache.insert(c, (*font_key, glyph_info));
                return Some((*font_key, glyph_info));
            }
        }
        None
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
/// You need to call [`Self::begin_pass`] and [`Self::font_image_delta`] once every frame.
pub struct Fonts {
    pub fonts: FontsImpl,
    galley_cache: GalleyCache,
}

impl Fonts {
    /// Create a new [`Fonts`] for text layout.
    /// This call is expensive, so only create one [`Fonts`] and then reuse it.
    pub fn new(options: TextOptions, definitions: FontDefinitions) -> Self {
        Self {
            fonts: FontsImpl::new(options, definitions),
            galley_cache: Default::default(),
        }
    }

    /// Call at the start of each frame with the latest known [`TextOptions`].
    ///
    /// Call after painting the previous frame, but before using [`Fonts`] for the new frame.
    ///
    /// This function will react to changes in [`TextOptions`],
    /// as well as notice when the font atlas is getting full, and handle that.
    pub fn begin_pass(&mut self, options: TextOptions) {
        let text_options_changed = self.fonts.options() != &options;
        let font_atlas_almost_full = self.fonts.atlas.fill_ratio() > 0.8;
        let needs_recreate = text_options_changed || font_atlas_almost_full;

        if needs_recreate {
            let definitions = self.fonts.definitions.clone();

            *self = Self {
                fonts: FontsImpl::new(options, definitions),
                galley_cache: Default::default(),
            };
        }

        self.galley_cache.flush_cache();
    }

    /// Call at the end of each frame (before painting) to get the change to the font texture since last call.
    pub fn font_image_delta(&mut self) -> Option<crate::ImageDelta> {
        self.fonts.atlas.take_delta()
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
        &self.fonts.atlas
    }

    /// The full font atlas image.
    #[inline]
    pub fn image(&self) -> crate::ColorImage {
        self.fonts.atlas.image().clone()
    }

    /// Current size of the font image.
    /// Pass this to [`crate::Tessellator`].
    pub fn font_image_size(&self) -> [usize; 2] {
        self.fonts.atlas.size()
    }

    /// Can we display this glyph?
    pub fn has_glyph(&mut self, font_id: &FontId, c: char) -> bool {
        self.fonts.font(&font_id.family).has_glyph(c)
    }

    /// Can we display all the glyphs in this text?
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
        self.fonts.atlas.fill_ratio()
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
        self.fonts.atlas.image().clone()
    }

    /// Current size of the font image.
    /// Pass this to [`crate::Tessellator`].
    pub fn font_image_size(&self) -> [usize; 2] {
        self.fonts.atlas.size()
    }

    /// Width of this character in points.
    ///
    /// If the font doesn't exist, this will return `0.0`.
    pub fn glyph_width(&mut self, font_id: &FontId, c: char) -> f32 {
        self.fonts
            .font(&font_id.family)
            .glyph_width(c, font_id.size)
    }

    /// Can we display this glyph?
    pub fn has_glyph(&mut self, font_id: &FontId, c: char) -> bool {
        self.fonts.font(&font_id.family).has_glyph(c)
    }

    /// Can we display all the glyphs in this text?
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
            .scaled_metrics(self.pixels_per_point, font_id.size)
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
        self.fonts.atlas.fill_ratio()
    }

    /// Will wrap text at the given width and line break at `\n`.
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    #[inline]
    pub fn layout(
        &mut self,
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
    #[inline]
    pub fn layout_no_wrap(
        &mut self,
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
    #[inline]
    pub fn layout_delayed_color(
        &mut self,
        text: String,
        font_id: FontId,
        wrap_width: f32,
    ) -> Arc<Galley> {
        self.layout(text, font_id, crate::Color32::PLACEHOLDER, wrap_width)
    }
}

// ----------------------------------------------------------------------------

/// The collection of fonts used by `epaint`.
///
/// Required in order to paint text.
pub struct FontsImpl {
    definitions: FontDefinitions,
    atlas: TextureAtlas,
    fonts_by_id: nohash_hasher::IntMap<FontFaceKey, FontFace>,
    fonts_by_name: ahash::HashMap<String, FontFaceKey>,
    family_cache: ahash::HashMap<FontFamily, CachedFamily>,
}

impl FontsImpl {
    /// Create a new [`FontsImpl`] for text layout.
    /// This call is expensive, so only create one [`FontsImpl`] and then reuse it.
    pub fn new(options: TextOptions, definitions: FontDefinitions) -> Self {
        let texture_width = options.max_texture_side.at_most(16 * 1024);
        let initial_height = 32; // Keep initial font atlas small, so it is fast to upload to GPU. This will expand as needed anyways.
        let atlas = TextureAtlas::new([texture_width, initial_height], options);

        let mut fonts_by_id: nohash_hasher::IntMap<FontFaceKey, FontFace> = Default::default();
        let mut fonts_by_name: ahash::HashMap<String, FontFaceKey> = Default::default();
        for (name, font_data) in &definitions.font_data {
            let tweak = font_data.tweak;
            let blob = blob_from_font_data(font_data);
            let font_face = FontFace::new(
                options,
                name.clone(),
                blob,
                font_data.index,
                tweak,
                font_data.weight,
            )
            .unwrap_or_else(|err| panic!("Error parsing {name:?} TTF/OTF font file: {err}"));
            let key = FontFaceKey::new();
            fonts_by_id.insert(key, font_face);
            fonts_by_name.insert(name.clone(), key);
        }

        Self {
            definitions,
            atlas,
            fonts_by_id,
            fonts_by_name,
            family_cache: Default::default(),
        }
    }

    pub fn options(&self) -> &TextOptions {
        self.atlas.options()
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
                    *self
                        .fonts_by_name
                        .get(font_name)
                        .unwrap_or_else(|| panic!("No font data found for {font_name:?}"))
                })
                .collect();

            CachedFamily::new(fonts, &mut self.fonts_by_id)
        });
        Font {
            fonts_by_id: &mut self.fonts_by_id,
            cached_family,
            atlas: &mut self.atlas,
        }
    }
}

// ----------------------------------------------------------------------------

struct CachedGalley {
    /// When it was last used
    last_used: u32,

    /// Hashes of all other entries this one depends on for quick re-layout.
    /// Their `last_used`s should be updated alongside this one to make sure they're
    /// not evicted.
    children: Option<Arc<[u64]>>,

    galley: Arc<Galley>,
}

#[derive(Default)]
struct GalleyCache {
    /// Frame counter used to do garbage collection on the cache
    generation: u32,
    cache: nohash_hasher::IntMap<u64, CachedGalley>,
}

impl GalleyCache {
    fn layout_internal(
        &mut self,
        fonts: &mut FontsImpl,
        mut job: LayoutJob,
        pixels_per_point: f32,
        allow_split_paragraphs: bool,
    ) -> (u64, Arc<Galley>) {
        if job.wrap.max_width.is_finite() {
            // Protect against rounding errors in egui layout code.

            // Say the user asks to wrap at width 200.0.
            // The text layout wraps, and reports that the final width was 196.0 points.
            // This then trickles up the `Ui` chain and gets stored as the width for a tooltip (say).
            // On the next frame, this is then set as the max width for the tooltip,
            // and we end up calling the text layout code again, this time with a wrap width of 196.0.
            // Except, somewhere in the `Ui` chain with added margins etc, a rounding error was introduced,
            // so that we actually set a wrap-width of 195.9997 instead.
            // Now the text that fit perfrectly at 196.0 needs to wrap one word earlier,
            // and so the text re-wraps and reports a new width of 185.0 points.
            // And then the cycle continues.

            // So we limit max_width to integers.

            // Related issues:
            // * https://github.com/emilk/egui/issues/4927
            // * https://github.com/emilk/egui/issues/4928
            // * https://github.com/emilk/egui/issues/5084
            // * https://github.com/emilk/egui/issues/5163

            job.wrap.max_width = job.wrap.max_width.round();
        }

        let hash = crate::util::hash((&job, OrderedFloat(pixels_per_point))); // TODO(emilk): even faster hasher?

        let galley = match self.cache.entry(hash) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                // The job was found in cache - no need to re-layout.
                let cached = entry.into_mut();
                cached.last_used = self.generation;

                let galley = Arc::clone(&cached.galley);
                if let Some(children) = &cached.children {
                    // The point of `allow_split_paragraphs` is to split large jobs into paragraph,
                    // and then cache each paragraph individually.
                    // That way, if we edit a single paragraph, only that paragraph will be re-layouted.
                    // For that to work we need to keep all the child/paragraph
                    // galleys alive while the parent galley is alive:
                    for child_hash in Arc::clone(children).iter() {
                        if let Some(cached_child) = self.cache.get_mut(child_hash) {
                            cached_child.last_used = self.generation;
                        }
                    }
                }

                galley
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let job = Arc::new(job);
                if allow_split_paragraphs && should_cache_each_paragraph_individually(&job) {
                    let (child_galleys, child_hashes) =
                        self.layout_each_paragraph_individually(fonts, &job, pixels_per_point);
                    debug_assert_eq!(
                        child_hashes.len(),
                        child_galleys.len(),
                        "Bug in `layout_each_paragraph_individually`"
                    );
                    let galley = Arc::new(Galley::concat(job, &child_galleys, pixels_per_point));

                    self.cache.insert(
                        hash,
                        CachedGalley {
                            last_used: self.generation,
                            children: Some(child_hashes.into()),
                            galley: Arc::clone(&galley),
                        },
                    );
                    galley
                } else {
                    let galley = super::layout(fonts, pixels_per_point, job);
                    let galley = Arc::new(galley);
                    entry.insert(CachedGalley {
                        last_used: self.generation,
                        children: None,
                        galley: Arc::clone(&galley),
                    });
                    galley
                }
            }
        };

        (hash, galley)
    }

    fn layout(
        &mut self,
        fonts: &mut FontsImpl,
        pixels_per_point: f32,
        job: LayoutJob,
        allow_split_paragraphs: bool,
    ) -> Arc<Galley> {
        self.layout_internal(fonts, job, pixels_per_point, allow_split_paragraphs)
            .1
    }

    /// Split on `\n` and lay out (and cache) each paragraph individually.
    fn layout_each_paragraph_individually(
        &mut self,
        fonts: &mut FontsImpl,
        job: &LayoutJob,
        pixels_per_point: f32,
    ) -> (Vec<Arc<Galley>>, Vec<u64>) {
        profiling::function_scope!();

        let mut current_section = 0;
        let mut start = 0;
        let mut max_rows_remaining = job.wrap.max_rows;
        let mut child_galleys = Vec::new();
        let mut child_hashes = Vec::new();

        while start < job.text.len() {
            let is_first_paragraph = start == 0;
            // `end` will not include the `\n` since we don't want to create an empty row in our
            // split galley
            let mut end = job.text[start..]
                .find('\n')
                .map_or(job.text.len(), |i| start + i);
            if end == job.text.len() - 1 && job.text.ends_with('\n') {
                end += 1; // If the text ends with a newline, we include it in the last paragraph.
            }

            let mut paragraph_job = LayoutJob {
                text: job.text[start..end].to_owned(),
                wrap: crate::text::TextWrapping {
                    max_rows: max_rows_remaining,
                    ..job.wrap
                },
                sections: Vec::new(),
                break_on_newline: job.break_on_newline,
                halign: job.halign,
                justify: job.justify,
                first_row_min_height: if is_first_paragraph {
                    job.first_row_min_height
                } else {
                    0.0
                },
                round_output_to_gui: job.round_output_to_gui,
            };

            // Add overlapping sections:
            for section in &job.sections[current_section..job.sections.len()] {
                let LayoutSection {
                    leading_space,
                    byte_range: section_range,
                    format,
                } = section;

                // `start` and `end` are the byte range of the current paragraph.
                // How does the current section overlap with the paragraph range?

                if section_range.end <= start {
                    // The section is behind us
                    current_section += 1;
                } else if end < section_range.start {
                    break; // Haven't reached this one yet.
                } else {
                    // Section range overlaps with paragraph range
                    debug_assert!(
                        section_range.start <= section_range.end,
                        "Bad byte_range: {section_range:?}"
                    );
                    let new_range = section_range.start.saturating_sub(start)
                        ..(section_range.end.at_most(end)).saturating_sub(start);
                    debug_assert!(
                        new_range.start <= new_range.end,
                        "Bad new section range: {new_range:?}"
                    );
                    paragraph_job.sections.push(LayoutSection {
                        leading_space: if start <= section_range.start {
                            *leading_space
                        } else {
                            0.0
                        },
                        byte_range: new_range,
                        format: format.clone(),
                    });
                }
            }

            // TODO(emilk): we could lay out each paragraph in parallel to get a nice speedup on multicore machines.
            let (hash, galley) =
                self.layout_internal(fonts, paragraph_job, pixels_per_point, false);
            child_hashes.push(hash);

            // This will prevent us from invalidating cache entries unnecessarily:
            if max_rows_remaining != usize::MAX {
                max_rows_remaining -= galley.rows.len();
            }

            let elided = galley.elided;
            child_galleys.push(galley);
            if elided {
                break;
            }

            start = end + 1;
        }

        (child_galleys, child_hashes)
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

/// If true, lay out and cache each paragraph (sections separated by newlines) individually.
///
/// This makes it much faster to re-layout the full text when only a portion of it has changed since last frame, i.e. when editing somewhere in a file with thousands of lines/paragraphs.
fn should_cache_each_paragraph_individually(job: &LayoutJob) -> bool {
    // We currently don't support this elided text, i.e. when `max_rows` is set.
    // Most often, elided text is elided to one row,
    // and so will always be fast to lay out.
    job.break_on_newline && job.wrap.max_rows == usize::MAX && job.text.contains('\n')
}

#[cfg(feature = "default_fonts")]
#[cfg(test)]
mod tests {
    use core::f32;

    use super::*;
    use crate::text::{TextWrapping, layout};
    use crate::{Stroke, text::TextFormat};
    use ecolor::Color32;
    use emath::Align;

    fn jobs() -> Vec<LayoutJob> {
        vec![
            LayoutJob::simple(
                String::default(),
                FontId::new(14.0, FontFamily::Monospace),
                Color32::WHITE,
                f32::INFINITY,
            ),
            LayoutJob::simple(
                "ends with newlines\n\n".to_owned(),
                FontId::new(14.0, FontFamily::Monospace),
                Color32::WHITE,
                f32::INFINITY,
            ),
            LayoutJob::simple(
                "Simple test.".to_owned(),
                FontId::new(14.0, FontFamily::Monospace),
                Color32::WHITE,
                f32::INFINITY,
            ),
            {
                let mut job = LayoutJob::simple(
                    "hi".to_owned(),
                    FontId::default(),
                    Color32::WHITE,
                    f32::INFINITY,
                );
                job.append("\n", 0.0, TextFormat::default());
                job.append("\n", 0.0, TextFormat::default());
                job.append("world", 0.0, TextFormat::default());
                job.wrap.max_rows = 2;
                job
            },
            {
                let mut job = LayoutJob::simple(
                    "Test text with a lot of words\n and a newline.".to_owned(),
                    FontId::new(14.0, FontFamily::Monospace),
                    Color32::WHITE,
                    40.0,
                );
                job.first_row_min_height = 30.0;
                job
            },
            LayoutJob::simple(
                "This some text that may be long.\nDet kanske också finns lite ÅÄÖ här.".to_owned(),
                FontId::new(14.0, FontFamily::Proportional),
                Color32::WHITE,
                50.0,
            ),
            {
                let mut job = LayoutJob {
                    first_row_min_height: 20.0,
                    ..Default::default()
                };
                job.append(
                    "1st paragraph has underline and strikethrough, and has some non-ASCII characters:\n ÅÄÖ.",
                    0.0,
                    TextFormat {
                        font_id: FontId::new(15.0, FontFamily::Monospace),
                        underline: Stroke::new(1.0, Color32::RED),
                        strikethrough: Stroke::new(1.0, Color32::GREEN),
                        ..Default::default()
                    },
                );
                job.append(
                    "2nd paragraph has some leading space.\n",
                    16.0,
                    TextFormat {
                        font_id: FontId::new(14.0, FontFamily::Proportional),
                        ..Default::default()
                    },
                );
                job.append(
                    "3rd paragraph is kind of boring, but has italics.\nAnd a newline",
                    0.0,
                    TextFormat {
                        font_id: FontId::new(10.0, FontFamily::Proportional),
                        italics: true,
                        ..Default::default()
                    },
                );

                job
            },
            {
                // Regression test for <https://github.com/emilk/egui/issues/7378>
                let mut job = LayoutJob::default();
                job.append("\n", 0.0, TextFormat::default());
                job.append("", 0.0, TextFormat::default());
                job
            },
        ]
    }

    #[expect(clippy::print_stdout)]
    #[test]
    fn test_split_paragraphs() {
        for pixels_per_point in [1.0, 2.0_f32.sqrt(), 2.0] {
            let mut fonts = FontsImpl::new(TextOptions::default(), FontDefinitions::default());

            for halign in [Align::Min, Align::Center, Align::Max] {
                for justify in [false, true] {
                    for mut job in jobs() {
                        job.halign = halign;
                        job.justify = justify;

                        let whole = GalleyCache::default().layout(
                            &mut fonts,
                            pixels_per_point,
                            job.clone(),
                            false,
                        );

                        let split = GalleyCache::default().layout(
                            &mut fonts,
                            pixels_per_point,
                            job.clone(),
                            true,
                        );

                        for (i, row) in whole.rows.iter().enumerate() {
                            println!(
                                "Whole row {i}: section_index_at_start={}, first glyph section_index: {:?}",
                                row.row.section_index_at_start,
                                row.row.glyphs.first().map(|g| g.section_index)
                            );
                        }
                        for (i, row) in split.rows.iter().enumerate() {
                            println!(
                                "Split row {i}: section_index_at_start={}, first glyph section_index: {:?}",
                                row.row.section_index_at_start,
                                row.row.glyphs.first().map(|g| g.section_index)
                            );
                        }

                        // Don't compare for equaliity; but format with a specific precision and make sure we hit that.
                        // NOTE: we use a rather low precision, because as long as we're within a pixel I think it's good enough.
                        similar_asserts::assert_eq!(
                            format!("{:#.1?}", split),
                            format!("{:#.1?}", whole),
                            "pixels_per_point: {pixels_per_point:.2}, input text: '{}'",
                            job.text
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_intrinsic_size() {
        let pixels_per_point = [1.0, 1.3, 2.0, 0.867];
        let max_widths = [40.0, 80.0, 133.0, 200.0];
        let rounded_output_to_gui = [false, true];

        for pixels_per_point in pixels_per_point {
            let mut fonts = FontsImpl::new(TextOptions::default(), FontDefinitions::default());

            for &max_width in &max_widths {
                for round_output_to_gui in rounded_output_to_gui {
                    for mut job in jobs() {
                        job.wrap = TextWrapping::wrap_at_width(max_width);

                        job.round_output_to_gui = round_output_to_gui;

                        let galley_wrapped =
                            layout(&mut fonts, pixels_per_point, job.clone().into());

                        job.wrap = TextWrapping::no_max_width();

                        let text = job.text.clone();
                        let galley_unwrapped = layout(&mut fonts, pixels_per_point, job.into());

                        let intrinsic_size = galley_wrapped.intrinsic_size();
                        let unwrapped_size = galley_unwrapped.size();

                        let difference = (intrinsic_size - unwrapped_size).length().abs();
                        similar_asserts::assert_eq!(
                            format!("{intrinsic_size:.4?}"),
                            format!("{unwrapped_size:.4?}"),
                            "Wrapped intrinsic size should almost match unwrapped size. Intrinsic: {intrinsic_size:.8?} vs unwrapped: {unwrapped_size:.8?}
                                Difference: {difference:.8?}
                                wrapped rows: {}, unwrapped rows: {}
                                pixels_per_point: {pixels_per_point}, text: {text:?}, max_width: {max_width}, round_output_to_gui: {round_output_to_gui}",
                            galley_wrapped.rows.len(),
                            galley_unwrapped.rows.len()
                            );
                        similar_asserts::assert_eq!(
                            format!("{intrinsic_size:.4?}"),
                            format!("{unwrapped_size:.4?}"),
                            "Unwrapped galley intrinsic size should exactly match its size. \
                                {:.8?} vs {:8?}",
                            galley_unwrapped.intrinsic_size(),
                            galley_unwrapped.size(),
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_fallback_glyph_width() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::empty());
        let mut view = fonts.with_pixels_per_point(1.0);

        let width = view.glyph_width(&FontId::new(12.0, FontFamily::Proportional), ' ');
        assert_eq!(width, 0.0);
    }
}
