use crate::{FontImage, ImageDelta};

#[derive(Clone, Copy, Eq, PartialEq)]
struct Rectu {
    /// inclusive
    min_x: usize,
    /// inclusive
    min_y: usize,
    /// exclusive
    max_x: usize,
    /// exclusive
    max_y: usize,
}

impl Rectu {
    const NOTHING: Self = Self {
        min_x: usize::MAX,
        min_y: usize::MAX,
        max_x: 0,
        max_y: 0,
    };
    const EVERYTHING: Self = Self {
        min_x: 0,
        min_y: 0,
        max_x: usize::MAX,
        max_y: usize::MAX,
    };
}

/// Contains font data in an atlas, where each character occupied a small rectangle.
///
/// More characters can be added, possibly expanding the texture.
#[derive(Clone)]
pub struct TextureAtlas {
    image: FontImage,
    /// What part of the image that is dirty
    dirty: Rectu,

    /// Used for when allocating new rectangles.
    cursor: (usize, usize),
    row_height: usize,

    /// Set when someone requested more space than was available.
    overflowed: bool,
}

impl TextureAtlas {
    pub fn new(size: [usize; 2]) -> Self {
        assert!(size[0] >= 1024, "Tiny texture atlas");
        Self {
            image: FontImage::new(size),
            dirty: Rectu::EVERYTHING,
            cursor: (0, 0),
            row_height: 0,
            overflowed: false,
        }
    }

    pub fn size(&self) -> [usize; 2] {
        self.image.size
    }

    fn max_height(&self) -> usize {
        // the initial width is likely the max texture side size
        self.image.width()
    }

    /// When this get high, it might be time to clear and start over!
    pub fn fill_ratio(&self) -> f32 {
        if self.overflowed {
            1.0
        } else {
            (self.cursor.1 + self.row_height) as f32 / self.max_height() as f32
        }
    }

    /// Call to get the change to the image since last call.
    pub fn take_delta(&mut self) -> Option<ImageDelta> {
        let dirty = std::mem::replace(&mut self.dirty, Rectu::NOTHING);
        if dirty == Rectu::NOTHING {
            None
        } else if dirty == Rectu::EVERYTHING {
            Some(ImageDelta::full(self.image.clone()))
        } else {
            let pos = [dirty.min_x, dirty.min_y];
            let size = [dirty.max_x - dirty.min_x, dirty.max_y - dirty.min_y];
            let region = self.image.region(pos, size);
            Some(ImageDelta::partial(pos, region))
        }
    }

    /// Returns the coordinates of where the rect ended up,
    /// and invalidates the region.
    pub fn allocate(&mut self, (w, h): (usize, usize)) -> ((usize, usize), &mut FontImage) {
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

        let required_height = self.cursor.1 + self.row_height;

        if required_height > self.max_height() {
            // This is a bad place to be - we need to start reusing space :/

            #[cfg(feature = "tracing")]
            tracing::wan!("epaint texture atlas overflowed!");

            self.cursor = (0, self.image.height() / 3); // Restart a bit down - the top of the atlas has too many important things in it
            self.overflowed = true; // this will signal the user that we need to recreate the texture atlas next frame.
        } else if resize_to_min_height(&mut self.image, required_height) {
            self.dirty = Rectu::EVERYTHING;
        }

        let pos = self.cursor;
        self.cursor.0 += w + PADDING;

        self.dirty.min_x = self.dirty.min_x.min(pos.0);
        self.dirty.min_y = self.dirty.min_y.min(pos.1);
        self.dirty.max_x = self.dirty.max_x.max(pos.0 + w);
        self.dirty.max_y = self.dirty.max_y.max(pos.1 + h);

        (pos, &mut self.image)
    }
}

fn resize_to_min_height(image: &mut FontImage, required_height: usize) -> bool {
    while required_height >= image.height() {
        image.size[1] *= 2; // double the height
    }

    if image.width() * image.height() > image.pixels.len() {
        image.pixels.resize(image.width() * image.height(), 0.0);
        true
    } else {
        false
    }
}
