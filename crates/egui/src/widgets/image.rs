use std::sync::Arc;

use crate::load::Bytes;
use crate::{load::SizeHint, load::TexturePoll, *};
use emath::Rot2;

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
/// There are three ways to construct this widget:
/// - [`Image2::from_uri`]
/// - [`Image2::from_bytes`]
/// - [`Image2::from_static_bytes`]
///
/// In both cases the task of actually loading the image
/// is deferred to when the `Image2` is added to the [`Ui`].
///
/// See [`crate::load`] for more information.
pub struct Image2<'a> {
    source: ImageSource<'a>,
    texture_options: TextureOptions,
    size_hint: SizeHint,
    fit: ImageFit,
    sense: Sense,
}

#[derive(Default, Clone, Copy)]
enum ImageFit {
    // TODO: options for aspect ratio
    // TODO: other fit strategies
    // FitToWidth,
    // FitToHeight,
    // FitToWidthExact(f32),
    // FitToHeightExact(f32),
    #[default]
    ShrinkToFit,
}

impl ImageFit {
    pub fn calculate_final_size(&self, available_size: Vec2, image_size: Vec2) -> Vec2 {
        let aspect_ratio = image_size.x / image_size.y;
        // TODO: more image sizing options
        match self {
            // ImageFit::FitToWidth => todo!(),
            // ImageFit::FitToHeight => todo!(),
            // ImageFit::FitToWidthExact(_) => todo!(),
            // ImageFit::FitToHeightExact(_) => todo!(),
            ImageFit::ShrinkToFit => {
                let width = if available_size.x < image_size.x {
                    available_size.x
                } else {
                    image_size.x
                };
                let height = if available_size.y < image_size.y {
                    available_size.y
                } else {
                    image_size.y
                };
                if width < height {
                    Vec2::new(width, width / aspect_ratio)
                } else {
                    Vec2::new(height * aspect_ratio, height)
                }
            }
        }
    }
}

/// This type tells the [`Ui`] how to load the image.
pub enum ImageSource<'a> {
    /// Load the image from a URI.
    ///
    /// This could be a `file://` url, `http://` url, or a `bare` identifier.
    /// How the URI will be turned into a texture for rendering purposes is
    /// up to the registered loaders to handle.
    ///
    /// See [`crate::load`] for more information.
    Uri(&'a str),

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
            size_hint: Default::default(),
            fit: Default::default(),
            sense: Sense::hover(),
        }
    }

    /// Load the image from a URI.
    ///
    /// See [`ImageSource::Uri`].
    pub fn from_uri(uri: &'a str) -> Self {
        Self {
            source: ImageSource::Uri(uri),
            texture_options: Default::default(),
            size_hint: Default::default(),
            fit: Default::default(),
            sense: Sense::hover(),
        }
    }

    /// Load the image from some raw `'static` bytes.
    ///
    /// For example, you can use this to load an image from bytes obtained via [`include_bytes`].
    ///
    /// See [`ImageSource::Bytes`].
    pub fn from_static_bytes(name: &'static str, bytes: &'static [u8]) -> Self {
        Self {
            source: ImageSource::Bytes(name, Bytes::Static(bytes)),
            texture_options: Default::default(),
            size_hint: Default::default(),
            fit: Default::default(),
            sense: Sense::hover(),
        }
    }

    /// Load the image from some raw bytes.
    ///
    /// See [`ImageSource::Bytes`].
    pub fn from_bytes(name: &'static str, bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            source: ImageSource::Bytes(name, Bytes::Shared(bytes.into())),
            texture_options: Default::default(),
            size_hint: Default::default(),
            fit: Default::default(),
            sense: Sense::hover(),
        }
    }

    /// Texture options used when creating the texture.
    #[inline]
    pub fn texture_options(mut self, texture_options: TextureOptions) -> Self {
        self.texture_options = texture_options;
        self
    }

    /// Size hint used when creating the texture.
    #[inline]
    pub fn size_hint(mut self, size_hint: impl Into<SizeHint>) -> Self {
        self.size_hint = size_hint.into();
        self
    }

    /// Make the image respond to clicks and/or drags.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }
}

impl<'a> Widget for Image2<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let uri = match self.source {
            ImageSource::Uri(uri) => uri,
            ImageSource::Bytes(uri, bytes) => {
                match bytes {
                    Bytes::Static(bytes) => ui.ctx().include_static_bytes(uri, bytes),
                    Bytes::Shared(bytes) => ui.ctx().include_bytes(uri, bytes),
                }
                uri
            }
        };

        match ui
            .ctx()
            .try_load_texture(uri, self.texture_options, self.size_hint)
        {
            Ok(TexturePoll::Ready { texture }) => {
                let final_size = self.fit.calculate_final_size(
                    ui.available_size(),
                    Vec2::new(texture.size[0] as f32, texture.size[1] as f32),
                );

                let (rect, response) = ui.allocate_exact_size(final_size, self.sense);

                let mut mesh = Mesh::with_texture(texture.id);
                mesh.add_rect_with_uv(
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                ui.painter().add(Shape::mesh(mesh));

                response
            }
            Ok(TexturePoll::Pending { .. }) => {
                ui.spinner().on_hover_text(format!("Loading {uri:?}…"))
            }
            Err(err) => ui.colored_label(ui.visuals().error_fg_color, err.to_string()),
        }
    }
}
