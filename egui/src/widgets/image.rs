use crate::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct Image {
    texture_id: TextureId,
    desired_size: Vec2,
    bg_fill: Srgba,
    tint: Srgba,
}

impl Image {
    pub fn new(texture_id: TextureId, desired_size: Vec2) -> Self {
        Self {
            texture_id,
            desired_size,
            tint: color::WHITE,
            ..Default::default()
        }
    }

    /// A solid color to put behind the image. Useful for transparent images.
    pub fn bg_fill(mut self, bg_fill: Srgba) -> Self {
        self.bg_fill = bg_fill;
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
    pub fn tint(mut self, tint: Srgba) -> Self {
        self.tint = tint;
        self
    }
}

impl Widget for Image {
    fn ui(self, ui: &mut Ui) -> Response {
        use paint::*;
        let Self {
            texture_id,
            desired_size,
            bg_fill,
            tint,
        } = self;
        let rect = ui.allocate_space(desired_size);
        if bg_fill != Default::default() {
            let mut triangles = Triangles::default();
            triangles.add_colored_rect(rect, bg_fill);
            ui.painter().add(PaintCmd::Triangles(triangles));
        }
        {
            // TODO: builder pattern for Triangles
            let uv = [pos2(0.0, 0.0), pos2(1.0, 1.0)];
            let mut triangles = Triangles::with_texture(texture_id);
            triangles.add_rect_with_uv(rect, uv.into(), tint);
            ui.painter().add(PaintCmd::Triangles(triangles));
        }

        ui.interact_hover(rect)
    }
}
