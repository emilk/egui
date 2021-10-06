use std::{collections::BTreeMap, sync::Arc};

use crate::{
    mutex::Mutex,
    text::{
        font::{Font, FontImpl},
        Galley, LayoutJob,
    },
    Texture, TextureAtlas,
};

// TODO: rename
/// One of a few categories of styles of text, e.g. body, button or heading.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TextStyle {
    /// Used when small text is needed.
    Small,
    /// Normal labels. Easily readable, doesn't take up too much space.
    Body,
    /// Buttons. Maybe slightly bigger than `Body`.
    Button,
    /// Heading. Probably larger than `Body`.
    Heading,
    /// Same size as `Body`, but used when monospace is important (for aligning number, code snippets, etc).
    Monospace,
}

impl TextStyle {
    pub fn all() -> impl Iterator<Item = TextStyle> {
        [
            TextStyle::Small,
            TextStyle::Body,
            TextStyle::Button,
            TextStyle::Heading,
            TextStyle::Monospace,
        ]
        .iter()
        .copied()
    }
}

/// Which style of font: [`Monospace`][`FontFamily::Monospace`] or [`Proportional`][`FontFamily::Proportional`].
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum FontFamily {
    /// A font where each character is the same width (`w` is the same width as `i`).
    Monospace,
    /// A font where some characters are wider than other (e.g. 'w' is wider than 'i').
    Proportional,
}

/// The data of a `.ttf` or `.otf` file.
pub type FontData = std::borrow::Cow<'static, [u8]>;

fn ab_glyph_font_from_font_data(name: &str, data: &FontData) -> ab_glyph::FontArc {
    match data {
        std::borrow::Cow::Borrowed(bytes) => ab_glyph::FontArc::try_from_slice(bytes),
        std::borrow::Cow::Owned(bytes) => ab_glyph::FontArc::try_from_vec(bytes.clone()),
    }
    .unwrap_or_else(|err| panic!("Error parsing {:?} TTF/OTF font file: {}", name, err))
}

/// Describes the font data and the sizes to use.
///
/// Often you would start with [`FontDefinitions::default()`] and then add/change the contents.
///
/// ```
/// # use {epaint::text::{FontDefinitions, TextStyle, FontFamily}};
/// # struct FakeEguiCtx {};
/// # impl FakeEguiCtx { fn set_fonts(&self, _: FontDefinitions) {} }
/// # let ctx = FakeEguiCtx {};
/// let mut fonts = FontDefinitions::default();
///
/// // Large button text:
/// fonts.family_and_size.insert(
///     TextStyle::Button,
///     (FontFamily::Proportional, 32.0)
/// );
///
/// ctx.set_fonts(fonts);
/// ```
///
/// You can also install your own custom fonts:
/// ```
/// # use {epaint::text::{FontDefinitions, TextStyle, FontFamily}};
/// # struct FakeEguiCtx {};
/// # impl FakeEguiCtx { fn set_fonts(&self, _: FontDefinitions) {} }
/// # let ctx = FakeEguiCtx {};
/// let mut fonts = FontDefinitions::default();
///
/// // Install my own font (maybe supporting non-latin characters):
/// fonts.font_data.insert("my_font".to_owned(),
///    std::borrow::Cow::Borrowed(include_bytes!("../../fonts/Ubuntu-Light.ttf"))); // .ttf and .otf supported
///
/// // Put my font first (highest priority):
/// fonts.fonts_for_family.get_mut(&FontFamily::Proportional).unwrap()
///     .insert(0, "my_font".to_owned());
///
/// // Put my font as last fallback for monospace:
/// fonts.fonts_for_family.get_mut(&FontFamily::Monospace).unwrap()
///     .push("my_font".to_owned());
///
/// ctx.set_fonts(fonts);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct FontDefinitions {
    /// List of font names and their definitions.
    /// The definition must be the contents of either a `.ttf` or `.otf` font file.
    ///
    /// `epaint` has built-in-default for these,
    /// but you can override them if you like.
    pub font_data: BTreeMap<String, FontData>,

    /// Which fonts (names) to use for each [`FontFamily`].
    ///
    /// The list should be a list of keys into [`Self::font_data`].
    /// When looking for a character glyph `epaint` will start with
    /// the first font and then move to the second, and so on.
    /// So the first font is the primary, and then comes a list of fallbacks in order of priority.
    pub fonts_for_family: BTreeMap<FontFamily, Vec<String>>,

    /// The [`FontFamily`] and size you want to use for a specific [`TextStyle`].
    pub family_and_size: BTreeMap<TextStyle, (FontFamily, f32)>,
}

