use std::{borrow::Cow, collections::BTreeMap, sync::Arc};

use crate::{
    text::{glyph_atlas::GlyphAtlas, Galley, LayoutJob},
    TextureAtlas,
};
use ecolor::Color32;
use emath::{vec2, GuiRounding, NumExt as _, OrderedFloat};

use parley::{
    fontique::{self, Blob, FontInfoOverride, QueryFamily},
    PositionedLayoutItem,
};

#[cfg(feature = "default_fonts")]
use epaint_default_fonts::{EMOJI_ICON, HACK_REGULAR, NOTO_EMOJI_REGULAR, UBUNTU_LIGHT};

use super::{
    glyph_atlas::SubpixelBin,
    style::{FontFamily, FontId, GenericFamily, TextFormat},
};

// ----------------------------------------------------------------------------

/// A `.ttf` or `.otf` file and a font face index.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontData {
    // TODO(valadaptive): the font definitions API takes an Arc<FontData> but Parley wants the data *itself* to be an Arc
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

    /// When using this font's metrics to layout a row,
    /// shift the entire row downwards by this fraction of the font size (in points).
    ///
    /// A positive value shifts the text downwards.
    /// A negative value shifts it upwards.
    pub baseline_offset_factor: f32,
}

impl Default for FontTweak {
    fn default() -> Self {
        Self {
            scale: 1.0,
            y_offset_factor: 0.0,
            y_offset: 0.0,
            baseline_offset_factor: 0.0,
        }
    }
}

// ----------------------------------------------------------------------------

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

    /// Which named font families to use for each [`GenericFamily`].
    ///
    /// The list should be a list of keys into [`Self::font_data`].
    /// When looking for a character glyph `epaint` will start with
    /// the first font and then move to the second, and so on.
    /// So the first font is the primary, and then comes a list of fallbacks in order of priority.
    pub families: BTreeMap<GenericFamily, Vec<String>>,

    /// Whether system fonts should also be loaded. Useful for supporting broad character sets without shipping large
    /// fonts, at the expense of load time.
    pub include_system_fonts: bool,
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

/// Optionally, you can add a font you insert to be used as a generic font family.
#[derive(Debug, Clone)]
pub struct InsertFontFamily {
    /// Font family
    pub family: GenericFamily,

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
            GenericFamily::Monospace,
            vec![
                "Hack".to_owned(),
                "Ubuntu-Light".to_owned(), // fallback for √ etc
                "NotoEmoji-Regular".to_owned(),
                "emoji-icon-font".to_owned(),
            ],
        );
        families.insert(
            GenericFamily::SystemUi,
            vec![
                "Ubuntu-Light".to_owned(),
                "NotoEmoji-Regular".to_owned(),
                "emoji-icon-font".to_owned(),
            ],
        );
        families.insert(
            GenericFamily::SansSerif,
            vec![
                "Ubuntu-Light".to_owned(),
                "NotoEmoji-Regular".to_owned(),
                "emoji-icon-font".to_owned(),
            ],
        );
        families.insert(
            GenericFamily::Emoji,
            vec!["NotoEmoji-Regular".to_owned(), "emoji-icon-font".to_owned()],
        );

        Self {
            font_data,
            families,
            include_system_fonts: false,
        }
    }
}

impl FontDefinitions {
    /// No fonts.
    pub fn empty() -> Self {
        let mut families = BTreeMap::new();
        families.insert(GenericFamily::Monospace, vec![]);
        families.insert(GenericFamily::SystemUi, vec![]);

        Self {
            font_data: Default::default(),
            families,
            include_system_fonts: false,
        }
    }

