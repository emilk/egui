//! PNG encoding for screenshots — constructors for [`EncodedPng`].

use crate::protocol::EncodedPng;

impl EncodedPng {
    /// Encode an [`egui::ColorImage`] (e.g. from [`egui::Event::Screenshot`]) as PNG.
    ///
    /// # Errors
    /// When the encoder fails.
    pub fn from_color_image(image: &egui::ColorImage) -> Result<Self, image::ImageError> {
        let size = [image.size[0] as u32, image.size[1] as u32];
        let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
        Self::from_rgba(size, &rgba)
    }

    /// Encode tightly-packed RGBA8 pixels (`width * height * 4` bytes) as PNG.
    ///
    /// PNG keeps high-resolution captures off the hot path of socket throughput — a 1550×2114
    /// RGBA8 buffer is ~13 MiB raw but typically <1 MiB encoded.
    ///
    /// # Errors
    /// When the encoder fails (e.g. the buffer length doesn't match `width * height * 4`).
    pub fn from_rgba(size: [u32; 2], rgba: &[u8]) -> Result<Self, image::ImageError> {
        use image::ImageEncoder as _;
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes).write_image(
            rgba,
            size[0],
            size[1],
            image::ExtendedColorType::Rgba8,
        )?;
        Ok(Self { size, bytes })
    }
}
