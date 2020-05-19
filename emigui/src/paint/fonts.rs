use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    hash::{Hash, Hasher},
    sync::Arc,
};

use {parking_lot::Mutex, serde_derive::Serialize};

use super::{
    font::Font,
    texture_atlas::{Texture, TextureAtlas},
};

/// TODO: rename
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum TextStyle {
    Body,
    Button,
    Heading,
    Monospace,
}

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum FontFamily {
    Monospace,
    VariableWidth,
}

pub type FontDefinitions = BTreeMap<TextStyle, (FontFamily, f32)>;

pub fn default_font_definitions() -> FontDefinitions {
    let mut definitions = FontDefinitions::new();
    definitions.insert(TextStyle::Body, (FontFamily::VariableWidth, 14.0));
    definitions.insert(TextStyle::Button, (FontFamily::VariableWidth, 16.0));
    definitions.insert(TextStyle::Heading, (FontFamily::VariableWidth, 24.0));
    definitions.insert(TextStyle::Monospace, (FontFamily::Monospace, 13.0));
    definitions
}

pub struct Fonts {
    pixels_per_point: f32,
    definitions: FontDefinitions,
    fonts: BTreeMap<TextStyle, Font>,
    texture: Texture,
}

impl Fonts {
    pub fn new(pixels_per_point: f32) -> Fonts {
        Fonts::from_definitions(default_font_definitions(), pixels_per_point)
    }

    pub fn from_definitions(definitions: FontDefinitions, pixels_per_point: f32) -> Fonts {
        let mut fonts = Fonts {
            pixels_per_point,
            definitions: Default::default(),
            fonts: Default::default(),
            texture: Default::default(),
        };
        fonts.set_sizes(definitions);
        fonts
    }

    pub fn definitions(&self) -> &FontDefinitions {
        &self.definitions
    }

    pub fn set_sizes(&mut self, definitions: FontDefinitions) {
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
        self.fonts = definitions
            .into_iter()
            .map(|(text_style, (family, size))| {
                let typeface_data: &[u8] = match family {
                    FontFamily::Monospace => monospae_typeface_data,
                    FontFamily::VariableWidth => variable_typeface_data,
                };

                (
                    text_style,
                    Font::new(atlas.clone(), typeface_data, size, self.pixels_per_point),
                )
            })
            .collect();
        self.texture = atlas.lock().texture().clone();

        let mut hasher = DefaultHasher::new();
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
