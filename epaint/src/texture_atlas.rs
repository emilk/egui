// TODO: `TextureData` or similar?
/// An 8-bit texture containing font data.
#[derive(Clone, Default)]
pub struct Texture {
    /// e.g. a hash of the data. Use this to detect changes!
    /// If the texture changes, this too will change.
    pub version: u64,
    pub width: usize,
    pub height: usize,
    /// White color with the given alpha (linear space 0-255).
    pub pixels: Vec<u8>,
}

impl Texture {
    /// Returns the textures as `sRGBA` premultiplied pixels, row by row, top to bottom.
    pub fn srgba_pixels(&'_ self) -> impl Iterator<Item = super::Color32> + '_ {
        use super::Color32;
        let srgba_from_luminance_lut: Vec<Color32> =
            (0..=255).map(Color32::from_white_alpha).collect();
        self.pixels
            .iter()
            .map(move |&l| srgba_from_luminance_lut[l as usize])
    }
}

impl std::ops::Index<(usize, usize)> for Texture {
    type Output = u8;

    fn index(&self, (x, y): (usize, usize)) -> &u8 {
        assert!(x < self.width);
        assert!(y < self.height);
        &self.pixels[y * self.width + x]
    }
}

impl std::ops::IndexMut<(usize, usize)> for Texture {
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
    texture: Texture,

    /// Used for when allocating new rectangles.
    cursor: (usize, usize),
    row_height: usize,
}

impl TextureAtlas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            texture: Texture {
                version: 0,
                width,
                height,
                pixels: vec![0; width * height],
            },
            ..Default::default()
        }
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn texture_mut(&mut self) -> &mut Texture {
        self.texture.version += 1;
        &mut self.texture
    }

    /// Returns the coordinates of where the rect ended up.
    pub fn allocate(&mut self, (w, h): (usize, usize)) -> (usize, usize) {
        /// On some low-precision GPUs (my old iPad) characters get muddled up
        /// if we don't add some empty pixels between the characters.
        /// On modern high-precision GPUs this is not needed.
        const PADDING: usize = 1;

        assert!(w <= self.texture.width);
        if self.cursor.0 + w > self.texture.width {
            // New row:
            self.cursor.0 = 0;
            self.cursor.1 += self.row_height + PADDING;
            self.row_height = 0;
        }

        self.row_height = self.row_height.max(h);
        while self.cursor.1 + self.row_height >= self.texture.height {
            self.texture.height *= 2;
        }

        if self.texture.width * self.texture.height > self.texture.pixels.len() {
            self.texture
                .pixels
                .resize(self.texture.width * self.texture.height, 0);
        }

        let pos = self.cursor;
        self.cursor.0 += w + PADDING;
        self.texture.version += 1;
        (pos.0 as usize, pos.1 as usize)
    }
}
