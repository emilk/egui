use std::borrow::Cow;

use crate::load::TextureLoadResult;
use crate::{
    load::{Bytes, SizeHint, SizedTexture, TexturePoll},
    *,
};
use emath::Rot2;
use epaint::{util::FloatOrd, RectShape};

/// A widget which displays an image.
///
/// The task of actually loading the image is deferred to when the `Image` is added to the [`Ui`],
/// and how it is loaded depends on the provided [`ImageSource`]:
///
/// - [`ImageSource::Uri`] will load the image using the [asynchronous loading process][`load`].
/// - [`ImageSource::Bytes`] will also load the image using the [asynchronous loading process][`load`], but with lower latency.
/// - [`ImageSource::Texture`] will use the provided texture.
///
/// See [`load`] for more information.
///
/// ### Examples
/// // Using it in a layout:
/// ```
/// # egui::__run_test_ui(|ui| {
/// ui.add(
///     egui::Image::new(egui::include_image!("../../assets/ferris.png"))
///         .rounding(5.0)
/// );
/// # });
/// ```
///
/// // Using it just to paint:
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let rect = egui::Rect::from_min_size(Default::default(), egui::Vec2::splat(100.0));
/// egui::Image::new(egui::include_image!("../../assets/ferris.png"))
///     .rounding(5.0)
///     .tint(egui::Color32::LIGHT_BLUE)
///     .paint_at(ui, rect);
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Debug, Clone)]
pub struct Image<'a> {
    source: ImageSource<'a>,
    texture_options: TextureOptions,
    image_options: ImageOptions,
    sense: Sense,
    size: ImageSize,
    pub(crate) show_loading_spinner: Option<bool>,
}