    /// Set whether [`Self::include_system_fonts`] is enabled.
    pub fn with_system_fonts(mut self, include_system_fonts: bool) -> Self {
        self.include_system_fonts = include_system_fonts;
        self
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

// ----------------------------------------------------------------------------

/// "View struct" of the fields of [`Fonts`] necessary to perform a layout job.
#[doc(hidden)]
pub(super) struct FontsLayoutView<'a> {
    pub font_context: &'a mut parley::FontContext,
    pub layout_context: &'a mut parley::LayoutContext<Color32>,
    pub texture_atlas: &'a mut TextureAtlas,
    pub glyph_atlas: &'a mut GlyphAtlas,
    pub font_tweaks: &'a mut ahash::HashMap<u64, FontTweak>,
    pub pixels_per_point: f32,
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
pub struct FontStore {
    max_texture_side: usize,
    definitions: FontDefinitions,
    font_tweaks: ahash::HashMap<u64, FontTweak>,
    atlas: TextureAtlas,
    galley_cache: GalleyCache,

    // TODO(valadaptive): glyph_width_cache and has_glyphs_cache should be frame-to-frame caches, but FrameCache is in
    // the egui crate
    glyph_width_cache: ahash::HashMap<char, f32>,
    has_glyphs_cache: ahash::HashMap<(Cow<'static, str>, Cow<'static, FontId>), bool>,
    all_families: Option<Vec<FontFamily>>,

    pub(super) font_context: parley::FontContext,
    pub(super) layout_context: parley::LayoutContext<Color32>,
    pub(super) glyph_atlas: GlyphAtlas,
}

impl FontStore {
    /// Create a new [`Fonts`] for text layout.
    /// This call is expensive, so only create one [`Fonts`] and then reuse it.
    ///
    /// * `max_texture_side`: largest supported texture size (one side).
    pub fn new(max_texture_side: usize, definitions: FontDefinitions) -> Self {
        let texture_width = max_texture_side.at_most(16 * 1024).at_most(
            1024, /* limit atlas size to test that multiple atlases work */
        );
        // Keep initial font atlas small, so it is fast to upload to GPU. This will expand as needed anyways.
        let initial_height = 32;
        let atlas = TextureAtlas::new([texture_width, initial_height]);

        let collection = fontique::Collection::new(fontique::CollectionOptions {
            shared: false,
            system_fonts: definitions.include_system_fonts,
        });

        let mut font_store = Self {
            max_texture_side,
            definitions,
            font_tweaks: Default::default(),
            glyph_atlas: GlyphAtlas::new(),
            atlas,
            galley_cache: Default::default(),

            glyph_width_cache: Default::default(),
            has_glyphs_cache: Default::default(),
            all_families: Default::default(),

            font_context: parley::FontContext {
                collection,
                source_cache: fontique::SourceCache::new_shared(),
            },
            layout_context: parley::LayoutContext::new(),
        };

        font_store.load_fonts_from_definitions();

        font_store
    }

    /// Call at the start of each frame with the latest known `max_texture_side`.
    ///
    /// Call after painting the previous frame, but before using [`Fonts`] for the new frame.
    ///
    /// This function will react to changes in `max_texture_side`, as well as notice when the font atlas is getting
    /// full, and handle that.
    pub fn begin_pass(&mut self, max_texture_side: usize) {
        let max_texture_side_changed = self.max_texture_side != max_texture_side;
        // TODO(valadaptive): this seems suspicious. Does this mean the atlas can never use more than 80% of its actual
        // capacity?
        let font_atlas_almost_full = self.atlas.fill_ratio() > 0.8;
        let needs_recreate = max_texture_side_changed || font_atlas_almost_full;

        if needs_recreate {
            self.clear_atlas(max_texture_side);
        }

        self.galley_cache.flush_cache();
        // TODO(valadaptive): make this configurable?
        self.font_context.source_cache.prune(250, false);
    }

    fn load_fonts_from_definitions(&mut self) {
        for (name, data) in &self.definitions.font_data {
            // TODO(valadaptive): in the case where we're just adding new fonts, we can probably reuse the blobs
            let blob = Blob::new(Arc::new(data.font.clone()));
            self.font_tweaks.insert(blob.id(), data.tweak);
            // TODO(valadaptive): we completely ignore the font index because fontique only lets us load all the fonts
            self.font_context.collection.register_fonts(
                blob,
                Some(FontInfoOverride {
                    family_name: Some(name),
                    ..Default::default()
                }),
            );
        }

        for (generic_family, family_fonts) in &self.definitions.families {
            let family_ids: Vec<_> = family_fonts
                .iter()
                .filter_map(|family_name| self.font_context.collection.family_id(family_name))
                .collect();

            self.font_context
                .collection
                .set_generic_families(generic_family.as_parley(), family_ids.into_iter());
        }

        self.clear_cache(self.max_texture_side);
    }

    pub fn with_pixels_per_point(&mut self, pixels_per_point: f32) -> Fonts<'_> {
        Fonts {
            fonts: self,
            pixels_per_point,
        }
    }

    #[inline]
    pub fn definitions(&self) -> &FontDefinitions {
        &self.definitions
    }

    pub fn set_definitions(&mut self, definitions: FontDefinitions) {
        // We need to recreate the font collection if we start or stop loading system fonts
        if definitions.include_system_fonts != self.definitions.include_system_fonts {
            self.font_context.collection = fontique::Collection::new(fontique::CollectionOptions {
                shared: false,
                system_fonts: self.definitions.include_system_fonts,
            });
        } else {
            self.font_context.collection.clear();
        }

        self.definitions = definitions;
        self.font_tweaks.clear();
        self.font_context.source_cache.prune(0, true);
        self.clear_cache(self.max_texture_side);
        self.load_fonts_from_definitions();
    }

    /// Width of this character in points.
    pub fn glyph_width(&mut self, font_id: &FontId, c: char) -> f32 {
        *self.glyph_width_cache.entry(c).or_insert_with(|| {
            let text = c.to_string();
            let text_style = TextFormat::simple(font_id.clone(), Default::default()).as_parley();
            let mut builder =
                self.layout_context
                    .tree_builder(&mut self.font_context, 1.0, &text_style);
            builder.push_text(&text);
            let (mut layout, _) = builder.build();
            layout.break_lines().break_next(f32::MAX);
            let Some(first_line) = layout.lines().next() else {
                return 0.0;
            };
            first_line.metrics().advance
        })
    }

    pub fn preload_common_characters(&mut self, font_id: &FontId) {
        // LAST_ASCII - FIRST_ASCII + 1 ASCII characters (since the range is inclusive) + the degrees symbol and
        // password replacement character
        let mut common_chars = String::with_capacity(LAST_ASCII - FIRST_ASCII + 3);

        // Preload the printable ASCII characters [32, 126] (which excludes control codes):
        const FIRST_ASCII: usize = 32; // 32 == space
        const LAST_ASCII: usize = 126;
        for c in (FIRST_ASCII..=LAST_ASCII).map(|c| c as u8 as char) {
            common_chars.push(c);
        }
        common_chars.push('°');
        common_chars.push(crate::text::PASSWORD_REPLACEMENT_CHAR);

        self.preload_text(1.0, font_id, &common_chars);
    }

    fn preload_text(&mut self, pixels_per_point: f32, font_id: &FontId, text: &str) {
        let style = TextFormat {
            font_id: font_id.clone(),
            ..Default::default()
        };
        let style = style.as_parley();
        let mut builder = self
            .layout_context
            .tree_builder(&mut self.font_context, 1.0, &style);
        builder.push_text(text);
        let (mut layout, _) = builder.build();
        layout.break_all_lines(None);
        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(run) = item else {
                    continue;
                };

                for x_offset in SubpixelBin::SUBPIXEL_OFFSETS {
                    self.glyph_atlas
                        .render_glyph_run(
                            &mut self.atlas,
                            &run,
                            vec2(x_offset, 0.0),
                            pixels_per_point,
                            &self.font_tweaks,
                        )
                        .for_each(|_| {});
                }
            }
        }
    }

    pub fn has_glyphs_for(&mut self, font_id: &FontId, text: &str) -> bool {
        if let Some(has_glyphs) = self
            .has_glyphs_cache
            .get(&(text.into(), Cow::Borrowed(font_id)))
        {
            return *has_glyphs;
        }

        *self
            .has_glyphs_cache
            .entry((Cow::Owned(text.to_owned()), Cow::Owned(font_id.clone())))
            .or_insert_with(|| {
                let style = TextFormat {
                    font_id: font_id.clone(),
                    ..Default::default()
                };
                let style = style.as_parley();
                let mut builder =
                    self.layout_context
                        .tree_builder(&mut self.font_context, 1.0, &style);
                builder.push_text(text);
                let (mut layout, _) = builder.build();
                layout.break_all_lines(None);

                for line in layout.lines() {
                    for item in line.items() {
                        let PositionedLayoutItem::GlyphRun(run) = item else {
                            continue;
                        };

                        for glyph in run.glyphs() {
                            if glyph.id == 0 {
                                return false;
                            }
                        }
                    }
                }

                true
            })
    }

    pub fn with_characters(&mut self, font_family: &FontFamily, cb: impl FnMut(u32, u16)) {
        let mut query = self
            .font_context
            .collection
            .query(&mut self.font_context.source_cache);
        query.set_families(std::iter::once(match font_family {
            FontFamily::Named(cow) => QueryFamily::Named(cow),
            FontFamily::Generic(generic_family) => QueryFamily::Generic(generic_family.as_parley()),
        }));

        let mut font_data = None;

        query.matches_with(|font| {
            font_data = Some((font.blob.clone(), font.index));
            fontique::QueryStatus::Stop
        });

        let Some((font_data, font_index)) = font_data else {
            return;
        };

        let Some(swash_font) =
            parley::swash::FontRef::from_index(font_data.as_ref(), font_index as usize)
        else {
            return;
        };

        swash_font.charmap().enumerate(cb);
    }

    /// Height of one row of text in points.
    #[allow(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    pub fn row_height(&mut self, font_id: &FontId) -> f32 {
        // TODO(valadaptive): if styling is changed so line height is more overridable, this function won't make very
        // much sense
        font_id.size
    }

    fn clear_atlas(&mut self, new_max_texture_side: usize) {
        self.atlas.clear();
        self.glyph_atlas.clear();
        self.galley_cache.clear();
        self.max_texture_side = new_max_texture_side;
    }

    fn clear_cache(&mut self, new_max_texture_side: usize) {
        self.clear_atlas(new_max_texture_side);
        self.glyph_width_cache.clear();
        self.has_glyphs_cache.clear();
        self.all_families = None;
    }

    /// Call at the end of each frame (before painting) to get the change to the font texture since last call.
    pub fn font_image_delta(&mut self) -> Option<crate::ImageDelta> {
        self.atlas.take_delta()
    }

    #[inline]
    pub fn max_texture_side(&self) -> usize {
        self.max_texture_side
    }

    /// The font atlas.
    /// Pass this to [`crate::Tessellator`].
    pub fn texture_atlas(&self) -> &TextureAtlas {
        &self.atlas
    }

    /// Current size of the font image.
    /// Pass this to [`crate::Tessellator`].
    pub fn font_image_size(&self) -> [usize; 2] {
        self.atlas.size()
    }

    /// List of all loaded font families.
    pub fn families(&mut self) -> &[FontFamily] {
        self.all_families.get_or_insert_with(|| {
            let mut all_families = self
                .font_context
                .collection
                .family_names()
                .map(|name| FontFamily::Named(Cow::Owned(name.to_owned())))
                .collect::<Vec<_>>();

            all_families.sort_by_cached_key(|f| match f {
                FontFamily::Named(name) => name.to_lowercase(),
                FontFamily::Generic(_) => unreachable!(),
            });

            all_families
        })
    }

    pub fn num_galleys_in_cache(&self) -> usize {
        self.galley_cache.num_galleys_in_cache()
    }

    /// How full is the font atlas?
    ///
    /// This increases as new fonts and/or glyphs are used,
    /// but can also decrease in a call to [`Self::begin_pass`].
    pub fn font_atlas_fill_ratio(&self) -> f32 {
        self.atlas.fill_ratio()
    }
}

