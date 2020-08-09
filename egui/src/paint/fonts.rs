use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    sync::Arc,
};

use parking_lot::Mutex;

use super::{
    font::Font,
    texture_atlas::{Texture, TextureAtlas},
};

// TODO: rename
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TextStyle {
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
}

impl Default for FontDefinitions {
    fn default() -> Self {
        Self::with_pixels_per_point(f32::NAN) // must be set later
    }
}

impl FontDefinitions {
    pub fn with_pixels_per_point(pixels_per_point: f32) -> Self {
        let mut fonts = BTreeMap::new();
        fonts.insert(TextStyle::Body, (FontFamily::VariableWidth, 14.0));
        fonts.insert(TextStyle::Button, (FontFamily::VariableWidth, 16.0));
        fonts.insert(TextStyle::Heading, (FontFamily::VariableWidth, 24.0));
        fonts.insert(TextStyle::Monospace, (FontFamily::Monospace, 13.0));

        Self {
            pixels_per_point,
            fonts,
        }
    }
}

/// Note: the `default()` fonts are invalid (missing `pixels_per_point`).
#[derive(Default)]
pub struct Fonts {
    definitions: FontDefinitions,
    fonts: BTreeMap<TextStyle, Font>,
    texture: Texture,
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

        let mut atlas = TextureAtlas::new(512, 8); // TODO: better default?

        // Make the top left four pixels fully white:
        let pos = atlas.allocate((2, 2));
        assert_eq!(pos, (0, 0));
        atlas.texture_mut()[(0, 0)] = 255;
        atlas.texture_mut()[(0, 1)] = 255;
        atlas.texture_mut()[(1, 0)] = 255;
        atlas.texture_mut()[(1, 1)] = 255;

        let atlas = Arc::new(Mutex::new(atlas));

        // TODO: figure out a way to make the wasm smaller despite including a font. Zip it?
        let monospae_typeface_data = include_bytes!("../../fonts/ProggyClean.ttf"); // Use 13 for this. NOTHING ELSE.

        // let monospae_typeface_data = include_bytes!("../../fonts/Roboto-Regular.ttf");

        let variable_typeface_data = include_bytes!("../../fonts/Comfortaa-Regular.ttf"); // Funny, hard to read

        // let variable_typeface_data = include_bytes!("../../fonts/DejaVuSans.ttf"); // Basic, boring, takes up more space

        self.definitions = definitions.clone();
        let FontDefinitions {
            pixels_per_point,
            fonts,
        } = definitions;
        self.fonts = fonts
            .into_iter()
            .map(|(text_style, (family, size))| {
                let typeface_data: &[u8] = match family {
                    FontFamily::Monospace => monospae_typeface_data,
                    FontFamily::VariableWidth => variable_typeface_data,
                };

                (
                    text_style,
                    Font::new(atlas.clone(), typeface_data, size, pixels_per_point),
                )
            })
            .collect();
        self.texture = atlas.lock().texture().clone();

        let mut hasher = ahash::AHasher::default();
        self.texture.pixels.hash(&mut hasher);
        self.texture.id = hasher.finish();
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}

impl std::ops::Index<TextStyle> for Fonts {
    type Output = Font;

    fn index(&self, text_style: TextStyle) -> &Font {
        &self.fonts[&text_style]
    }
}
