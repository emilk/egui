use egui::{mutex::Mutex, TextureFilter, TextureOptions};

#[cfg(feature = "svg")]
pub use usvg::FitTo;

/// An image to be shown in egui.
///
/// Load once, and save somewhere in your app state.
///
/// Use the `svg` and `image` features to enable more constructors.
pub struct RetainedImage {
    debug_name: String,
    size: [usize; 2],
    /// Cleared once [`Self::texture`] has been loaded.
    image: Mutex<egui::ColorImage>,
    /// Lazily loaded when we have an egui context.
    texture: Mutex<Option<egui::TextureHandle>>,
    options: TextureOptions,
}

impl RetainedImage {
    pub fn from_color_image(debug_name: impl Into<String>, image: ColorImage) -> Self {
        Self {
            debug_name: debug_name.into(),
            size: image.size,
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
    /// with e.g. `image = { version = "0.24", features = ["jpeg", "png"] }`.
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
            load_image_bytes(image_bytes)?,
        ))
    }

    /// Pass in the bytes of an SVG that you've loaded.
    ///
    /// # Errors
    /// On invalid image
    #[cfg(feature = "svg")]
    pub fn from_svg_bytes(debug_name: impl Into<String>, svg_bytes: &[u8]) -> Result<Self, String> {
        Self::from_svg_bytes_with_size(debug_name, svg_bytes, FitTo::Original)
    }

    /// Pass in the str of an SVG that you've loaded.
    ///
    /// # Errors
    /// On invalid image
    #[cfg(feature = "svg")]
    pub fn from_svg_str(debug_name: impl Into<String>, svg_str: &str) -> Result<Self, String> {
        Self::from_svg_bytes(debug_name, svg_str.as_bytes())
    }

    /// Pass in the bytes of an SVG that you've loaded
    /// and the scaling option to resize the SVG with
    ///
    /// # Errors
    /// On invalid image
    #[cfg(feature = "svg")]
    pub fn from_svg_bytes_with_size(
        debug_name: impl Into<String>,
        svg_bytes: &[u8],
        size: FitTo,
    ) -> Result<Self, String> {
        Ok(Self::from_color_image(
            debug_name,
            load_svg_bytes_with_size(svg_bytes, size)?,
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
    pub fn with_options(mut self, options: TextureOptions) -> Self {
        self.options = options;

        // If the texture has already been uploaded, this will force it to be re-uploaded with the
        // updated filter.
        *self.texture.lock() = None;

        self
    }

    #[deprecated = "Use with_options instead"]
    pub fn with_texture_filter(self, filter: TextureFilter) -> Self {
        self.with_options(TextureOptions {
            magnification: filter,
            minification: filter,
        })
    }

    /// The size of the image data (number of pixels wide/high).
    pub fn size(&self) -> [usize; 2] {
        self.size
    }

    /// The width of the image.
    pub fn width(&self) -> usize {
        self.size[0]
    }

    /// The height of the image.
    pub fn height(&self) -> usize {
        self.size[1]
    }

    /// The size of the image data (number of pixels wide/high).
    pub fn size_vec2(&self) -> egui::Vec2 {
        let [w, h] = self.size();
        egui::vec2(w as f32, h as f32)
    }

    /// The debug name of the image, e.g. the file name.
    pub fn debug_name(&self) -> &str {
        &self.debug_name
    }

    /// The texture if for this image.
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
        let mut desired_size = self.size_vec2();
        desired_size *= (max_size.x / desired_size.x).min(1.0);
        desired_size *= (max_size.y / desired_size.y).min(1.0);
        self.show_size(ui, desired_size)
    }

    /// Show the image with the original size (one image pixel = one gui point).
    pub fn show(&self, ui: &mut egui::Ui) -> egui::Response {
        self.show_size(ui, self.size_vec2())
    }

    /// Show the image with the given scale factor (1.0 = original size).
    pub fn show_scaled(&self, ui: &mut egui::Ui, scale: f32) -> egui::Response {
        self.show_size(ui, self.size_vec2() * scale)
    }

    /// Show the image with the given size.
    pub fn show_size(&self, ui: &mut egui::Ui, desired_size: egui::Vec2) -> egui::Response {
        // We need to convert the SVG to a texture to display it:
        // Future improvement: tell backend to do mip-mapping of the image to
        // make it look smoother when downsized.
        ui.image(self.texture_id(ui.ctx()), desired_size)
    }
}

// ----------------------------------------------------------------------------

use egui::ColorImage;

/// Load a (non-svg) image.
///
/// Requires the "image" feature. You must also opt-in to the image formats you need
/// with e.g. `image = { version = "0.24", features = ["jpeg", "png"] }`.
///
/// # Errors
/// On invalid image or unsupported image format.
#[cfg(feature = "image")]
pub fn load_image_bytes(image_bytes: &[u8]) -> Result<egui::ColorImage, String> {
    let image = image::load_from_memory(image_bytes).map_err(|err| err.to_string())?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
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
pub fn load_svg_bytes(svg_bytes: &[u8]) -> Result<egui::ColorImage, String> {
    load_svg_bytes_with_size(svg_bytes, FitTo::Original)
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
    fit_to: FitTo,
) -> Result<egui::ColorImage, String> {
    let mut opt = usvg::Options::default();
    opt.fontdb.load_system_fonts();

    let rtree = usvg::Tree::from_data(svg_bytes, &opt.to_ref()).map_err(|err| err.to_string())?;

    let pixmap_size = rtree.svg_node().size.to_screen_size();
    let [w, h] = match fit_to {
        FitTo::Original => [pixmap_size.width(), pixmap_size.height()],
        FitTo::Size(w, h) => [w, h],
        FitTo::Height(h) => [
            (pixmap_size.width() as f32 * (h as f32 / pixmap_size.height() as f32)) as u32,
            h,
        ],
        FitTo::Width(w) => [
            w,
            (pixmap_size.height() as f32 * (w as f32 / pixmap_size.width() as f32)) as u32,
        ],
        FitTo::Zoom(z) => [
            (pixmap_size.width() as f32 * z) as u32,
            (pixmap_size.height() as f32 * z) as u32,
        ],
    };

    let mut pixmap = tiny_skia::Pixmap::new(w, h)
        .ok_or_else(|| format!("Failed to create SVG Pixmap of size {}x{}", w, h))?;

    resvg::render(&rtree, fit_to, Default::default(), pixmap.as_mut())
        .ok_or_else(|| "Failed to render SVG".to_owned())?;

    let image = egui::ColorImage::from_rgba_unmultiplied([w as _, h as _], pixmap.data());

    Ok(image)
}
