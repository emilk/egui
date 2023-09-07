use crate::load::TextureLoadResult;
use crate::{
    load::{Bytes, SizeHint, SizedTexture, TexturePoll},
    *,
};
use emath::Rot2;
use epaint::{util::FloatOrd, RectShape};

/// A widget which displays an image.
///
/// There are two ways to construct this widget:
/// - [`Image2::from_uri`]
/// - [`Image2::from_bytes`]
///
/// In both cases the task of actually loading the image
/// is deferred to when the `Image2` is added to the [`Ui`].
///
/// See [`crate::load`] for more information.
#[derive(Debug, Clone)]
pub struct Image2 {
    source: ImageSource,
    texture_options: TextureOptions,
    image_options: ImageOptions,
    sense: Sense,
    size: ImageSize,
}

impl Image2 {
    /// Load the image from some source.
    pub fn new(source: ImageSource) -> Self {
        Self {
            source,
            texture_options: Default::default(),
            image_options: ImageOptions {
                uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                bg_fill: Default::default(),
                tint: Color32::WHITE,
                rotation: None,
                rounding: Rounding::ZERO,
            },
            sense: Sense::hover(),
            size: Default::default(),
        }
    }

    /// Load the image from a URI.
    ///
    /// See [`ImageSource::Uri`].
    pub fn from_uri<S: Into<String>>(uri: S) -> Self {
        Self::new(ImageSource::Uri(uri.into()))
    }

    /// Load the image from an existing texture.
    ///
    /// See [`ImageSource::Texture`].
    pub fn from_texture(texture: SizedTexture) -> Self {
        Self::new(ImageSource::Texture(texture))
    }

    /// Load the image from some raw bytes.
    ///
    /// See [`ImageSource::Bytes`].
    pub fn from_bytes(uri: &'static str, bytes: impl Into<Bytes>) -> Self {
        Self::new(ImageSource::Bytes(uri, bytes.into()))
    }

    /// Texture options used when creating the texture.
    #[inline]
    pub fn texture_options(mut self, texture_options: TextureOptions) -> Self {
        self.texture_options = texture_options;
        self
    }

    #[inline]
    pub fn extent(mut self, extent: Option<Vec2>) -> Self {
        self.size.extent = extent;
        self
    }

    #[inline]
    pub fn fit_to_original_size(mut self, scale: Option<f32>) -> Self {
        self.size.fit = ImageFit::Original(scale);
        self
    }

    #[inline]
    pub fn fit_to_exact_size(mut self, size: Vec2) -> Self {
        self.size.fit = ImageFit::Exact(size);
        self
    }

    #[inline]
    pub fn fit_to_fraction(mut self, fraction: Vec2) -> Self {
        self.size.fit = ImageFit::Fraction(fraction);
        self
    }

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
    pub fn uv(mut self, uv: impl Into<Rect>) -> Self {
        self.image_options.uv = uv.into();
        self
    }

    /// A solid color to put behind the image. Useful for transparent images.
    pub fn bg_fill(mut self, bg_fill: impl Into<Color32>) -> Self {
        self.image_options.bg_fill = bg_fill.into();
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
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
    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.image_options.rounding = rounding.into();
        if self.image_options.rounding != Rounding::ZERO {
            self.image_options.rotation = None; // incompatible with rounding
        }
        self
    }
}

impl Image2 {
    pub fn calculate_size(&self, available_size: Vec2, image_size: Vec2) -> Vec2 {
        self.size.get(available_size, image_size)
    }

    pub fn uri(&self) -> &str {
        match &self.source {
            ImageSource::Uri(uri) => uri,
            ImageSource::Bytes(uri, _) => uri,
            // Note: texture source is never in "loading" state
            ImageSource::Texture(_) => "<unknown>",
        }
    }

    fn load_texture(&self, ui: &Ui) -> TextureLoadResult {
        match self.source.clone() {
            ImageSource::Texture(texture) => Ok(TexturePoll::Ready { texture }),
            ImageSource::Uri(uri) => ui.ctx().try_load_texture(
                &uri,
                self.texture_options,
                self.size.hint(ui.available_size()),
            ),
            ImageSource::Bytes(uri, bytes) => {
                ui.ctx().include_bytes(uri, bytes);
                ui.ctx().try_load_texture(
                    uri,
                    self.texture_options,
                    self.size.hint(ui.available_size()),
                )
            }
        }
    }

