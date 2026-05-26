//! Shared screenshot PNG encoder.
//!
//! Both peers — the live [`crate::InspectionPlugin`] and `egui_kittest`'s harness inspector
//! — encode their frames here so they produce identically-encoded
//! [`crate::protocol::FrameScreenshot`]s.

/// Encode tightly-packed RGBA8 pixels (`width * height * 4` bytes) as PNG using `image`'s
/// default settings (`CompressionType::Default` + `FilterType::Adaptive`).
///
/// PNG keeps high-resolution captures off the hot path of socket throughput — a 1550×2114
/// RGBA8 buffer is ~13 MiB raw but typically <1 MiB encoded.
///
/// # Errors
/// When the encoder fails (e.g. the buffer length doesn't match `width * height * 4`).
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, image::ImageError> {
    use image::ImageEncoder as _;
    let mut out = std::io::Cursor::new(Vec::new());
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)?;
    Ok(out.into_inner())
}
