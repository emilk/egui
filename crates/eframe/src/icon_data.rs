//! Helpers for loading [`egui::IconData`].

use egui::IconData;

/// Helpers for working with [`IconData`].
pub trait IconDataExt {
    /// Convert into [`image::RgbaImage`]
    ///
    /// # Errors
    /// If `width*height != 4 * rgba.len()`, or if the image is too big.
    fn to_image(&self) -> Result<image::RgbaImage, String>;

    /// Encode as PNG.
    ///
    /// # Errors
    /// The image is invalid, or the PNG encoder failed.
    fn to_png_bytes(&self) -> Result<Vec<u8>, String>;
}

/// Load the contents of .png file.
///
/// # Errors
/// If this is not a valid png.
pub fn from_png_bytes(png_bytes: &[u8]) -> Result<IconData, image::ImageError> {
    crate::profile_function!();
    let image = image::load_from_memory(png_bytes)?;
    Ok(from_image(image))
}

fn from_image(image: image::DynamicImage) -> IconData {
    let image = image.into_rgba8();
    IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

impl IconDataExt for IconData {
    fn to_image(&self) -> Result<image::RgbaImage, String> {
        crate::profile_function!();
        let Self {
            rgba,
            width,
            height,
        } = self.clone();
        image::RgbaImage::from_raw(width, height, rgba).ok_or_else(|| "Invalid IconData".to_owned())
    }

    fn to_png_bytes(&self) -> Result<Vec<u8>, String> {
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
