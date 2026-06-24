//! PNG encoding for screenshots — constructors for [`EncodedPng`].

use crate::protocol::EncodedPng;

impl EncodedPng {
    /// Encode an [`egui::ColorImage`] (e.g. from [`egui::Event::Screenshot`]) as PNG.
    ///
    /// # Errors
    /// When the encoder fails.
    pub fn from_color_image(image: &egui::ColorImage) -> Result<Self, image::ImageError> {
        let size = [image.size[0] as u32, image.size[1] as u32];
        Self::from_rgba(size, image.as_raw())
    }

    /// Encode an [`egui::ColorImage`] downscaled by `scale` — a factor in `(0.0, 1.0]` of the
    /// captured pixel dimensions. `scale >= 1.0` encodes at native resolution unchanged: the
    /// framebuffer is the most detail available, so we never upscale.
    ///
    /// # Errors
    /// When the encoder fails.
    pub fn from_color_image_scaled(
        image: &egui::ColorImage,
        scale: f32,
    ) -> Result<Self, image::ImageError> {
        let [w, h] = [image.size[0] as u32, image.size[1] as u32];
        if scale >= 1.0 || w == 0 || h == 0 {
            return Self::from_rgba([w, h], image.as_raw());
        }
        let tw = ((w as f32 * scale).round() as u32).max(1);
        let th = ((h as f32 * scale).round() as u32).max(1);
        let src = image::RgbaImage::from_raw(w, h, image.as_raw().to_vec())
            .expect("ColorImage backing buffer is always width * height * 4 bytes");
        let resized = image::imageops::resize(&src, tw, th, image::imageops::FilterType::Triangle);
        Self::from_rgba([tw, th], resized.as_raw())
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