    fn paint_at(&self, ui: &mut Ui, rect: Rect, texture: &SizedTexture) {
        paint_image_at(ui, rect, &self.image_options, texture);
    }
}

impl Widget for Image2 {
    fn ui(self, ui: &mut Ui) -> Response {
        match self.load_texture(ui) {
            Ok(TexturePoll::Ready { texture }) => {
                let size = self.calculate_size(ui.available_size(), texture.size);
                let (rect, response) = ui.allocate_exact_size(size, self.sense);
                self.paint_at(ui, rect, &texture);
                response
            }
            Ok(TexturePoll::Pending { size }) => match size {
                Some(size) => {
                    let size = self.calculate_size(ui.available_size(), size);
                    ui.allocate_ui(size, |ui| {
                        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                            ui.spinner()
                                .on_hover_text(format!("Loading {:?}…", self.uri()))
                        })
                    })
                    .response
                }
                None => ui
                    .spinner()
                    .on_hover_text(format!("Loading {:?}…", self.uri())),
            },
            Err(err) => ui.colored_label(ui.visuals().error_fg_color, err.to_string()),
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
    /// Determines the maximum extent of the image.
    ///
    /// This setting is applied after calculating `fit`.
    ///
    /// Defaults to `None`
    pub extent: Option<Vec2>,
    /// Determines how the image should shrink/expand/stretch/etc. to fit within its allocated space.
    ///
    /// This setting is applied first.
    ///
    /// Defaults to `ImageFit::Fraction([1, 1])`
    pub fit: ImageFit,
}

