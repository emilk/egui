#![allow(deprecated)]

use egui::{load::SizedTexture, mutex::Mutex, ColorImage, TextureOptions, Vec2};

#[cfg(feature = "svg")]
use egui::SizeHint;

/// An image to be shown in egui.
///
/// Load once, and save somewhere in your app state.
///
/// Use the `svg` and `image` features to enable more constructors.
///
/// ⚠ This type is deprecated: Consider using [`egui::Image`] instead.
#[deprecated = "consider using `egui::Image` instead"]
pub struct RetainedImage {
    debug_name: String,

    /// Texel size.
    ///
    /// Same as [`Self.image`]`.size`
    texel_size: [usize; 2],

    /// Original SVG size (if this is an SVG), or same as [`Self::texel_size`].
    source_size: Vec2,

    /// Cleared once [`Self::texture`] has been loaded.
    image: Mutex<ColorImage>,

    /// Lazily loaded when we have an egui context.
    texture: Mutex<Option<egui::TextureHandle>>,

    options: TextureOptions,
}

impl RetainedImage {
    pub fn from_color_image(debug_name: impl Into<String>, image: ColorImage) -> Self {
        Self {
            debug_name: debug_name.into(),
            texel_size: image.size,
            source_size: image.source_size,
            image: Mutex::new(image),
            texture: Default::default(),
            options: Default::default(),
        }
    }

    /// Load a (non-svg) image.
    ///
    /// `image_bytes` should be the raw contents of an image file (`.png`, `.jpg`, …).
    ///
    /// Requires the "image" feature. You must also opt-in to the image formats you need
    /// with e.g. `image = { version = "0.25", features = ["jpeg", "png"] }`.
    ///
    /// # Errors
    /// On invalid image or unsupported image format.
    #[cfg(feature = "image")]
    pub fn from_image_bytes(
        debug_name: impl Into<String>,
        image_bytes: &[u8],
    ) -> Result<Self, String> {
        Ok(Self::from_color_image(
            debug_name,
            load_image_bytes(image_bytes).map_err(|err| err.to_string())?,
        ))
    }

    /// Pass in the bytes of an SVG that you've loaded.
    ///
    /// # Errors
    /// On invalid image
    #[cfg(feature = "svg")]
    pub fn from_svg_bytes(
        debug_name: impl Into<String>,
        svg_bytes: &[u8],
        options: &resvg::usvg::Options<'_>,
    ) -> Result<Self, String> {
        Self::from_svg_bytes_with_size(debug_name, svg_bytes, Default::default(), options)
    }

    /// Pass in the str of an SVG that you've loaded.
    ///
    /// # Errors
    /// On invalid image
    #[cfg(feature = "svg")]
    pub fn from_svg_str(
        debug_name: impl Into<String>,
        svg_str: &str,
        options: &resvg::usvg::Options<'_>,
    ) -> Result<Self, String> {
        Self::from_svg_bytes_with_size(debug_name, svg_str.as_bytes(), Default::default(), options)
    }

    /// Pass in the bytes of an SVG that you've loaded
    /// and the scaling option to resize the SVG with.
    ///
    /// # Errors
    /// On invalid image
    #[cfg(feature = "svg")]
    pub fn from_svg_bytes_with_size(
        debug_name: impl Into<String>,
        svg_bytes: &[u8],
        size_hint: SizeHint,
        options: &resvg::usvg::Options<'_>,
    ) -> Result<Self, String> {
        Ok(Self::from_color_image(
            debug_name,
            load_svg_bytes_with_size(svg_bytes, size_hint, options)?,
        ))
    }

    /// Set the texture filters to use for the image.
    ///
    /// **Note:** If the texture has already been uploaded to the GPU, this will require
    /// re-uploading the texture with the updated filter.
    ///
    /// # Example
    /// ```rust
    /// # use egui_extras::RetainedImage;
    /// # use egui::{Color32, epaint::{ColorImage, textures::TextureOptions}};
    /// # let pixels = vec![Color32::BLACK];
    /// # let color_image = ColorImage {
    /// #   size: [1, 1],
    /// #   pixels,
    /// # };
    /// #
    /// // Upload a pixel art image without it getting blurry when resized
    /// let image = RetainedImage::from_color_image("my_image", color_image)
    ///     .with_options(TextureOptions::NEAREST);
    /// ```
    #[inline]
    pub fn with_options(mut self, options: TextureOptions) -> Self {
        self.options = options;

        // If the texture has already been uploaded, this will force it to be re-uploaded with the
        // updated filter.
        *self.texture.lock() = None;

        self
    }

    /// The size of the image data (number of pixels wide/high).
    pub fn texel_size(&self) -> [usize; 2] {
        self.texel_size
    }

    /// The size of the original SVG image (if any).
    ///
    /// Note that this can differ from [`Self::texel_size`] if the SVG was rasterized at a different
    /// resolution than the size of the original SVG.
    pub fn source_size(&self) -> Vec2 {
        self.source_size
    }

    /// The debug name of the image, e.g. the file name.
    pub fn debug_name(&self) -> &str {
        &self.debug_name
    }

    /// The texture id for this image.
    pub fn texture_id(&self, ctx: &egui::Context) -> egui::TextureId {
        self.texture
            .lock()
            .get_or_insert_with(|| {
                let image: &mut ColorImage = &mut self.image.lock();
                let image = std::mem::take(image);
                ctx.load_texture(&self.debug_name, image, self.options)
            })
            .id()
    }

    /// Show the image with the given maximum size.
    pub fn show_max_size(&self, ui: &mut egui::Ui, max_size: egui::Vec2) -> egui::Response {
        let mut desired_size = self.source_size();
        desired_size *= (max_size.x / desired_size.x).min(1.0);
        desired_size *= (max_size.y / desired_size.y).min(1.0);
        self.show_size(ui, desired_size)
    }

    /// Show the image with the original size (one image pixel = one gui point).
    pub fn show(&self, ui: &mut egui::Ui) -> egui::Response {
        self.show_size(ui, self.source_size())
    }

    /// Show the image with the given scale factor (1.0 = original size).
    pub fn show_scaled(&self, ui: &mut egui::Ui, scale: f32) -> egui::Response {
        self.show_size(ui, self.source_size() * scale)
    }

    /// Show the image with the given size.
    pub fn show_size(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) -> egui::Response {
        // We need to convert the SVG to a texture to display it:
        // Future improvement: tell backend to do mip-mapping of the image to
        // make it look smoother when downsized.
        ui.image(SizedTexture::new(self.texture_id(ui.ctx()), desired_size))
    }
}

// ----------------------------------------------------------------------------

/// Load a (non-svg) image.
///
/// Requires the "image" feature. You must also opt-in to the image formats you need
/// with e.g. `image = { version = "0.25", features = ["jpeg", "png"] }`.
///
/// # Errors
/// On invalid image or unsupported image format.
#[cfg(feature = "image")]
pub fn load_image_bytes(image_bytes: &[u8]) -> Result<ColorImage, egui::load::LoadError> {
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

    Ok(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
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
) -> Result<ColorImage, String> {
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
) -> Result<ColorImage, String> {
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

    let image = ColorImage::from_rgba_premultiplied([w as _, h as _], pixmap.data())
        .with_source_size(source_size);

    Ok(image)
}
