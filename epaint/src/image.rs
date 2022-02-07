use crate::Color32;

/// An image stored in RAM.
///
/// To load an image file, see [`ColorImage::from_rgba_unmultiplied`].
///
/// In order to paint the image on screen, you first need to convert it to
///
/// See also: [`ColorImage`], [`AlphaImage`].
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ImageData {
    /// RGBA image.
    Color(ColorImage),
    /// Used for the font texture.
    Alpha(AlphaImage),
}

impl ImageData {
    pub fn size(&self) -> [usize; 2] {
        match self {
            Self::Color(image) => image.size,
            Self::Alpha(image) => image.size,
        }
    }

    pub fn width(&self) -> usize {
        self.size()[0]
    }

    pub fn height(&self) -> usize {
        self.size()[1]
    }

    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Color(_) => 4,
            Self::Alpha(_) => 1,
        }
    }
}

// ----------------------------------------------------------------------------

/// A 2D RGBA color image in RAM.
#[derive(Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ColorImage {
    /// width, height.
    pub size: [usize; 2],
    /// The pixels, row by row, from top to bottom.
    pub pixels: Vec<Color32>,
}

impl ColorImage {
    /// Create an image filled with the given color.
    pub fn new(size: [usize; 2], color: Color32) -> Self {
        Self {
            size,
            pixels: vec![color; size[0] * size[1]],
        }
    }

    /// Create an `Image` from flat un-multiplied RGBA data.
    ///
    /// This is usually what you want to use after having loaded an image file.
    ///
    /// Panics if `size[0] * size[1] * 4 != rgba.len()`.
    ///
    /// ## Example using the [`image`](crates.io/crates/image) crate:
    /// ``` ignore
    /// fn load_image_from_path(path: &std::path::Path) -> Result<egui::ColorImage, image::ImageError> {
    ///     let image = image::io::Reader::open(path)?.decode()?;
    ///     let size = [image.width() as _, image.height() as _];
    ///     let image_buffer = image.to_rgba8();
    ///     let pixels = image_buffer.as_flat_samples();
    ///     Ok(egui::ColorImage::from_rgba_unmultiplied(
    ///         size,
    ///         pixels.as_slice(),
    ///     ))
    /// }
    ///
    /// fn load_image_from_memory(image_data: &[u8]) -> Result<ColorImage, image::ImageError> {
    ///     let image = image::load_from_memory(image_data)?;
    ///     let size = [image.width() as _, image.height() as _];
    ///     let image_buffer = image.to_rgba8();
    ///     let pixels = image_buffer.as_flat_samples();
    ///     Ok(ColorImage::from_rgba_unmultiplied(
    ///         size,
    ///         pixels.as_slice(),
    ///     ))
    /// }
    /// ```
    pub fn from_rgba_unmultiplied(size: [usize; 2], rgba: &[u8]) -> Self {
        assert_eq!(size[0] * size[1] * 4, rgba.len());
        let pixels = rgba
            .chunks_exact(4)
            .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();
        Self { size, pixels }
    }

    /// An example color image, useful for tests.
    pub fn example() -> Self {
        let width = 128;
        let height = 64;
        let mut img = Self::new([width, height], Color32::TRANSPARENT);
        for y in 0..height {
            for x in 0..width {
                let h = x as f32 / width as f32;
                let s = 1.0;
                let v = 1.0;
                let a = y as f32 / height as f32;
                img[(x, y)] = crate::color::Hsva { h, s, v, a }.into();
            }
        }
        img
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.size[0]
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.size[1]
    }
}

impl std::ops::Index<(usize, usize)> for ColorImage {
    type Output = Color32;

    #[inline]
    fn index(&self, (x, y): (usize, usize)) -> &Color32 {
        let [w, h] = self.size;
        assert!(x < w && y < h);
        &self.pixels[y * w + x]
    }
}

impl std::ops::IndexMut<(usize, usize)> for ColorImage {
    #[inline]
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Color32 {
        let [w, h] = self.size;
        assert!(x < w && y < h);
        &mut self.pixels[y * w + x]
    }
}