impl Default for FontDefinitions {
    fn default() -> Self {
        #[allow(unused)]
        let mut font_data: BTreeMap<String, FontData> = BTreeMap::new();

        let mut fonts_for_family = BTreeMap::new();

        #[cfg(feature = "default_fonts")]
        {
            // TODO: figure out a way to make the WASM smaller despite including fonts. Zip them?

            // Use size 13 for this. NOTHING ELSE:
            font_data.insert(
                "ProggyClean".to_owned(),
                std::borrow::Cow::Borrowed(include_bytes!("../../fonts/ProggyClean.ttf")),
            );
            font_data.insert(
                "Ubuntu-Light".to_owned(),
                std::borrow::Cow::Borrowed(include_bytes!("../../fonts/Ubuntu-Light.ttf")),
            );

            // Some good looking emojis. Use as first priority:
            font_data.insert(
                "NotoEmoji-Regular".to_owned(),
                std::borrow::Cow::Borrowed(include_bytes!("../../fonts/NotoEmoji-Regular.ttf")),
            );
            // Bigger emojis, and more. <http://jslegers.github.io/emoji-icon-font/>:
            font_data.insert(
                "emoji-icon-font".to_owned(),
                std::borrow::Cow::Borrowed(include_bytes!("../../fonts/emoji-icon-font.ttf")),
            );

            fonts_for_family.insert(
                FontFamily::Monospace,
                vec![
                    "ProggyClean".to_owned(),
                    "Ubuntu-Light".to_owned(), // fallback for √ etc
                    "NotoEmoji-Regular".to_owned(),
                    "emoji-icon-font".to_owned(),
                ],
            );
            fonts_for_family.insert(
                FontFamily::Proportional,
                vec![
                    "Ubuntu-Light".to_owned(),
                    "NotoEmoji-Regular".to_owned(),
                    "emoji-icon-font".to_owned(),
                ],
            );
        }

        #[cfg(not(feature = "default_fonts"))]
        {
            fonts_for_family.insert(FontFamily::Monospace, vec![]);
            fonts_for_family.insert(FontFamily::Proportional, vec![]);
        }

        let mut family_and_size = BTreeMap::new();
        family_and_size.insert(TextStyle::Small, (FontFamily::Proportional, 10.0));
        family_and_size.insert(TextStyle::Body, (FontFamily::Proportional, 14.0));
        family_and_size.insert(TextStyle::Button, (FontFamily::Proportional, 14.0));
        family_and_size.insert(TextStyle::Heading, (FontFamily::Proportional, 20.0));
        family_and_size.insert(TextStyle::Monospace, (FontFamily::Monospace, 13.0)); // 13 for `ProggyClean`

        Self {
            font_data,
            fonts_for_family,
            family_and_size,
        }
    }
}

/// The collection of fonts used by `epaint`.
pub struct Fonts {
    pixels_per_point: f32,
    definitions: FontDefinitions,
    fonts: BTreeMap<TextStyle, Font>,
    atlas: Arc<Mutex<TextureAtlas>>,
    /// Copy of the texture in the texture atlas.
    /// This is so we can return a reference to it (the texture atlas is behind a lock).
    buffered_texture: Mutex<Arc<Texture>>,

    galley_cache: Mutex<GalleyCache>,
}

impl Fonts {
    pub fn new(pixels_per_point: f32, definitions: FontDefinitions) -> Self {
        assert!(
            0.0 < pixels_per_point && pixels_per_point < 100.0,
            "pixels_per_point out of range: {}",
            pixels_per_point
        );

        // We want an atlas big enough to be able to include all the Emojis in the `TextStyle::Heading`,
        // so we can show the Emoji picker demo window.
        let mut atlas = TextureAtlas::new(2048, 64);

        {
            // Make the top left pixel fully white:
            let pos = atlas.allocate((1, 1));
            assert_eq!(pos, (0, 0));
            atlas.texture_mut()[pos] = 255;
        }

        let atlas = Arc::new(Mutex::new(atlas));

        let mut font_impl_cache = FontImplCache::new(atlas.clone(), pixels_per_point, &definitions);

        let fonts = definitions
            .family_and_size
            .iter()
            .map(|(&text_style, &(family, scale_in_points))| {
                let fonts = &definitions.fonts_for_family.get(&family);
                let fonts = fonts.unwrap_or_else(|| {
                    panic!("FontFamily::{:?} is not bound to any fonts", family)
                });
                let fonts: Vec<Arc<FontImpl>> = fonts
                    .iter()
                    .map(|font_name| font_impl_cache.font_impl(font_name, scale_in_points))
                    .collect();

                (text_style, Font::new(text_style, fonts))
            })
            .collect();

        {
            let mut atlas = atlas.lock();
            let texture = atlas.texture_mut();
            // Make sure we seed the texture version with something unique based on the default characters:
            texture.version = crate::util::hash(&texture.pixels);
        }

        Self {
            pixels_per_point,
            definitions,
            fonts,
            atlas,
            buffered_texture: Default::default(), //atlas.lock().texture().clone();
            galley_cache: Default::default(),
        }
    }

    #[deprecated = "Renamed to Fonts::new"]
    pub fn from_definitions(pixels_per_point: f32, definitions: FontDefinitions) -> Self {
        Self::new(pixels_per_point, definitions)
    }