impl<'a> Image<'a> {
    /// Load the image from some source.
    pub fn new(source: impl Into<ImageSource<'a>>) -> Self {
        fn new_mono(source: ImageSource<'_>) -> Image<'_> {
            let size = if let ImageSource::Texture(tex) = &source {
                // User is probably expecting their texture to have
                // the exact size of the provided `SizedTexture`.
                ImageSize {
                    maintain_aspect_ratio: true,
                    max_size: Vec2::INFINITY,
                    fit: ImageFit::Exact(tex.size),
                }
            } else {
                Default::default()
            };

            Image {
                source,
                texture_options: Default::default(),
                image_options: Default::default(),
                sense: Sense::hover(),
                size,
                show_loading_spinner: None,
            }
        }

        new_mono(source.into())
    }

    /// Load the image from a URI.
    ///
    /// See [`ImageSource::Uri`].
    pub fn from_uri(uri: impl Into<Cow<'a, str>>) -> Self {
        Self::new(ImageSource::Uri(uri.into()))
    }

    /// Load the image from an existing texture.
    ///
    /// See [`ImageSource::Texture`].
    pub fn from_texture(texture: impl Into<SizedTexture>) -> Self {
        Self::new(ImageSource::Texture(texture.into()))
    }

    /// Load the image from some raw bytes.
    ///
    /// For better error messages, use the `bytes://` prefix for the URI.
    ///
    /// See [`ImageSource::Bytes`].
    pub fn from_bytes(uri: impl Into<Cow<'static, str>>, bytes: impl Into<Bytes>) -> Self {
        Self::new(ImageSource::Bytes {
            uri: uri.into(),
            bytes: bytes.into(),
        })
    }

    /// Texture options used when creating the texture.
    #[inline]
    pub fn texture_options(mut self, texture_options: TextureOptions) -> Self {
        self.texture_options = texture_options;
        self
    }

    /// Set the max width of the image.
    ///
    /// No matter what the image is scaled to, it will never exceed this limit.
    #[inline]
    pub fn max_width(mut self, width: f32) -> Self {
        self.size.max_size.x = width;
        self
    }

    /// Set the max height of the image.
    ///
    /// No matter what the image is scaled to, it will never exceed this limit.
    #[inline]
    pub fn max_height(mut self, height: f32) -> Self {
        self.size.max_size.y = height;
        self
    }

    /// Set the max size of the image.
    ///
    /// No matter what the image is scaled to, it will never exceed this limit.
    #[inline]
    pub fn max_size(mut self, size: Vec2) -> Self {
        self.size.max_size = size;
        self
    }

    /// Whether or not the [`ImageFit`] should maintain the image's original aspect ratio.
    #[inline]
    pub fn maintain_aspect_ratio(mut self, value: bool) -> Self {
        self.size.maintain_aspect_ratio = value;
        self
    }

    /// Fit the image to its original size with some scaling.
    ///
    /// This will cause the image to overflow if it is larger than the available space.
    ///
    /// If [`Image::max_size`] is set, this is guaranteed to never exceed that limit.
    #[inline]
    pub fn fit_to_original_size(mut self, scale: f32) -> Self {
        self.size.fit = ImageFit::Original { scale };
        self
    }

    /// Fit the image to an exact size.
    ///
    /// If [`Image::max_size`] is set, this is guaranteed to never exceed that limit.
    #[inline]
    pub fn fit_to_exact_size(mut self, size: Vec2) -> Self {
        self.size.fit = ImageFit::Exact(size);
        self
    }

    /// Fit the image to a fraction of the available space.
    ///
    /// If [`Image::max_size`] is set, this is guaranteed to never exceed that limit.
    #[inline]
    pub fn fit_to_fraction(mut self, fraction: Vec2) -> Self {
        self.size.fit = ImageFit::Fraction(fraction);
        self
    }

    /// Fit the image to 100% of its available size, shrinking it if necessary.
    ///
    /// This is a shorthand for [`Image::fit_to_fraction`] with `1.0` for both width and height.
    ///
    /// If [`Image::max_size`] is set, this is guaranteed to never exceed that limit.
    #[inline]
    pub fn shrink_to_fit(self) -> Self {
        self.fit_to_fraction(Vec2::new(1.0, 1.0))
    }

    /// Make the image respond to clicks and/or drags.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Select UV range. Default is (0,0) in top-left, (1,1) bottom right.
    #[inline]
    pub fn uv(mut self, uv: impl Into<Rect>) -> Self {
        self.image_options.uv = uv.into();
        self
    }

    /// A solid color to put behind the image. Useful for transparent images.
    #[inline]
    pub fn bg_fill(mut self, bg_fill: impl Into<Color32>) -> Self {
        self.image_options.bg_fill = bg_fill.into();
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
    #[inline]
    pub fn tint(mut self, tint: impl Into<Color32>) -> Self {
        self.image_options.tint = tint.into();
        self
    }

    /// Rotate the image about an origin by some angle
    ///
    /// Positive angle is clockwise.
    /// Origin is a vector in normalized UV space ((0,0) in top-left, (1,1) bottom right).
    ///
    /// To rotate about the center you can pass `Vec2::splat(0.5)` as the origin.
    ///
    /// Due to limitations in the current implementation,
    /// this will turn off rounding of the image.
    #[inline]
    pub fn rotate(mut self, angle: f32, origin: Vec2) -> Self {
        self.image_options.rotation = Some((Rot2::from_angle(angle), origin));
        self.image_options.rounding = Rounding::ZERO; // incompatible with rotation
        self
    }

    /// Round the corners of the image.
    ///
    /// The default is no rounding ([`Rounding::ZERO`]).
    ///
    /// Due to limitations in the current implementation,
    /// this will turn off any rotation of the image.
    #[inline]
    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.image_options.rounding = rounding.into();
        if self.image_options.rounding != Rounding::ZERO {
            self.image_options.rotation = None; // incompatible with rounding
        }
        self
    }

    /// Show a spinner when the image is loading.
    ///
    /// By default this uses the value of [`Visuals::image_loading_spinners`].
    #[inline]
    pub fn show_loading_spinner(mut self, show: bool) -> Self {
        self.show_loading_spinner = Some(show);
        self
    }
}

impl<'a, T: Into<ImageSource<'a>>> From<T> for Image<'a> {
    fn from(value: T) -> Self {
        Image::new(value)
    }
}

