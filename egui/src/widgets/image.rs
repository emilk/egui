use crate::*;

/// An widget to show an image of a given size.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Clone, Copy, Debug)]
pub struct Image {
    texture_id: TextureId,
    uv: Rect,
    size: Vec2,
    bg_fill: Color32,
    tint: Color32,
}

impl Image {
    pub fn new(texture_id: TextureId, size: impl Into<Vec2>) -> Self {
        Self {
            texture_id,
            uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            size: size.into(),
            bg_fill: Default::default(),
            tint: Color32::WHITE,
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
}

impl Image {
    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn paint_at(&self, ui: &mut Ui, rect: Rect) {
        use paint::*;
        let Self {
            texture_id,
            uv,
            size: _,
            bg_fill,
            tint,
        } = self;

        if *bg_fill != Default::default() {
            let mut triangles = Triangles::default();
            triangles.add_colored_rect(rect, *bg_fill);
            ui.painter().add(Shape::triangles(triangles));
        }

        {
            // TODO: builder pattern for Triangles
            let mut triangles = Triangles::with_texture(*texture_id);
            triangles.add_rect_with_uv(rect, *uv, *tint);
            ui.painter().add(Shape::triangles(triangles));
        }
    }
}

impl Widget for Image {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::hover());
        self.paint_at(ui, rect);
        response
    }
}
