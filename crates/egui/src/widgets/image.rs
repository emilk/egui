use crate::*;
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
    pub fn rotate(mut self, angle: f32, origin: Vec2) -> Self {
        self.rotation = Some((Rot2::from_angle(angle), origin));
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
            } = self;

            if *bg_fill != Default::default() {
                let mut mesh = Mesh::default();
                mesh.add_colored_rect(rect, *bg_fill);
                ui.painter().add(Shape::mesh(mesh));
            }

            {
                // TODO(emilk): builder pattern for Mesh
                let mut mesh = Mesh::with_texture(*texture_id);
                mesh.add_rect_with_uv(rect, *uv, *tint);
                if let Some((rot, origin)) = rotation {
                    mesh.rotate(*rot, rect.min + *origin * *size);
                }
                ui.painter().add(Shape::mesh(mesh));
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