impl<'a> Image<'a> {
    /// Returns the size the image will occupy in the final UI.
    #[inline]
    pub fn calc_size(&self, available_size: Vec2, original_image_size: Option<Vec2>) -> Vec2 {
        let original_image_size = original_image_size.unwrap_or(Vec2::splat(24.0)); // Fallback for still-loading textures, or failure to load.
        self.size.calc_size(available_size, original_image_size)
    }

    pub fn load_and_calc_size(&self, ui: &mut Ui, available_size: Vec2) -> Option<Vec2> {
        let image_size = self.load_for_size(ui.ctx(), available_size).ok()?.size()?;
        Some(self.size.calc_size(available_size, image_size))
    }

    #[inline]
    pub fn size(&self) -> Option<Vec2> {
        match &self.source {
            ImageSource::Texture(texture) => Some(texture.size),
            ImageSource::Uri(_) | ImageSource::Bytes { .. } => None,
        }
    }

    #[inline]
    pub fn image_options(&self) -> &ImageOptions {
        &self.image_options
    }

    #[inline]
    pub fn source(&self) -> &ImageSource<'a> {
        &self.source
    }

    /// Load the image from its [`Image::source`], returning the resulting [`SizedTexture`].
    ///
    /// The `available_size` is used as a hint when e.g. rendering an svg.
    ///
    /// # Errors
    /// May fail if they underlying [`Context::try_load_texture`] call fails.
    pub fn load_for_size(&self, ctx: &Context, available_size: Vec2) -> TextureLoadResult {
        let size_hint = self.size.hint(available_size);
        self.source
            .clone()
            .load(ctx, self.texture_options, size_hint)
    }

    /// Paint the image in the given rectangle.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let rect = egui::Rect::from_min_size(Default::default(), egui::Vec2::splat(100.0));
    /// egui::Image::new(egui::include_image!("../../assets/ferris.png"))
    ///     .rounding(5.0)
    ///     .tint(egui::Color32::LIGHT_BLUE)
    ///     .paint_at(ui, rect);
    /// # });
    /// ```
    #[inline]
    pub fn paint_at(&self, ui: &mut Ui, rect: Rect) {
        paint_texture_load_result(
            ui,
            &self.load_for_size(ui.ctx(), rect.size()),
            rect,
            self.show_loading_spinner,
            &self.image_options,
        );
    }
}

impl<'a> Widget for Image<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tlr = self.load_for_size(ui.ctx(), ui.available_size());
        let original_image_size = tlr.as_ref().ok().and_then(|t| t.size());
        let ui_size = self.calc_size(ui.available_size(), original_image_size);

        let (rect, response) = ui.allocate_exact_size(ui_size, self.sense);
        if ui.is_rect_visible(rect) {
            paint_texture_load_result(
                ui,
                &tlr,
                rect,
                self.show_loading_spinner,
                &self.image_options,
            );
        }
        texture_load_result_response(&self.source, &tlr, response)
    }
}

/// This type determines the constraints on how
/// the size of an image should be calculated.
#[derive(Debug, Clone, Copy)]
pub struct ImageSize {
    /// Whether or not the final size should maintain the original aspect ratio.
    ///
    /// This setting is applied last.
    ///
    /// This defaults to `true`.
    pub maintain_aspect_ratio: bool,

    /// Determines the maximum size of the image.
    ///
    /// Defaults to `Vec2::INFINITY` (no limit).
    pub max_size: Vec2,

    /// Determines how the image should shrink/expand/stretch/etc. to fit within its allocated space.
    ///
    /// This setting is applied first.
    ///
    /// Defaults to `ImageFit::Fraction([1, 1])`
    pub fit: ImageFit,
}

