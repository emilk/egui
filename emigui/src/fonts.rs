use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use crate::{font::Font, texture_atlas::TextureAtlas};

/// TODO: rename
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum TextStyle {
    Body,
    Button,
    Heading,
    // Monospace,
}

pub struct Fonts {
    fonts: BTreeMap<TextStyle, Font>,
    texture: (u16, u16, Vec<u8>),
}

impl Fonts {
    pub fn new() -> Fonts {
        let mut atlas = TextureAtlas::new(128, 8); // TODO: better default?

        // Make one white pixel for use for various stuff:
        let pos = atlas.allocate((1, 1));
        atlas[pos] = 255;

        let atlas = Arc::new(Mutex::new(atlas));

        // TODO: figure out a way to make the wasm smaller despite including a font.
        // let font_data = include_bytes!("../fonts/ProggyClean.ttf"); // Use 13 for this. NOTHING ELSE.
        // let font_data = include_bytes!("../fonts/DejaVuSans.ttf");
        let font_data = include_bytes!("../fonts/Roboto-Regular.ttf");

        let mut fonts = BTreeMap::new();
        fonts.insert(TextStyle::Body, Font::new(atlas.clone(), font_data, 20));
        fonts.insert(TextStyle::Button, fonts[&TextStyle::Body].clone());
        fonts.insert(TextStyle::Heading, Font::new(atlas.clone(), font_data, 30));

        let texture = atlas.lock().unwrap().clone().into_texture();

        Fonts { fonts, texture }
    }

    pub fn texture(&self) -> (u16, u16, &[u8]) {
        (self.texture.0, self.texture.1, &self.texture.2)
    }
}

impl std::ops::Index<TextStyle> for Fonts {
    type Output = Font;

    fn index(&self, text_style: TextStyle) -> &Font {
        &self.fonts[&text_style]
    }
}
