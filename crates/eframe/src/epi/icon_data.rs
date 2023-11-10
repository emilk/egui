/// Image data for an application icon.
///
/// Use a square image, e.g. 256x256 pixels.
/// You can use a transparent background.
#[derive(Clone)]
pub struct IconData {
    /// RGBA pixels, with separate/unmultiplied alpha.
    pub rgba: Vec<u8>,

    /// Image width. This should be a multiple of 4.
    pub width: u32,

    /// Image height. This should be a multiple of 4.
    pub height: u32,
}

impl std::fmt::Debug for IconData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IconData")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl IconData {
    /// Convert into [`image::RgbaImage`]
    ///
    /// # Errors
    /// If this is not a valid png.
    pub fn try_from_png_bytes(png_bytes: &[u8]) -> Result<Self, image::ImageError> {
        crate::profile_function!();
        let image = image::load_from_memory(png_bytes)?;
        Ok(Self::from_image(image))
    }

    fn from_image(image: image::DynamicImage) -> Self {
        let image = image.into_rgba8();
        Self {
            width: image.width(),
            height: image.height(),
            rgba: image.into_raw(),
        }
    }

    /// Convert into [`image::RgbaImage`]
    ///
    /// # Errors
    /// If `width*height != 4 * rgba.len()`, or if the image is too big.
    pub fn to_image(&self) -> Result<image::RgbaImage, String> {
        crate::profile_function!();
        let Self {
            rgba,
            width,
            height,
        } = self.clone();
        image::RgbaImage::from_raw(width, height, rgba).ok_or_else(|| "Invalid IconData".to_owned())
    }

    /// Encode as PNG.
    ///
    /// # Errors
    /// The image is invalid, or the PNG encoder failed.
    pub fn to_png_bytes(&self) -> Result<Vec<u8>, String> {
        crate::profile_function!();
        let image = self.to_image()?;
        let mut png_bytes: Vec<u8> = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut png_bytes),
                image::ImageOutputFormat::Png,
            )
            .map_err(|err| err.to_string())?;
        Ok(png_bytes)
    }
}