/// This type determines how the image should try to fit within the UI.
///
/// The final fit will be clamped to [`ImageSize::max_size`].
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ImageFit {
    /// Fit the image to its original size, scaled by some factor.
    ///
    /// Ignores how much space is actually available in the ui.
    Original { scale: f32 },

    /// Fit the image to a fraction of the available size.
    Fraction(Vec2),

    /// Fit the image to an exact size.
    ///
    /// Ignores how much space is actually available in the ui.
    Exact(Vec2),
}

impl ImageFit {
    pub fn resolve(self, available_size: Vec2, image_size: Vec2) -> Vec2 {
        match self {
            ImageFit::Original { scale } => image_size * scale,
            ImageFit::Fraction(fract) => available_size * fract,
            ImageFit::Exact(size) => size,
        }
    }
}

impl ImageSize {
    /// Size hint for e.g. rasterizing an svg.
    pub fn hint(&self, available_size: Vec2) -> SizeHint {
        let size = match self.fit {
            ImageFit::Original { scale } => return SizeHint::Scale(scale.ord()),
            ImageFit::Fraction(fract) => available_size * fract,
            ImageFit::Exact(size) => size,
        };

        let size = size.min(self.max_size);

        // TODO(emilk): take pixels_per_point into account here!

        // `inf` on an axis means "any value"
        match (size.x.is_finite(), size.y.is_finite()) {
            (true, true) => SizeHint::Size(size.x.round() as u32, size.y.round() as u32),
            (true, false) => SizeHint::Width(size.x.round() as u32),
            (false, true) => SizeHint::Height(size.y.round() as u32),
            (false, false) => SizeHint::Scale(1.0.ord()),
        }
    }

    /// Calculate the final on-screen size in points.
    pub fn calc_size(&self, available_size: Vec2, original_image_size: Vec2) -> Vec2 {
        let Self {
            maintain_aspect_ratio,
            max_size,
            fit,
        } = *self;
        match fit {
            ImageFit::Original { scale } => {
                let image_size = original_image_size * scale;
                if image_size.x <= max_size.x && image_size.y <= max_size.y {
                    image_size
                } else {
                    scale_to_fit(image_size, max_size, maintain_aspect_ratio)
                }
            }
            ImageFit::Fraction(fract) => {
                let scale_to_size = (available_size * fract).min(max_size);
                scale_to_fit(original_image_size, scale_to_size, maintain_aspect_ratio)
            }
            ImageFit::Exact(size) => {
                let scale_to_size = size.min(max_size);
                scale_to_fit(original_image_size, scale_to_size, maintain_aspect_ratio)
            }
        }
    }
}

// TODO: unit-tests
fn scale_to_fit(image_size: Vec2, available_size: Vec2, maintain_aspect_ratio: bool) -> Vec2 {
    if maintain_aspect_ratio {
        let ratio_x = available_size.x / image_size.x;
        let ratio_y = available_size.y / image_size.y;
        let ratio = if ratio_x < ratio_y { ratio_x } else { ratio_y };
        let ratio = if ratio.is_finite() { ratio } else { 1.0 };
        image_size * ratio
    } else {
        available_size
    }
}

impl Default for ImageSize {
    #[inline]
    fn default() -> Self {
        Self {
            max_size: Vec2::INFINITY,
            fit: ImageFit::Fraction(Vec2::new(1.0, 1.0)),
            maintain_aspect_ratio: true,
        }
    }
}

