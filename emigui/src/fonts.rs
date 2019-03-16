use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

use crate::{
    font::Font,
    texture_atlas::{Texture, TextureAtlas},
};

/// TODO: rename
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum TextStyle {
    Body,
    Button,
    Heading,
    // Monospace,
}

pub type FontSizes = BTreeMap<TextStyle, f32>;

pub struct Fonts {
    pixels_per_point: f32,
    sizes: FontSizes,
    fonts: BTreeMap<TextStyle, Font>,
    texture: Texture,
}

impl Fonts {
    pub fn new(pixels_per_point: f32) -> Fonts {
        let mut sizes = FontSizes::new();
        sizes.insert(TextStyle::Body, 18.0);
        sizes.insert(TextStyle::Button, 22.0);
        sizes.insert(TextStyle::Heading, 28.0);
        Fonts::from_sizes(sizes, pixels_per_point)
    }

    pub fn from_sizes(sizes: FontSizes, pixels_per_point: f32) -> Fonts {
        let mut fonts = Fonts {
            pixels_per_point,
            sizes: Default::default(),
            fonts: Default::default(),
            texture: Default::default(),
        };
        fonts.set_sizes(sizes);
        fonts
    }

    pub fn sizes(&self) -> &FontSizes {
        &self.sizes
    }

    pub fn set_sizes(&mut self, sizes: FontSizes) {
        if self.sizes == sizes {
            return;
        }

        let mut atlas = TextureAtlas::new(512, 8); // TODO: better default?

        // Make one white pixel for use for various stuff:
        let pos = atlas.allocate((1, 1));
        atlas.texture_mut()[pos] = 255;

        let atlas = Arc::new(Mutex::new(atlas));

        // TODO: figure out a way to make the wasm smaller despite including a font. Zip it?
        let typeface_data = include_bytes!("../fonts/Comfortaa-Regular.ttf");
        // let typeface_data = include_bytes!("../fonts/DejaVuSans.ttf");
        // let typeface_data = include_bytes!("../fonts/ProggyClean.ttf"); // Use 13 for this. NOTHING ELSE.
        // let typeface_data = include_bytes!("../fonts/Roboto-Regular.ttf");
        self.sizes = sizes.clone();
        self.fonts = sizes
            .into_iter()
            .map(|(text_style, size)| {
                (
                    text_style,
                    Font::new(atlas.clone(), typeface_data, size, self.pixels_per_point),
                )
            })
            .collect();
        self.texture = atlas.lock().unwrap().texture().clone();

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