/// This type determines how the image should try to fit within the UI.
///
/// This has lower precedence than [`ImageSize::extents`], meaning that the image size will be clamped to [`ImageSize::extents`].
#[derive(Debug, Clone, Copy)]
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
            return SizeHint::Original(None);
        };

        let fit = match self.fit {
            ImageFit::Original(scale) => return SizeHint::Original(scale.map(FloatOrd::ord)),
            ImageFit::Fraction(fract) => available_size * fract,
            ImageFit::Exact(size) => size,
        };

        let fit = match self.extent {
            Some(extent) => fit.min(extent),
            None => fit,
        };

        // `inf` on an axis means "any value"
        match (fit.x.is_finite(), fit.y.is_finite()) {
            (true, true) => SizeHint::Size(fit.x.round() as u32, fit.y.round() as u32),
            (true, false) => SizeHint::Width(fit.x.round() as u32),
            (false, true) => SizeHint::Height(fit.y.round() as u32),
            (false, false) => SizeHint::Original(None),
        }
    }

    fn get(&self, available_size: Vec2, image_size: Vec2) -> Vec2 {
        match self.fit {
            ImageFit::Original(None) => {
                if let Some(available_size) = self.extent {
                    if self.maintain_aspect_ratio {
                        let ratio_x = available_size.x / image_size.x;
                        let ratio_y = available_size.y / image_size.y;
                        let ratio = if ratio_x < ratio_y { ratio_x } else { ratio_y };

                        return Vec2::new(image_size.x * ratio, image_size.y * ratio);
                    }
                }

                image_size
            }
            ImageFit::Original(Some(scale)) => {
                let image_size = image_size * scale;

                if let Some(available_size) = self.extent {
                    if self.maintain_aspect_ratio {
                        let ratio_x = available_size.x / image_size.x;
                        let ratio_y = available_size.y / image_size.y;
                        let ratio = if ratio_x < ratio_y { ratio_x } else { ratio_y };

                        return Vec2::new(image_size.x * ratio, image_size.y * ratio);
                    }
                }

                image_size
            }
            ImageFit::Fraction(fract) => {
                let available_size =
                    (available_size * fract).max(self.extent.unwrap_or(Vec2::ZERO));

                if self.maintain_aspect_ratio {
                    let ratio_x = available_size.x / image_size.x;
                    let ratio_y = available_size.y / image_size.y;
                    let ratio = if ratio_x < ratio_y { ratio_x } else { ratio_y };

                    return Vec2::new(image_size.x * ratio, image_size.y * ratio);
                }

                available_size
            }
            ImageFit::Exact(size) => {
                let available_size = size.max(self.extent.unwrap_or(Vec2::ZERO));

                if self.maintain_aspect_ratio {
                    let ratio_x = available_size.x / image_size.x;
                    let ratio_y = available_size.y / image_size.y;
                    let ratio = if ratio_x < ratio_y { ratio_x } else { ratio_y };

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
            extent: None,
            fit: ImageFit::Fraction(Vec2::new(1.0, 1.0)),
            maintain_aspect_ratio: true,
        }
    }
}

/// This type tells the [`Ui`] how to load the image.
#[derive(Debug, Clone)]
pub enum ImageSource {
    /// Load the image from a URI.
    ///
    /// This could be a `file://` url, `http://` url, or a `bare` identifier.
    /// How the URI will be turned into a texture for rendering purposes is
    /// up to the registered loaders to handle.
    ///
    /// See [`crate::load`] for more information.
    Uri(String),

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
    /// See [`crate::load`] for more information.
    Bytes(&'static str, Bytes),
}

impl<'a> From<&'a str> for ImageSource {
    fn from(value: &'a str) -> Self {
        Self::Uri(value.into())
    }
}

impl From<String> for ImageSource {
    #[inline]
    fn from(value: String) -> Self {
        Self::Uri(value)
    }
}

impl<T: Into<Bytes>> From<(&'static str, T)> for ImageSource {
    #[inline]
    fn from((uri, bytes): (&'static str, T)) -> Self {
        Self::Bytes(uri, bytes.into())
    }
}

/// A widget which displays a sized texture.
///
/// In both cases the task of actually loading the image
/// is deferred to when the `Image2` is added to the [`Ui`].
///
/// See [`crate::load`] for more information.
#[derive(Debug, Clone)]
pub struct RawImage {
    texture: SizedTexture,
    texture_options: TextureOptions,
    image_options: ImageOptions,
    sense: Sense,
}

impl RawImage {
    /// Load the image from some source.
    pub fn new(texture: SizedTexture) -> Self {
        Self {
            texture,
            texture_options: Default::default(),
            image_options: ImageOptions {
                uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                bg_fill: Default::default(),
                tint: Color32::WHITE,
                rotation: None,
                rounding: Rounding::ZERO,
            },
            sense: Sense::hover(),
        }
    }

    /// Texture options used when creating the texture.
    #[inline]
    pub fn texture_options(mut self, texture_options: TextureOptions) -> Self {
        self.texture_options = texture_options;
        self
    }

    /// Make the image respond to clicks and/or drags.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Select UV range. Default is (0,0) in top-left, (1,1) bottom right.
    pub fn uv(mut self, uv: impl Into<Rect>) -> Self {
        self.image_options.uv = uv.into();
        self
    }

    /// A solid color to put behind the image. Useful for transparent images.
    pub fn bg_fill(mut self, bg_fill: impl Into<Color32>) -> Self {
        self.image_options.bg_fill = bg_fill.into();
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
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
    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.image_options.rounding = rounding.into();
        if self.image_options.rounding != Rounding::ZERO {
            self.image_options.rotation = None; // incompatible with rounding
        }
        self
    }
}

impl RawImage {
    pub fn size(&self) -> Vec2 {
        self.texture.size
    }

    pub fn paint_at(&self, ui: &mut Ui, rect: Rect) {
        paint_image_at(ui, rect, &self.image_options, &self.texture);
    }
}

impl Widget for RawImage {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.size(), self.sense);
        self.paint_at(ui, rect);
        response
    }
}

#[derive(Debug, Clone)]
pub struct ImageOptions {
    uv: Rect,
    bg_fill: Color32,
    tint: Color32,
    rotation: Option<(Rot2, Vec2)>,
    rounding: Rounding,
}

pub fn paint_image_at(ui: &mut Ui, rect: Rect, options: &ImageOptions, texture: &SizedTexture) {
    if !ui.is_rect_visible(rect) {
        return;
    }

    let mut mesh = Mesh::with_texture(texture.id);
    mesh.add_rect_with_uv(
        rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    ui.painter().add(Shape::mesh(mesh));

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