/// This type tells the [`Ui`] how to load an image.
///
/// This is used by [`Image::new`] and [`Ui::image`].
#[derive(Clone)]
pub enum ImageSource<'a> {
    /// Load the image from a URI, e.g. `https://example.com/image.png`.
    ///
    /// This could be a `file://` path, `https://` url, `bytes://` identifier, or some other scheme.
    ///
    /// How the URI will be turned into a texture for rendering purposes is
    /// up to the registered loaders to handle.
    ///
    /// See [`crate::load`] for more information.
    Uri(Cow<'a, str>),

    /// Load the image from an existing texture.
    ///
    /// The user is responsible for loading the texture, determining its size,
    /// and allocating a [`TextureId`] for it.
    Texture(SizedTexture),

    /// Load the image from some raw bytes.
    ///
    /// The [`Bytes`] may be:
    /// - `'static`, obtained from `include_bytes!` or similar
    /// - Anything that can be converted to `Arc<[u8]>`
    ///
    /// This instructs the [`Ui`] to cache the raw bytes, which are then further processed by any registered loaders.
    ///
    /// See also [`include_image`] for an easy way to load and display static images.
    ///
    /// See [`crate::load`] for more information.
    Bytes {
        /// The unique identifier for this image, e.g. `bytes://my_logo.png`.
        ///
        /// You should use a proper extension (`.jpg`, `.png`, `.svg`, etc) for the image to load properly.
        ///
        /// Use the `bytes://` scheme for the URI for better error messages.
        uri: Cow<'static, str>,

        bytes: Bytes,
    },
}

impl<'a> std::fmt::Debug for ImageSource<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageSource::Bytes { uri, .. } | ImageSource::Uri(uri) => uri.as_ref().fmt(f),
            ImageSource::Texture(st) => st.id.fmt(f),
        }
    }
}

impl<'a> ImageSource<'a> {
    /// Size of the texture, if known.
    #[inline]
    pub fn texture_size(&self) -> Option<Vec2> {
        match self {
            ImageSource::Texture(texture) => Some(texture.size),
            ImageSource::Uri(_) | ImageSource::Bytes { .. } => None,
        }
    }

    /// # Errors
    /// Failure to load the texture.
    pub fn load(
        self,
        ctx: &Context,
        texture_options: TextureOptions,
        size_hint: SizeHint,
    ) -> TextureLoadResult {
        match self {
            Self::Texture(texture) => Ok(TexturePoll::Ready { texture }),
            Self::Uri(uri) => ctx.try_load_texture(uri.as_ref(), texture_options, size_hint),
            Self::Bytes { uri, bytes } => {
                ctx.include_bytes(uri.clone(), bytes);
                ctx.try_load_texture(uri.as_ref(), texture_options, size_hint)
            }
        }
    }

    /// Get the `uri` that this image was constructed from.
    ///
    /// This will return `None` for [`Self::Texture`].
    pub fn uri(&self) -> Option<&str> {
        match self {
            ImageSource::Bytes { uri, .. } | ImageSource::Uri(uri) => Some(uri),
            ImageSource::Texture(_) => None,
        }
    }
}

pub fn paint_texture_load_result(
    ui: &Ui,
    tlr: &TextureLoadResult,
    rect: Rect,
    show_loading_spinner: Option<bool>,
    options: &ImageOptions,
) {
    match tlr {
        Ok(TexturePoll::Ready { texture }) => {
            paint_texture_at(ui.painter(), rect, options, texture);
        }
        Ok(TexturePoll::Pending { .. }) => {
            let show_loading_spinner =
                show_loading_spinner.unwrap_or(ui.visuals().image_loading_spinners);
            if show_loading_spinner {
                Spinner::new().paint_at(ui, rect);
            }
        }
        Err(_) => {
            let font_id = TextStyle::Body.resolve(ui.style());
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "⚠",
                font_id,
                ui.visuals().error_fg_color,
            );
        }
    }
}

/// Attach tooltips like "Loading…" or "Failed loading: …".
pub fn texture_load_result_response(
    source: &ImageSource<'_>,
    tlr: &TextureLoadResult,
    response: Response,
) -> Response {
    match tlr {
        Ok(TexturePoll::Ready { .. }) => response,
        Ok(TexturePoll::Pending { .. }) => {
            let uri = source.uri().unwrap_or("image");
            response.on_hover_text(format!("Loading {uri}…"))
        }
        Err(err) => {
            let uri = source.uri().unwrap_or("image");
            response.on_hover_text(format!("Failed loading {uri}: {err}"))
        }
    }
}

impl<'a> From<&'a str> for ImageSource<'a> {
    #[inline]
    fn from(value: &'a str) -> Self {
        Self::Uri(value.into())
    }
}

