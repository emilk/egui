/// A texture pixels, used for fonts.
#[derive(Clone, Default)]
pub struct TextureAtlas {
    width: usize,
    height: usize,
    pixels: Vec<u8>,

    /// Used for when adding new rects
    cursor: (usize, usize),
    row_height: usize,
}

impl TextureAtlas {
    pub fn new(width: usize, height: usize) -> Self {
        TextureAtlas {
            width: width,
            height: height,
            pixels: vec![0; width * height],
            ..Default::default()
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn into_texture(self) -> (u16, u16, Vec<u8>) {
        (self.width as u16, self.height as u16, self.pixels)
    }

    /// Returns the coordinates of where the rect ended up.
    pub fn allocate(&mut self, (w, h): (usize, usize)) -> (usize, usize) {
        assert!(w <= self.width);
        if self.cursor.0 + w > self.width {
            // New row:
            self.cursor.0 = 0;
            self.cursor.1 += self.row_height;
            self.row_height = 0;
        }

        self.row_height = self.row_height.max(h);
        while self.cursor.1 + self.row_height >= self.height {
            self.height *= 2;
        }

        if self.width * self.height > self.pixels.len() {
            self.pixels.resize(self.width * self.height, 0);
        }

        let pos = self.cursor;
        self.cursor.0 += w;
        (pos.0 as usize, pos.1 as usize)
    }
}

impl std::ops::Index<(usize, usize)> for TextureAtlas {
    type Output = u8;

    fn index(&self, (x, y): (usize, usize)) -> &u8 {
        assert!(x < self.width);
        assert!(y < self.height);
        &self.pixels[y * self.width + x]
    }
}

impl std::ops::IndexMut<(usize, usize)> for TextureAtlas {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut u8 {
        assert!(x < self.width);
        assert!(y < self.height);
        &mut self.pixels[y * self.width + x]
    }
}