impl From<ColorImage> for ImageData {
    #[inline(always)]
    fn from(image: ColorImage) -> Self {
        Self::Color(image)
    }
}

// ----------------------------------------------------------------------------

/// An 8-bit image, representing difference levels of transparent white.
///
/// Used for the font texture
#[derive(Clone, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AlphaImage {
    /// width, height
    pub size: [usize; 2],
    /// The alpha (linear space 0-255) of something white.
    ///
    /// One byte per pixel. Often you want to use [`Self::srgba_pixels`] instead.
    pub pixels: Vec<u8>,
}

impl AlphaImage {
    pub fn new(size: [usize; 2]) -> Self {
        Self {
            size,
            pixels: vec![0; size[0] * size[1]],
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.size[0]
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.size[1]
    }

    /// Returns the textures as `sRGBA` premultiplied pixels, row by row, top to bottom.
    ///
    /// `gamma` should normally be set to 1.0.
    /// If you are having problems with text looking skinny and pixelated, try
    /// setting a lower gamma, e.g. `0.5`.
    pub fn srgba_pixels(
        &'_ self,
        gamma: f32,
    ) -> impl ExactSizeIterator<Item = super::Color32> + '_ {
        let srgba_from_alpha_lut: Vec<Color32> = (0..=255)
            .map(|a| {
                let a = super::color::linear_f32_from_linear_u8(a).powf(gamma);
                super::Rgba::from_white_alpha(a).into()
            })
            .collect();

        self.pixels
            .iter()
            .map(move |&a| srgba_from_alpha_lut[a as usize])
    }

    /// Clone a sub-region as a new image
    pub fn region(&self, [x, y]: [usize; 2], [w, h]: [usize; 2]) -> AlphaImage {
        assert!(x + w <= self.width());
        assert!(y + h <= self.height());

        let mut pixels = Vec::with_capacity(w * h);
        for y in y..y + h {
            let offset = y * self.width() + x;
            pixels.extend(&self.pixels[offset..(offset + w)]);
        }
        assert_eq!(pixels.len(), w * h);
        AlphaImage {
            size: [w, h],
            pixels,
        }
    }
}

impl std::ops::Index<(usize, usize)> for AlphaImage {
    type Output = u8;

    #[inline]
    fn index(&self, (x, y): (usize, usize)) -> &u8 {
        let [w, h] = self.size;
        assert!(x < w && y < h);
        &self.pixels[y * w + x]
    }
}

impl std::ops::IndexMut<(usize, usize)> for AlphaImage {
    #[inline]
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut u8 {
        let [w, h] = self.size;
        assert!(x < w && y < h);
        &mut self.pixels[y * w + x]
    }
}

impl From<AlphaImage> for ImageData {
    #[inline(always)]
    fn from(image: AlphaImage) -> Self {
        Self::Alpha(image)
    }
}

// ----------------------------------------------------------------------------

/// A change to an image.
///
/// Either a whole new image,
/// or an update to a rectangular region of it.
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[must_use = "The painter must take care of this"]
pub struct ImageDelta {
    /// What to set the texture to.
    pub image: ImageData,

    /// If `None`, set the whole texture to [`Self::image`].
    /// If `Some(pos)`, update a sub-region of an already allocated texture.
    pub pos: Option<[usize; 2]>,
}

impl ImageDelta {
    /// Update the whole texture.
    pub fn full(image: impl Into<ImageData>) -> Self {
        Self {
            image: image.into(),
            pos: None,
        }
    }

    /// Update a sub-region of an existing texture.
    pub fn partial(pos: [usize; 2], image: impl Into<ImageData>) -> Self {
        Self {
            image: image.into(),
            pos: Some(pos),
        }
    }

    /// Is this affecting the whole texture?
    /// If `false`, this is a partial (sub-region) update.
    pub fn is_whole(&self) -> bool {
        self.pos.is_none()
    }
}
