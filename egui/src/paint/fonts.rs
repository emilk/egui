use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::mutex::Mutex;

use super::{
    font::{Font, FontImpl},
    texture_atlas::{Texture, TextureAtlas},
};

// TODO: rename
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TextStyle {
    Small,
    Body,
    Button,
    Heading,
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

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FontFamily {
    Monospace,
    VariableWidth,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontDefinitions {
    /// The dpi scale factor. Needed to get pixel perfect fonts.
    pub pixels_per_point: f32,

    pub fonts: BTreeMap<TextStyle, (FontFamily, f32)>,

    /// The TTF data for each font family.
    /// Egui has built-in-default for these,
    /// but you can override them if you like.
    pub ttf_data: BTreeMap<FontFamily, &'static [u8]>,

    /// ttf data for emoji font(s), if any, in order of preference
    pub emoji_ttf_data: Vec<&'static [u8]>,
}

impl Default for FontDefinitions {
    fn default() -> Self {
        Self::with_pixels_per_point(f32::NAN) // must be set later
    }
}

impl FontDefinitions {
    pub fn with_pixels_per_point(pixels_per_point: f32) -> Self {
        let mut fonts = BTreeMap::new();
        fonts.insert(TextStyle::Small, (FontFamily::VariableWidth, 10.0));
        fonts.insert(TextStyle::Body, (FontFamily::VariableWidth, 14.0));
        fonts.insert(TextStyle::Button, (FontFamily::VariableWidth, 16.0));
        fonts.insert(TextStyle::Heading, (FontFamily::VariableWidth, 24.0));
        fonts.insert(TextStyle::Monospace, (FontFamily::Monospace, 13.0)); // 13 for `ProggyClean`

        // TODO: figure out a way to make the WASM smaller despite including a font. Zip it?
        let monospace_typeface_data = include_bytes!("../../fonts/ProggyClean.ttf"); // Use 13 for this. NOTHING ELSE.
        let variable_typeface_data = include_bytes!("../../fonts/Ubuntu-Light.ttf");

        let mut ttf_data: BTreeMap<FontFamily, &'static [u8]> = BTreeMap::new();
        ttf_data.insert(FontFamily::Monospace, monospace_typeface_data);
        ttf_data.insert(FontFamily::VariableWidth, variable_typeface_data);

        Self {
            pixels_per_point,
            fonts,
            ttf_data,
            emoji_ttf_data: vec![
                include_bytes!("../../fonts/NotoEmoji-Regular.ttf"), // few, but good looking. Use as first priority
                include_bytes!("../../fonts/emoji-icon-font.ttf"), // bigger and more: http://jslegers.github.io/emoji-icon-font/
            ],
        }
    }
}

/// Note: the `default()` fonts are invalid (missing `pixels_per_point`).
#[derive(Default)]
pub struct Fonts {
    definitions: FontDefinitions,
    fonts: BTreeMap<TextStyle, Font>,
    atlas: Arc<Mutex<TextureAtlas>>,
    /// Copy of the texture in the texture atlas.
    /// This is so we can return a reference to it (the texture atlas is behind a lock).
    buffered_texture: Mutex<Arc<Texture>>,
}

impl Fonts {
    pub fn from_definitions(definitions: FontDefinitions) -> Fonts {
        let mut fonts = Self::default();
        fonts.set_definitions(definitions);
        fonts
    }

    pub fn definitions(&self) -> &FontDefinitions {
        &self.definitions
    }

    pub fn set_definitions(&mut self, definitions: FontDefinitions) {
        if self.definitions == definitions {
            return;
        }

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

        self.definitions = definitions;

        let mut font_impl_cache = FontImplCache::new(atlas.clone(), &self.definitions);

        self.fonts = self
            .definitions
            .fonts
            .iter()
            .map(|(&text_style, &(family, size))| {
                let mut fonts = vec![];

                fonts.push(font_impl_cache.font_impl(FontSource::Family(family), size));

                if family == FontFamily::Monospace {
                    // monospace should have ubuntu as fallback (for √ etc):
                    fonts.push(
                        font_impl_cache
                            .font_impl(FontSource::Family(FontFamily::VariableWidth), size),
                    );
                }

                for index in 0..self.definitions.emoji_ttf_data.len() {
                    let emoji_font_impl = font_impl_cache.font_impl(FontSource::Emoji(index), size);
                    fonts.push(emoji_font_impl);
                }

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

        self.buffered_texture = Default::default(); //atlas.lock().texture().clone();
        self.atlas = atlas;
    }

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

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FontSource {
    Family(FontFamily),
    /// Emoji fonts are numbered from hight priority (0) and onwards
    Emoji(usize),
}

pub struct FontImplCache {
    atlas: Arc<Mutex<TextureAtlas>>,
    pixels_per_point: f32,
    font_families: std::collections::BTreeMap<FontFamily, Arc<rusttype::Font<'static>>>,
    emoji_fonts: Vec<Arc<rusttype::Font<'static>>>,

    /// can't have f32 in a HashMap or BTreeMap,
    /// so let's do a linear search
    cache: Vec<(FontSource, f32, Arc<FontImpl>)>,
}

impl FontImplCache {
    pub fn new(atlas: Arc<Mutex<TextureAtlas>>, definitions: &super::FontDefinitions) -> Self {
        let font_families = definitions
            .ttf_data
            .iter()
            .map(|(family, ttf_data)| {
                (
                    *family,
                    Arc::new(rusttype::Font::try_from_bytes(ttf_data).expect("Error parsing TTF")),
                )
            })
            .collect();

        let emoji_fonts = definitions
            .emoji_ttf_data
            .iter()
            .map(|ttf_data| {
                Arc::new(rusttype::Font::try_from_bytes(ttf_data).expect("Error parsing TTF"))
            })
            .collect();

        Self {
            atlas,
            pixels_per_point: definitions.pixels_per_point,
            font_families,
            emoji_fonts,
            cache: Default::default(),
        }
    }

    pub fn rusttype_font(&self, source: FontSource) -> Arc<rusttype::Font<'static>> {
        match source {
            FontSource::Family(family) => self.font_families.get(&family).unwrap().clone(),
            FontSource::Emoji(index) => self.emoji_fonts[index].clone(),
        }
    }

    pub fn font_impl(&mut self, source: FontSource, scale_in_points: f32) -> Arc<FontImpl> {
        for entry in &self.cache {
            if (entry.0, entry.1) == (source, scale_in_points) {
                return entry.2.clone();
            }
        }

        let font_impl = Arc::new(FontImpl::new(
            self.atlas.clone(),
            self.pixels_per_point,
            self.rusttype_font(source),
            scale_in_points,
        ));
        self.cache
            .push((source, scale_in_points, font_impl.clone()));
        font_impl
    }
}