    #[inline(always)]
    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }

    pub fn definitions(&self) -> &FontDefinitions {
        &self.definitions
    }

    #[inline(always)]
    pub fn round_to_pixel(&self, point: f32) -> f32 {
        (point * self.pixels_per_point).round() / self.pixels_per_point
    }

    #[inline(always)]
    pub fn floor_to_pixel(&self, point: f32) -> f32 {
        (point * self.pixels_per_point).floor() / self.pixels_per_point
    }

    /// Call each frame to get the latest available font texture data.
    pub fn texture(&self) -> Arc<Texture> {
        let atlas = self.atlas.lock();
        let mut buffered_texture = self.buffered_texture.lock();
        if buffered_texture.version != atlas.texture().version {
            *buffered_texture = Arc::new(atlas.texture().clone());
        }

        buffered_texture.clone()
    }

    /// Width of this character in points.
    pub fn glyph_width(&self, text_style: TextStyle, c: char) -> f32 {
        self.fonts[&text_style].glyph_width(c)
    }

    /// Height of one row of text. In points
    pub fn row_height(&self, text_style: TextStyle) -> f32 {
        self.fonts[&text_style].row_height()
    }

    /// Layout some text.
    /// This is the most advanced layout function.
    /// See also [`Self::layout`], [`Self::layout_no_wrap`] and
    /// [`Self::layout_delayed_color`].
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    pub fn layout_job(&self, job: LayoutJob) -> Arc<Galley> {
        self.galley_cache.lock().layout(self, job)
    }

    /// Will wrap text at the given width and line break at `\n`.
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    pub fn layout(
        &self,
        text: String,
        text_style: TextStyle,
        color: crate::Color32,
        wrap_width: f32,
    ) -> Arc<Galley> {
        let job = LayoutJob::simple(text, text_style, color, wrap_width);
        self.layout_job(job)
    }

    /// Will line break at `\n`.
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    pub fn layout_no_wrap(
        &self,
        text: String,
        text_style: TextStyle,
        color: crate::Color32,
    ) -> Arc<Galley> {
        let job = LayoutJob::simple(text, text_style, color, f32::INFINITY);
        self.layout_job(job)
    }

    /// Like [`Self::layout`], made for when you want to pick a color for the text later.
    ///
    /// The implementation uses memoization so repeated calls are cheap.
    pub fn layout_delayed_color(
        &self,
        text: String,
        text_style: TextStyle,
        wrap_width: f32,
    ) -> Arc<Galley> {
        self.layout_job(LayoutJob::simple(
            text,
            text_style,
            crate::Color32::TEMPORARY_COLOR,
            wrap_width,
        ))
    }

    pub fn num_galleys_in_cache(&self) -> usize {
        self.galley_cache.lock().num_galleys_in_cache()
    }

    /// Must be called once per frame to clear the [`Galley`] cache.
    pub fn end_frame(&self) {
        self.galley_cache.lock().end_frame();
    }
}

impl std::ops::Index<TextStyle> for Fonts {
    type Output = Font;

    #[inline(always)]
    fn index(&self, text_style: TextStyle) -> &Font {
        &self.fonts[&text_style]
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
    fn layout(&mut self, fonts: &Fonts, job: LayoutJob) -> Arc<Galley> {
        let hash = crate::util::hash(&job); // TODO: even faster hasher?

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
    pub fn end_frame(&mut self) {
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
    ab_glyph_fonts: BTreeMap<String, ab_glyph::FontArc>,

    /// Map font names and size to the cached `FontImpl`.
    /// Can't have f32 in a HashMap or BTreeMap, so let's do a linear search
    cache: Vec<(String, f32, Arc<FontImpl>)>,
}

impl FontImplCache {
    pub fn new(
        atlas: Arc<Mutex<TextureAtlas>>,
        pixels_per_point: f32,
        definitions: &super::FontDefinitions,
    ) -> Self {
        let ab_glyph_fonts = definitions
            .font_data
            .iter()
            .map(|(name, font_data)| (name.clone(), ab_glyph_font_from_font_data(name, font_data)))
            .collect();

        Self {
            atlas,
            pixels_per_point,
            ab_glyph_fonts,
            cache: Default::default(),
        }
    }

    pub fn ab_glyph_font(&self, font_name: &str) -> ab_glyph::FontArc {
        self.ab_glyph_fonts
            .get(font_name)
            .unwrap_or_else(|| panic!("No font data found for {:?}", font_name))
            .clone()
    }

    pub fn font_impl(&mut self, font_name: &str, scale_in_points: f32) -> Arc<FontImpl> {
        for entry in &self.cache {
            if (entry.0.as_str(), entry.1) == (font_name, scale_in_points) {
                return entry.2.clone();
            }
        }

        let y_offset = if font_name == "emoji-icon-font" {
            scale_in_points * 0.235 // TODO: remove font alignment hack
        } else {
            0.0
        };
        let y_offset = y_offset - 3.0; // Tweaked to make text look centered in buttons and text edit fields

        let scale_in_points = if font_name == "emoji-icon-font" {
            scale_in_points * 0.8 // TODO: remove HACK!
        } else {
            scale_in_points
        };

        let font_impl = Arc::new(FontImpl::new(
            self.atlas.clone(),
            self.pixels_per_point,
            self.ab_glyph_font(font_name),
            scale_in_points,
            y_offset,
        ));
        self.cache
            .push((font_name.to_owned(), scale_in_points, font_impl.clone()));
        font_impl
    }
}
