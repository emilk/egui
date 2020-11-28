use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::mutex::Mutex;

use super::{
    font::Font,
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
        let variable_typeface_data = include_bytes!("../../fonts/Comfortaa-Regular.ttf"); // Funny, hard to read

        let mut ttf_data: BTreeMap<FontFamily, &'static [u8]> = BTreeMap::new();
        ttf_data.insert(FontFamily::Monospace, monospace_typeface_data);
        ttf_data.insert(FontFamily::VariableWidth, variable_typeface_data);

        Self {
            pixels_per_point,
            fonts,
            ttf_data,
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

        let mut atlas = TextureAtlas::new(512, 16); // TODO: better default?

        {
            // Make the top left pixel fully white:
            let pos = atlas.allocate((1, 1));
            assert_eq!(pos, (0, 0));
            atlas.texture_mut()[pos] = 255;
        }

        let atlas = Arc::new(Mutex::new(atlas));

        self.definitions = definitions.clone();
        let FontDefinitions {
            pixels_per_point,
            fonts,
            ttf_data,
        } = definitions;
        self.fonts = fonts
            .into_iter()
            .map(|(text_style, (family, size))| {
                let typeface_data = ttf_data
                    .get(&family)
                    .unwrap_or_else(|| panic!("Missing TTF data for {:?}", family));
                let font = Font::new(atlas.clone(), typeface_data, size, pixels_per_point);

                (text_style, font)
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
