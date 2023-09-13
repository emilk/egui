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
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Debug, Clone)]
pub struct Image<'a> {
    source: ImageSource<'a>,
    texture_options: TextureOptions,
    image_options: ImageOptions,
    sense: Sense,
    size: ImageSize,
    show_spinner: Option<bool>,
}

impl<'a> Image<'a> {
    /// Load the image from some source.
    pub fn new(source: impl Into<ImageSource<'a>>) -> Self {
        fn new_mono(source: ImageSource<'_>) -> Image<'_> {
            let size = if let ImageSource::Texture(tex) = &source {
                // User is probably expecting their texture to have
                // the exact size of the provided `SizedTexture`.
                ImageSize {
                    maintain_aspect_ratio: false,
                    max_size: None,
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
                show_spinner: None,
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
    /// See [`ImageSource::Bytes`].
    pub fn from_bytes(uri: impl Into<Cow<'static, str>>, bytes: impl Into<Bytes>) -> Self {
        Self::new(ImageSource::Bytes(uri.into(), bytes.into()))
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
        match self.size.max_size.as_mut() {
            Some(max_size) => max_size.x = width,
            None => self.size.max_size = Some(Vec2::new(width, f32::INFINITY)),
        }
        self
    }

    /// Set the max height of the image.
    ///
    /// No matter what the image is scaled to, it will never exceed this limit.
    #[inline]
    pub fn max_height(mut self, height: f32) -> Self {
        match self.size.max_size.as_mut() {
            Some(max_size) => max_size.y = height,
            None => self.size.max_size = Some(Vec2::new(f32::INFINITY, height)),
        }
        self
    }

    /// Set the max size of the image.
    ///
    /// No matter what the image is scaled to, it will never exceed this limit.
    #[inline]
    pub fn max_size(mut self, size: Option<Vec2>) -> Self {
        self.size.max_size = size;
        self
    }

    /// Whether or not the [`ImageFit`] should maintain the image's original aspect ratio.
    #[inline]
    pub fn maintain_aspect_ratio(mut self, value: bool) -> Self {
        self.size.maintain_aspect_ratio = value;
        self
    }

    /// Fit the image to its original size.
    ///
    /// This will cause the image to overflow if it is larger than the available space.
    ///
    /// If [`Image::max_size`] is set, this is guaranteed to never exceed that limit.
    #[inline]
    pub fn fit_to_original_size(mut self, scale: Option<f32>) -> Self {
        self.size.fit = ImageFit::Original(scale);
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
    /// By default this uses the value of [`Style::image_loading_spinners`].
    #[inline]
    pub fn show_spinner(mut self, show: bool) -> Self {
        self.show_spinner = Some(show);
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
    pub fn calculate_size(&self, available_size: Vec2, image_size: Vec2) -> Vec2 {
        self.size.get(available_size, image_size)
    }

    #[inline]
    pub fn size(&self) -> Option<Vec2> {
        match &self.source {
            ImageSource::Texture(texture) => Some(texture.size),
            ImageSource::Uri(_) | ImageSource::Bytes(_, _) => None,
        }
    }

    #[inline]
    pub fn source(&self) -> &ImageSource<'a> {
        &self.source
    }

    /// Get the `uri` that this image was constructed from.
    ///
    /// This will return `<unknown>` for [`ImageSource::Texture`].
    #[inline]
    pub fn uri(&self) -> &str {
        match &self.source {
            ImageSource::Bytes(uri, _) | ImageSource::Uri(uri) => uri,
            // Note: texture source is never in "loading" state
            ImageSource::Texture(_) => "<unknown>",
        }
    }

    /// Load the image from its [`Image::source`], returning the resulting [`SizedTexture`].
    ///
    /// # Errors
    ///
    /// May fail if they underlying [`Context::try_load_texture`] call fails.
    pub fn load(&self, ui: &Ui) -> TextureLoadResult {
        match self.source.clone() {
            ImageSource::Texture(texture) => Ok(TexturePoll::Ready { texture }),
            ImageSource::Uri(uri) => ui.ctx().try_load_texture(
                uri.as_ref(),
                self.texture_options,
                self.size.hint(ui.available_size()),
            ),
            ImageSource::Bytes(uri, bytes) => {
                ui.ctx().include_bytes(uri.clone(), bytes);
                ui.ctx().try_load_texture(
                    uri.as_ref(),
                    self.texture_options,
                    self.size.hint(ui.available_size()),
                )
            }
        }
    }

    #[inline]
    pub fn paint_at(&self, ui: &mut Ui, rect: Rect, texture: &SizedTexture) {
        paint_image_at(ui, rect, &self.image_options, texture);
    }
}

impl<'a> Widget for Image<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        match self.load(ui) {
            Ok(TexturePoll::Ready { texture }) => {
                let size = self.calculate_size(ui.available_size(), texture.size);
                let (rect, response) = ui.allocate_exact_size(size, self.sense);
                self.paint_at(ui, rect, &texture);
                response
            }
            Ok(TexturePoll::Pending { size }) => {
                let spinner = |ui: &mut Ui| {
                    let show_spinner = self
                        .show_spinner
                        .unwrap_or(ui.style().image_loading_spinners);
                    if show_spinner {
                        ui.spinner()
                            .on_hover_text(format!("Loading {:?}…", self.uri()))
                    } else {
                        ui.allocate_response(
                            Vec2::splat(ui.style().spacing.interact_size.y),
                            Sense::hover(),
                        )
                    }
                };

                match size {
                    Some(size) => {
                        let size = self.calculate_size(ui.available_size(), size);
                        ui.allocate_ui(size, |ui| {
                            ui.with_layout(
                                Layout::centered_and_justified(Direction::TopDown),
                                spinner,
                            )
                        })
                        .response
                    }
                    None => spinner(ui),
                }
            }
            Err(err) => ui
                .colored_label(ui.visuals().error_fg_color, "⚠")
                .on_hover_text(err.to_string()),
        }
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
    /// Defaults to `None`
    pub max_size: Option<Vec2>,

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
    /// Fit the image to its original size, optionally scaling it by some factor.
    Original(Option<f32>),

    /// Fit the image to a fraction of the available size.
    Fraction(Vec2),

    /// Fit the image to an exact size.
    Exact(Vec2),
}

impl ImageSize {
    fn hint(&self, available_size: Vec2) -> SizeHint {
        if self.maintain_aspect_ratio {
            return SizeHint::Scale(1.0.ord());
        };

        let fit = match self.fit {
            ImageFit::Original(scale) => return SizeHint::Scale(scale.unwrap_or(1.0).ord()),
            ImageFit::Fraction(fract) => available_size * fract,
            ImageFit::Exact(size) => size,
        };

        let fit = match self.max_size {
            Some(extent) => fit.min(extent),
            None => fit,
        };

        // `inf` on an axis means "any value"
        match (fit.x.is_finite(), fit.y.is_finite()) {
            (true, true) => SizeHint::Size(fit.x.round() as u32, fit.y.round() as u32),
            (true, false) => SizeHint::Width(fit.x.round() as u32),
            (false, true) => SizeHint::Height(fit.y.round() as u32),
            (false, false) => SizeHint::Scale(1.0.ord()),
        }
    }

    fn get(&self, available_size: Vec2, image_size: Vec2) -> Vec2 {
        match self.fit {
            ImageFit::Original(scale) => {
                let image_size = image_size * scale.unwrap_or(1.0);

                if let Some(available_size) = self.max_size {
                    if image_size.x < available_size.x && image_size.y < available_size.y {
                        return image_size;
                    }

                    if self.maintain_aspect_ratio {
                        let ratio_x = available_size.x / image_size.x;
                        let ratio_y = available_size.y / image_size.y;
                        let ratio = if ratio_x < ratio_y { ratio_x } else { ratio_y };
                        let ratio = if ratio.is_infinite() { 1.0 } else { ratio };

                        return Vec2::new(image_size.x * ratio, image_size.y * ratio);
                    } else {
                        return image_size.min(available_size);
                    }
                }

                image_size
            }
            ImageFit::Fraction(fract) => {
                let available_size = available_size * fract;
                let available_size = match self.max_size {
                    Some(max_size) => available_size.min(max_size),
                    None => available_size,
                };

                if self.maintain_aspect_ratio {
                    let ratio_x = available_size.x / image_size.x;
                    let ratio_y = available_size.y / image_size.y;
                    let ratio = if ratio_x < ratio_y { ratio_x } else { ratio_y };
                    let ratio = if ratio.is_infinite() { 1.0 } else { ratio };

                    return Vec2::new(image_size.x * ratio, image_size.y * ratio);
                }

                available_size
            }
            ImageFit::Exact(size) => {
                let available_size = size;
                let available_size = match self.max_size {
                    Some(max_size) => available_size.min(max_size),
                    None => available_size,
                };

                if self.maintain_aspect_ratio {
                    let ratio_x = available_size.x / image_size.x;
                    let ratio_y = available_size.y / image_size.y;
                    let ratio = if ratio_x < ratio_y { ratio_x } else { ratio_y };
                    let ratio = if ratio.is_infinite() { 1.0 } else { ratio };

                    return Vec2::new(image_size.x * ratio, image_size.y * ratio);
                }

                available_size
            }
        }
    }
}

impl Default for ImageSize {
    #[inline]
    fn default() -> Self {
        Self {
            max_size: None,
            fit: ImageFit::Fraction(Vec2::new(1.0, 1.0)),
            maintain_aspect_ratio: true,
        }
    }
}

/// This type tells the [`Ui`] how to load an image.
///
/// This is used by [`Image::new`] and [`Ui::image`].
#[derive(Debug, Clone)]
pub enum ImageSource<'a> {
    /// Load the image from a URI.
    ///
    /// This could be a `file://` url, `http(s)?://` url, or a `bare` identifier.
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
    Bytes(Cow<'static, str>, Bytes),
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
        Self::Bytes(uri.into(), bytes.into())
    }
}

impl<T: Into<Bytes>> From<(Cow<'static, str>, T)> for ImageSource<'static> {
    #[inline]
    fn from((uri, bytes): (Cow<'static, str>, T)) -> Self {
        Self::Bytes(uri, bytes.into())
    }
}

impl<T: Into<Bytes>> From<(String, T)> for ImageSource<'static> {
    #[inline]
    fn from((uri, bytes): (String, T)) -> Self {
        Self::Bytes(uri.into(), bytes.into())
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

/// Paint a `SizedTexture` as an image according to some `ImageOptions` at a given `rect`.
pub fn paint_image_at(ui: &mut Ui, rect: Rect, options: &ImageOptions, texture: &SizedTexture) {
    if !ui.is_rect_visible(rect) {
        return;
    }

    if options.bg_fill != Default::default() {
        let mut mesh = Mesh::default();
        mesh.add_colored_rect(rect, options.bg_fill);
        ui.painter().add(Shape::mesh(mesh));
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
            ui.painter().add(Shape::mesh(mesh));
        }
        None => {
            ui.painter().add(RectShape {
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
