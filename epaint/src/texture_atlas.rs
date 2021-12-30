/// An 8-bit texture containing font data.
#[derive(Clone, Default)]
pub struct FontImage {
    /// e.g. a hash of the data. Use this to detect changes!
    /// If the texture changes, this too will change.
    pub version: u64,
    pub width: usize,
    pub height: usize,
    /// White color with the given alpha (linear space 0-255).
    pub pixels: Vec<u8>,
}

impl FontImage {
    pub fn size(&self) -> [usize; 2] {
        [self.width, self.height]
    }

    /// Returns the textures as `sRGBA` premultiplied pixels, row by row, top to bottom.
    ///
    /// `gamma` should normally be set to 1.0.
    /// If you are having problems with egui text looking skinny and pixelated, try
    /// setting a lower gamma, e.g. `0.5`.
    pub fn srgba_pixels(&'_ self, gamma: f32) -> impl Iterator<Item = super::Color32> + '_ {
        use super::Color32;

        let srgba_from_luminance_lut: Vec<Color32> = (0..=255)
            .map(|a| {
                let a = super::color::linear_f32_from_linear_u8(a).powf(gamma);
                super::Rgba::from_white_alpha(a).into()
            })
            .collect();
        self.pixels
            .iter()
            .map(move |&l| srgba_from_luminance_lut[l as usize])
    }
}

impl std::ops::Index<(usize, usize)> for FontImage {
    type Output = u8;

    #[inline]
    fn index(&self, (x, y): (usize, usize)) -> &u8 {
        assert!(x < self.width);
        assert!(y < self.height);
        &self.pixels[y * self.width + x]
    }
}

impl std::ops::IndexMut<(usize, usize)> for FontImage {
    #[inline]
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut u8 {
        assert!(x < self.width);
        assert!(y < self.height);
        &mut self.pixels[y * self.width + x]
    }
}

/// Contains font data in an atlas, where each character occupied a small rectangle.
///
/// More characters can be added, possibly expanding the texture.
#[derive(Clone, Default)]
pub struct TextureAtlas {
    image: FontImage,

    /// Used for when allocating new rectangles.
    cursor: (usize, usize),
    row_height: usize,
}

impl TextureAtlas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            image: FontImage {
                version: 0,
                width,
                height,
                pixels: vec![0; width * height],
            },
            ..Default::default()
        }
    }

    pub fn image(&self) -> &FontImage {
        &self.image
    }

    pub fn image_mut(&mut self) -> &mut FontImage {
        self.image.version += 1;
        &mut self.image
    }

    /// Returns the coordinates of where the rect ended up.
    pub fn allocate(&mut self, (w, h): (usize, usize)) -> (usize, usize) {
        /// On some low-precision GPUs (my old iPad) characters get muddled up
        /// if we don't add some empty pixels between the characters.
        /// On modern high-precision GPUs this is not needed.
        const PADDING: usize = 1;

        assert!(
            w <= self.image.width,
            "Tried to allocate a {} wide glyph in a {} wide texture atlas",
            w,
            self.image.width
        );
        if self.cursor.0 + w > self.image.width {
            // New row:
            self.cursor.0 = 0;
            self.cursor.1 += self.row_height + PADDING;
            self.row_height = 0;
        }

        self.row_height = self.row_height.max(h);
        while self.cursor.1 + self.row_height >= self.image.height {
            self.image.height *= 2;
        }

        if self.image.width * self.image.height > self.image.pixels.len() {
            self.image
                .pixels
                .resize(self.image.width * self.image.height, 0);
        }

        let pos = self.cursor;
        self.cursor.0 += w + PADDING;
        self.image.version += 1;
        (pos.0 as usize, pos.1 as usize)
    }
}
