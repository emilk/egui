use crate::load::TextureLoadResult;
use crate::{
    load::{Bytes, SizeHint, SizedTexture, TexturePoll},
    *,
};
use emath::Rot2;
use epaint::{util::FloatOrd, RectShape};

/// An widget to show an image of a given size.
///
/// In order to display an image you must first acquire a [`TextureHandle`].
/// This is best done with [`egui_extras::RetainedImage`](https://docs.rs/egui_extras/latest/egui_extras/image/struct.RetainedImage.html) or [`Context::load_texture`].
///
/// ```
/// struct MyImage {
///     texture: Option<egui::TextureHandle>,
/// }
///
/// impl MyImage {
///     fn ui(&mut self, ui: &mut egui::Ui) {
///         let texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
///             // Load the texture only once.
///             ui.ctx().load_texture(
///                 "my-image",
///                 egui::ColorImage::example(),
///                 Default::default()
///             )
///         });
///
///         // Show the image:
///         ui.add(egui::Image::new(texture, texture.size_vec2()));
///
///         // Shorter version:
///         ui.image(texture, texture.size_vec2());
///     }
/// }
/// ```
///
/// Se also [`crate::Ui::image`] and [`crate::ImageButton`].
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Clone, Copy, Debug)]
pub struct Image {
    texture_id: TextureId,
    uv: Rect,
    size: Vec2,
    bg_fill: Color32,
    tint: Color32,
    sense: Sense,
    rotation: Option<(Rot2, Vec2)>,
    rounding: Rounding,
}

impl Image {
    pub fn new(texture_id: impl Into<TextureId>, size: impl Into<Vec2>) -> Self {
        Self {
            texture_id: texture_id.into(),
            uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            size: size.into(),
            bg_fill: Default::default(),
            tint: Color32::WHITE,
            sense: Sense::hover(),
            rotation: None,
            rounding: Rounding::ZERO,
        }
    }

    /// Select UV range. Default is (0,0) in top-left, (1,1) bottom right.
    pub fn uv(mut self, uv: impl Into<Rect>) -> Self {
        self.uv = uv.into();
        self
    }

    /// A solid color to put behind the image. Useful for transparent images.
    pub fn bg_fill(mut self, bg_fill: impl Into<Color32>) -> Self {
        self.bg_fill = bg_fill.into();
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
    pub fn tint(mut self, tint: impl Into<Color32>) -> Self {
        self.tint = tint.into();
        self
    }

    /// Make the image respond to clicks and/or drags.
    ///
    /// Consider using [`ImageButton`] instead, for an on-hover effect.
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
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
        self.rotation = Some((Rot2::from_angle(angle), origin));
        self.rounding = Rounding::ZERO; // incompatible with rotation
        self
    }

    /// Round the corners of the image.
    ///
    /// The default is no rounding ([`Rounding::ZERO`]).
    ///
    /// Due to limitations in the current implementation,
    /// this will turn off any rotation of the image.
    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.rounding = rounding.into();
        if self.rounding != Rounding::ZERO {
            self.rotation = None; // incompatible with rounding
        }
        self
    }
}

impl Image {
    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn paint_at(&self, ui: &mut Ui, rect: Rect) {
        if ui.is_rect_visible(rect) {
            use epaint::*;
            let Self {
                texture_id,
                uv,
                size,
                bg_fill,
                tint,
                sense: _,
                rotation,
                rounding,
            } = self;

            if *bg_fill != Default::default() {
                let mut mesh = Mesh::default();
                mesh.add_colored_rect(rect, *bg_fill);
                ui.painter().add(Shape::mesh(mesh));
            }

            if let Some((rot, origin)) = rotation {
                // TODO(emilk): implement this using `PathShape` (add texture support to it).
                // This will also give us anti-aliasing of rotated images.
                egui_assert!(
                    *rounding == Rounding::ZERO,
                    "Image had both rounding and rotation. Please pick only one"
                );

                let mut mesh = Mesh::with_texture(*texture_id);
                mesh.add_rect_with_uv(rect, *uv, *tint);
                mesh.rotate(*rot, rect.min + *origin * *size);
                ui.painter().add(Shape::mesh(mesh));
            } else {
                ui.painter().add(RectShape {
                    rect,
                    rounding: *rounding,
                    fill: *tint,
                    stroke: Stroke::NONE,
                    fill_texture_id: *texture_id,
                    uv: *uv,
                });
            }
        }
    }
}

impl Widget for Image {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.size, self.sense);
        self.paint_at(ui, rect);
        response
    }
}

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
pub struct Image2<'a> {
    source: ImageSource<'a>,
    texture_options: TextureOptions,
    size: ImageSize,
    sense: Sense,
    uv: Rect,
    bg_fill: Color32,
    tint: Color32,
    rotation: Option<(Rot2, Vec2)>,
    rounding: Rounding,
}

