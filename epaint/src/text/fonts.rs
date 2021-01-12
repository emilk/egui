use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::{
    mutex::Mutex,
    text::font::{Font, FontImpl},
    Texture, TextureAtlas,
};

// TODO: rename
/// One of a few categories of styles of text, e.g. body, button or heading.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(rename_all = "snake_case"))]
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
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(rename_all = "snake_case"))]
pub enum FontFamily {
    /// A font where each character is the same width (`w` is the same width as `i`).
    Monospace,
    /// A font where some characters are wider than other (e.g. 'w' is wider than 'i').
    Proportional,
}

/// The data of a `.ttf` or `.otf` file.
pub type FontData = std::borrow::Cow<'static, [u8]>;

fn rusttype_font_from_font_data(name: &str, data: &FontData) -> rusttype::Font<'static> {
    match data {
        std::borrow::Cow::Borrowed(bytes) => rusttype::Font::try_from_bytes(bytes),
        std::borrow::Cow::Owned(bytes) => rusttype::Font::try_from_vec(bytes.clone()),
    }
    .unwrap_or_else(|| panic!("Error parsing {:?} TTF/OTF font file", name))
}

/// Describes the font data and the sizes to use.
///
/// This is how you can tell Egui which fonts and font sizes to use.
///
/// Often you would start with [`FontDefinitions::default()`] and then add/change the contents.
///
/// ``` ignore
/// # let mut ctx = egui::CtxRef::default();
/// let mut fonts = egui::FontDefinitions::default();
/// // Large button text:
/// fonts.family_and_size.insert(
///     egui::TextStyle::Button,
///     (egui::FontFamily::Proportional, 32.0));
/// ctx.set_fonts(fonts);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct FontDefinitions {
    /// List of font names and their definitions.
    /// The definition must be the contents of either a `.ttf` or `.otf` font file.
    ///
    /// Egui has built-in-default for these,
    /// but you can override them if you like.
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub font_data: BTreeMap<String, FontData>,

    /// Which fonts (names) to use for each [`FontFamily`].
    ///
    /// The list should be a list of keys into [`Self::font_data`].
    /// When looking for a character glyph Egui will start with
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
        family_and_size.insert(TextStyle::Button, (FontFamily::Proportional, 16.0));
        family_and_size.insert(TextStyle::Heading, (FontFamily::Proportional, 20.0));
        family_and_size.insert(TextStyle::Monospace, (FontFamily::Monospace, 13.0)); // 13 for `ProggyClean`

        Self {
            font_data,
            fonts_for_family,
            family_and_size,
        }
    }
}

/// The collection of fonts used by Egui.
///
/// Note: `Fonts::default()` is invalid (missing `pixels_per_point`).
#[derive(Default)]
pub struct Fonts {
    pixels_per_point: f32,
    definitions: FontDefinitions,
    fonts: BTreeMap<TextStyle, Font>,
    atlas: Arc<Mutex<TextureAtlas>>,
    /// Copy of the texture in the texture atlas.
    /// This is so we can return a reference to it (the texture atlas is behind a lock).
    buffered_texture: Mutex<Arc<Texture>>,
}

impl Fonts {
    pub fn from_definitions(pixels_per_point: f32, definitions: FontDefinitions) -> Self {
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

                (text_style, Font::new(fonts))
            })
            .collect();

        {
            let mut atlas = atlas.lock();
            let texture = atlas.texture_mut();
            // Make sure we seed the texture version with something unique based on the default characters:
            use std::collections::hash_map::DefaultHasher;
            let mut hasher = DefaultHasher::default();
            texture.pixels.hash(&mut hasher);
            texture.version = hasher.finish();
        }

        Self {
            pixels_per_point,
            definitions,
            fonts,
            atlas,
            buffered_texture: Default::default(), //atlas.lock().texture().clone();
        }
    }

    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }

    pub fn definitions(&self) -> &FontDefinitions {
        &self.definitions
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
}

impl std::ops::Index<TextStyle> for Fonts {
    type Output = Font;

    fn index(&self, text_style: TextStyle) -> &Font {
        &self.fonts[&text_style]
    }
}

// ----------------------------------------------------------------------------

struct FontImplCache {
    atlas: Arc<Mutex<TextureAtlas>>,
    pixels_per_point: f32,
    rusttype_fonts: std::collections::BTreeMap<String, Arc<rusttype::Font<'static>>>,

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
        let rusttype_fonts = definitions
            .font_data
            .iter()
            .map(|(name, font_data)| {
                (
                    name.clone(),
                    Arc::new(rusttype_font_from_font_data(name, font_data)),
                )
            })
            .collect();

        Self {
            atlas,
            pixels_per_point,
            rusttype_fonts,
            cache: Default::default(),
        }
    }

    pub fn rusttype_font(&self, font_name: &str) -> Arc<rusttype::Font<'static>> {
        self.rusttype_fonts
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
            self.rusttype_font(font_name),
            scale_in_points,
            y_offset,
        ));
        self.cache
            .push((font_name.to_owned(), scale_in_points, font_impl.clone()));
        font_impl
    }
}
