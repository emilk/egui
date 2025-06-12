#[cfg(feature = "svg")]
use egui::SizeHint;

// ----------------------------------------------------------------------------

/// Load a (non-svg) image.
///
/// Requires the "image" feature. You must also opt-in to the image formats you need
/// with e.g. `image = { version = "0.25", features = ["jpeg", "png"] }`.
///
/// # Errors
/// On invalid image or unsupported image format.
#[cfg(feature = "image")]
pub fn load_image_bytes(image_bytes: &[u8]) -> Result<egui::ColorImage, egui::load::LoadError> {
    profiling::function_scope!();
    let image = image::load_from_memory(image_bytes).map_err(|err| match err {
        image::ImageError::Unsupported(err) => match err.kind() {
            image::error::UnsupportedErrorKind::Format(format) => {
                egui::load::LoadError::FormatNotSupported {
                    detected_format: Some(format.to_string()),
                }
            }
            _ => egui::load::LoadError::Loading(err.to_string()),
        },
        err => egui::load::LoadError::Loading(err.to_string()),
    })?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();

    // TODO(emilk): if this is a PNG, looks for DPI info to calculate the source size,
    // e.g. for screenshots taken on a high-DPI/retina display.

    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}

/// Load an SVG and rasterize it into an egui image.
///
/// Requires the "svg" feature.
///
/// # Errors
/// On invalid image
#[cfg(feature = "svg")]
pub fn load_svg_bytes(
    svg_bytes: &[u8],
    options: &resvg::usvg::Options<'_>,
) -> Result<egui::ColorImage, String> {
    load_svg_bytes_with_size(svg_bytes, Default::default(), options)
}

/// Load an SVG and rasterize it into an egui image with a scaling parameter.
///
/// Requires the "svg" feature.
///
/// # Errors
/// On invalid image
#[cfg(feature = "svg")]
pub fn load_svg_bytes_with_size(
    svg_bytes: &[u8],
    size_hint: SizeHint,
    options: &resvg::usvg::Options<'_>,
) -> Result<egui::ColorImage, String> {
    use egui::Vec2;
    use resvg::{
        tiny_skia::Pixmap,
        usvg::{Transform, Tree},
    };

    profiling::function_scope!();

    let rtree = Tree::from_data(svg_bytes, options).map_err(|err| err.to_string())?;

    let source_size = Vec2::new(rtree.size().width(), rtree.size().height());

    let scaled_size = match size_hint {
        SizeHint::Size {
            width,
            height,
            maintain_aspect_ratio,
        } => {
            if maintain_aspect_ratio {
                // As large as possible, without exceeding the given size:
                let mut size = source_size;
                size *= width as f32 / source_size.x;
                if size.y > height as f32 {
                    size *= height as f32 / size.y;
                }
                size
            } else {
                Vec2::new(width as _, height as _)
            }
        }
        SizeHint::Height(h) => source_size * (h as f32 / source_size.y),
        SizeHint::Width(w) => source_size * (w as f32 / source_size.x),
        SizeHint::Scale(scale) => scale.into_inner() * source_size,
    };

    let scaled_size = scaled_size.round();
    let (w, h) = (scaled_size.x as u32, scaled_size.y as u32);

    let mut pixmap =
        Pixmap::new(w, h).ok_or_else(|| format!("Failed to create SVG Pixmap of size {w}x{h}"))?;

    resvg::render(
        &rtree,
        Transform::from_scale(w as f32 / source_size.x, h as f32 / source_size.y),
        &mut pixmap.as_mut(),
    );

    let image = egui::ColorImage::from_rgba_premultiplied([w as _, h as _], pixmap.data())
        .with_source_size(source_size);

    Ok(image)
}