/// This type determines the constraints on how
/// the size of an image should be calculated.
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
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

    fn finalize(&self, available_size: Vec2, image_size: Vec2) -> Vec2 {
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
#[derive(Clone)]
pub enum ImageSource<'a> {
    /// Load the image from a URI.
    ///
    /// This could be a `file://` url, `http://` url, or a `bare` identifier.
    /// How the URI will be turned into a texture for rendering purposes is
    /// up to the registered loaders to handle.
    ///
    /// See [`crate::load`] for more information.
    Uri(&'a str),

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

impl<'a> From<&'a str> for ImageSource<'a> {
    #[inline]
    fn from(value: &'a str) -> Self {
        Self::Uri(value)
    }
}

impl<T: Into<Bytes>> From<(&'static str, T)> for ImageSource<'static> {
    #[inline]
    fn from((uri, bytes): (&'static str, T)) -> Self {
        Self::Bytes(uri, bytes.into())
    }
}

impl<'a> Image2<'a> {
    /// Load the image from some source.
    pub fn new(source: ImageSource<'a>) -> Self {
        Self {
            source,
            texture_options: Default::default(),
            size: Default::default(),
            sense: Sense::hover(),
            uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            bg_fill: Default::default(),
            tint: Color32::WHITE,
            rotation: None,
            rounding: Rounding::ZERO,
        }
    }

    /// Load the image from a URI.
    ///
    /// See [`ImageSource::Uri`].
    pub fn from_uri(uri: &'a str) -> Self {
        Self::new(ImageSource::Uri(uri))
    }

    /// Load the iamge from an existing texture.
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
        self.uv = uv.into();
        self
    }

    /// A solid color to put behind the image. Useful for transparent images.
    pub fn bg_fill(mut self, bg_fill: impl Into<Color32>) -> Self {
        self.bg_fill = bg_fill.into();
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
    pub fn tint(mut self, tint: impl Into<Color32>) -> Self {
        self.tint = tint.into();
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
        self.rotation = Some((Rot2::from_angle(angle), origin));
        self.rounding = Rounding::ZERO; // incompatible with rotation
        self
    }

    /// Round the corners of the image.
    ///
    /// The default is no rounding ([`Rounding::ZERO`]).
    ///
    /// Due to limitations in the current implementation,
    /// this will turn off any rotation of the image.
    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.rounding = rounding.into();
        if self.rounding != Rounding::ZERO {
            self.rotation = None; // incompatible with rounding
        }
        self
    }

    fn load_texture(&self, ui: &Ui) -> TextureLoadResult {
        match self.source.clone() {
            ImageSource::Texture(texture) => Ok(TexturePoll::Ready { texture }),
            ImageSource::Uri(uri) => ui.ctx().try_load_texture(
                uri,
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

    fn uri(&self) -> &str {
        match self.source {
            ImageSource::Uri(uri) | ImageSource::Bytes(uri, _) => uri,
            // Note: texture source is never in "loading" state
            ImageSource::Texture(_) => "<unknown>",
        }
    }

    fn paint_at(&self, ui: &mut Ui, rect: Rect, texture: &SizedTexture) {
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

        if self.bg_fill != Default::default() {
            let mut mesh = Mesh::default();
            mesh.add_colored_rect(rect, self.bg_fill);
            ui.painter().add(Shape::mesh(mesh));
        }

        match self.rotation {
            Some((rot, origin)) => {
                // TODO(emilk): implement this using `PathShape` (add texture support to it).
                // This will also give us anti-aliasing of rotated images.
                egui_assert!(
                    self.rounding == Rounding::ZERO,
                    "Image had both rounding and rotation. Please pick only one"
                );

                let mut mesh = Mesh::with_texture(texture.id);
                mesh.add_rect_with_uv(rect, self.uv, self.tint);
                mesh.rotate(rot, rect.min + origin * rect.size());
                ui.painter().add(Shape::mesh(mesh));
            }
            None => {
                ui.painter().add(RectShape {
                    rect,
                    rounding: self.rounding,
                    fill: self.tint,
                    stroke: Stroke::NONE,
                    fill_texture_id: texture.id,
                    uv: self.uv,
                });
            }
        }
    }
}

impl<'a> Widget for Image2<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        match self.load_texture(ui) {
            Ok(TexturePoll::Ready { texture }) => {
                let final_size = self.size.finalize(ui.available_size(), texture.size_f32());
                let (rect, response) = ui.allocate_exact_size(final_size, self.sense);
                self.paint_at(ui, rect, &texture);
                response
            }
            Ok(TexturePoll::Pending { size }) => match size {
                Some(size) => {
                    ui.allocate_ui(Vec2::new(size[0] as f32, size[1] as f32), |ui| {
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