impl<'a> From<&'a String> for ImageSource<'a> {
    #[inline]
    fn from(value: &'a String) -> Self {
        Self::Uri(value.as_str().into())
    }
}

impl From<String> for ImageSource<'static> {
    fn from(value: String) -> Self {
        Self::Uri(value.into())
    }
}

impl<'a> From<&'a Cow<'a, str>> for ImageSource<'a> {
    #[inline]
    fn from(value: &'a Cow<'a, str>) -> Self {
        Self::Uri(value.clone())
    }
}

impl<'a> From<Cow<'a, str>> for ImageSource<'a> {
    #[inline]
    fn from(value: Cow<'a, str>) -> Self {
        Self::Uri(value)
    }
}

impl<T: Into<Bytes>> From<(&'static str, T)> for ImageSource<'static> {
    #[inline]
    fn from((uri, bytes): (&'static str, T)) -> Self {
        Self::Bytes {
            uri: uri.into(),
            bytes: bytes.into(),
        }
    }
}

impl<T: Into<Bytes>> From<(Cow<'static, str>, T)> for ImageSource<'static> {
    #[inline]
    fn from((uri, bytes): (Cow<'static, str>, T)) -> Self {
        Self::Bytes {
            uri,
            bytes: bytes.into(),
        }
    }
}

impl<T: Into<Bytes>> From<(String, T)> for ImageSource<'static> {
    #[inline]
    fn from((uri, bytes): (String, T)) -> Self {
        Self::Bytes {
            uri: uri.into(),
            bytes: bytes.into(),
        }
    }
}

impl<T: Into<SizedTexture>> From<T> for ImageSource<'static> {
    fn from(value: T) -> Self {
        Self::Texture(value.into())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ImageOptions {
    /// Select UV range. Default is (0,0) in top-left, (1,1) bottom right.
    pub uv: Rect,

    /// A solid color to put behind the image. Useful for transparent images.
    pub bg_fill: Color32,

    /// Multiply image color with this. Default is WHITE (no tint).
    pub tint: Color32,

    /// Rotate the image about an origin by some angle
    ///
    /// Positive angle is clockwise.
    /// Origin is a vector in normalized UV space ((0,0) in top-left, (1,1) bottom right).
    ///
    /// To rotate about the center you can pass `Vec2::splat(0.5)` as the origin.
    ///
    /// Due to limitations in the current implementation,
    /// this will turn off rounding of the image.
    pub rotation: Option<(Rot2, Vec2)>,

    /// Round the corners of the image.
    ///
    /// The default is no rounding ([`Rounding::ZERO`]).
    ///
    /// Due to limitations in the current implementation,
    /// this will turn off any rotation of the image.
    pub rounding: Rounding,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            bg_fill: Default::default(),
            tint: Color32::WHITE,
            rotation: None,
            rounding: Rounding::ZERO,
        }
    }
}

pub fn paint_texture_at(
    painter: &Painter,
    rect: Rect,
    options: &ImageOptions,
    texture: &SizedTexture,
) {
    if options.bg_fill != Default::default() {
        let mut mesh = Mesh::default();
        mesh.add_colored_rect(rect, options.bg_fill);
        painter.add(Shape::mesh(mesh));
    }

    match options.rotation {
        Some((rot, origin)) => {
            // TODO(emilk): implement this using `PathShape` (add texture support to it).
            // This will also give us anti-aliasing of rotated images.
            egui_assert!(
                options.rounding == Rounding::ZERO,
                "Image had both rounding and rotation. Please pick only one"
            );

            let mut mesh = Mesh::with_texture(texture.id);
            mesh.add_rect_with_uv(rect, options.uv, options.tint);
            mesh.rotate(rot, rect.min + origin * rect.size());
            painter.add(Shape::mesh(mesh));
        }
        None => {
            painter.add(RectShape {
                rect,
                rounding: options.rounding,
                fill: options.tint,
                stroke: Stroke::NONE,
                fill_texture_id: texture.id,
                uv: options.uv,
            });
        }
    }
}
