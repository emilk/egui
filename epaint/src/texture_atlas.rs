use crate::image::AlphaImage;

/// An 8-bit texture containing font data.
#[derive(Clone, Default)]
pub struct FontImage {
    /// e.g. a hash of the data. Use this to detect changes!
    /// If the texture changes, this too will change.
    pub version: u64,

    /// The actual image data.
    pub image: AlphaImage,
}

impl FontImage {
    #[inline]
    pub fn size(&self) -> [usize; 2] {
        self.image.size
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.image.size[0]
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.image.size[1]
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
    pub fn new(size: [usize; 2]) -> Self {
        Self {
            image: FontImage {
                version: 0,
                image: AlphaImage::new(size),
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
            w <= self.image.width(),
            "Tried to allocate a {} wide glyph in a {} wide texture atlas",
            w,
            self.image.width()
        );
        if self.cursor.0 + w > self.image.width() {
            // New row:
            self.cursor.0 = 0;
            self.cursor.1 += self.row_height + PADDING;
            self.row_height = 0;
        }

        self.row_height = self.row_height.max(h);
        resize_to_min_height(&mut self.image.image, self.cursor.1 + self.row_height);

        let pos = self.cursor;
        self.cursor.0 += w + PADDING;
        self.image.version += 1;
        (pos.0 as usize, pos.1 as usize)
    }
}

fn resize_to_min_height(image: &mut AlphaImage, min_height: usize) {
    while min_height >= image.height() {
        image.size[1] *= 2; // double the height
    }

    if image.width() * image.height() > image.pixels.len() {
        image.pixels.resize(image.width() * image.height(), 0);
    }
}