// ----------------------------------------------------------------------------

/// View into a [`FontStore`] that lets you perform text layout at a given DPI.
pub struct Fonts<'a> {
    fonts: &'a mut FontStore,
    pixels_per_point: f32,
}

impl Fonts<'_> {
    #[inline]
    pub fn definitions(&self) -> &FontDefinitions {
        self.fonts.definitions()
    }

    /// Width of this character in points.
    pub fn glyph_width(&mut self, font_id: &FontId, c: char) -> f32 {
        self.fonts.glyph_width(font_id, c)
    }

    /// Can we display all the glyphs in this text?
    pub fn has_glyphs_for(&mut self, font_id: &FontId, s: &str) -> bool {
        self.fonts.has_glyphs_for(font_id, s)
    }
    pub fn with_characters(&mut self, font_family: &FontFamily, cb: impl FnMut(u32, u16)) {
        self.fonts.with_characters(font_family, cb);
    }

    /// Height of one row of text in points.
    ///
    /// Returns a value rounded to [`emath::GUI_ROUNDING`].
    pub fn row_height(&mut self, font_id: &FontId) -> f32 {
        self.fonts
            .row_height(font_id)
            .round_to_pixels(self.pixels_per_point)
    }

    /// Call at the end of each frame (before painting) to get the change to the font texture since last call.
    pub fn font_image_delta(&mut self) -> Option<crate::ImageDelta> {
        self.fonts.font_image_delta()
    }

    #[inline]
    pub fn max_texture_side(&self) -> usize {
        self.fonts.max_texture_side()
    }

    /// The font atlas.
    /// Pass this to [`crate::Tessellator`].
    pub fn texture_atlas(&self) -> &TextureAtlas {
        self.fonts.texture_atlas()
    }

    /// Current size of the font image.
    /// Pass this to [`crate::Tessellator`].
    pub fn font_image_size(&self) -> [usize; 2] {
        self.fonts.font_image_size()
    }

    /// List of all loaded font families.
    pub fn families(&mut self) -> &[FontFamily] {
        self.fonts.families()
    }

    pub fn num_galleys_in_cache(&self) -> usize {
        self.fonts.num_galleys_in_cache()
    }

    /// How full is the font atlas?
    ///
    /// This increases as new fonts and/or glyphs are used,
    /// but can also decrease in a call to [`Self::begin_pass`].
    pub fn font_atlas_fill_ratio(&self) -> f32 {
        self.fonts.font_atlas_fill_ratio()
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
        self.fonts.galley_cache.layout(
            &mut FontsLayoutView {
                font_context: &mut self.fonts.font_context,
                layout_context: &mut self.fonts.layout_context,
                texture_atlas: &mut self.fonts.atlas,
                glyph_atlas: &mut self.fonts.glyph_atlas,
                font_tweaks: &mut self.fonts.font_tweaks,
                pixels_per_point: self.pixels_per_point,
            },
            job,
            self.pixels_per_point,
        )
    }

    /// Layout some text, without memoization.
    ///
    /// Mostly useful for benchmarking.
    #[inline]
    #[doc(hidden)]
    pub fn layout_job_uncached(&mut self, job: LayoutJob) -> Arc<Galley> {
        Arc::new(super::parley_layout::layout(
            &mut FontsLayoutView {
                font_context: &mut self.fonts.font_context,
                layout_context: &mut self.fonts.layout_context,
                texture_atlas: &mut self.fonts.atlas,
                glyph_atlas: &mut self.fonts.glyph_atlas,
                font_tweaks: &mut self.fonts.font_tweaks,
                pixels_per_point: self.pixels_per_point,
            },
            job,
        ))
    }

    /// Will wrap text at the given width and line break at `\n`.
    ///
    /// The implementation uses memoization so repeated calls are cheap.
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
    fn layout(
        &mut self,
        fonts: &mut FontsLayoutView<'_>,
        mut job: LayoutJob,
        pixels_per_point: f32,
    ) -> Arc<Galley> {
        if job.wrap.max_width.is_finite() {
            // Protect against rounding errors in egui layout code.

            // Say the user asks to wrap at width 200.0.
            // The text layout wraps, and reports that the final width was 196.0 points.
            // This than trickles up the `Ui` chain and gets stored as the width for a tooltip (say).
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

        match self.cache.entry(hash) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                let cached = entry.into_mut();
                cached.last_used = self.generation;
                cached.galley.clone()
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                //let galley = super::layout(fonts, job.into());
                let galley = super::parley_layout::layout(fonts, job);
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

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}
