use crate::{
    layers::{LayerId, ShapeIdx},
    math::{Align2, Pos2, Rect, Vec2},
    paint::{
        text::{Fonts, Galley, TextStyle},
        Shape, Stroke,
    },
    Color32, CtxRef,
};

/// Helper to paint shapes and text to a specific region on a specific layer.
///
/// All coordinates are screen coordinates in the unit points (one point can consist of many physical pixels).
#[derive(Clone)]
pub struct Painter {
    /// Source of fonts and destination of shapes
    ctx: CtxRef,

    /// Where we paint
    layer_id: LayerId,

    /// Everything painted in this `Painter` will be clipped against this.
    /// This means nothing outside of this rectangle will be visible on screen.
    clip_rect: Rect,
}

impl Painter {
    pub fn new(ctx: CtxRef, layer_id: LayerId, clip_rect: Rect) -> Self {
        Self {
            ctx,
            layer_id,
            clip_rect,
        }
    }

    #[must_use]
    pub fn with_layer_id(self, layer_id: LayerId) -> Self {
        Self {
            ctx: self.ctx,
            layer_id,
            clip_rect: self.clip_rect,
        }
    }

    /// redirect
    pub fn set_layer_id(&mut self, layer_id: LayerId) {
        self.layer_id = layer_id;
    }

    /// Create a painter for a sub-region of this `Painter`.
    ///
    /// The clip-rect of the returned `Painter` will be the intersection
    /// of the given rectangle and the `clip_rect()` of this `Painter`.
    pub fn sub_region(&self, rect: Rect) -> Self {
        Self::new(
            self.ctx.clone(),
            self.layer_id,
            rect.intersect(self.clip_rect),
        )
    }
}

/// ## Accessors etc
impl Painter {
    pub(crate) fn ctx(&self) -> &CtxRef {
        &self.ctx
    }

    /// Available fonts
    pub(crate) fn fonts(&self) -> &Fonts {
        self.ctx.fonts()
    }

    /// Where we paint
    pub fn layer_id(&self) -> LayerId {
        self.layer_id
    }

    /// Everything painted in this `Painter` will be clipped against this.
    /// This means nothing outside of this rectangle will be visible on screen.
    pub fn clip_rect(&self) -> Rect {
        self.clip_rect
    }

    /// Everything painted in this `Painter` will be clipped against this.
    /// This means nothing outside of this rectangle will be visible on screen.
    pub fn set_clip_rect(&mut self, clip_rect: Rect) {
        self.clip_rect = clip_rect;
    }

    /// Useful for pixel-perfect rendering
    pub fn round_to_pixel(&self, point: f32) -> f32 {
        self.ctx().round_to_pixel(point)
    }

    /// Useful for pixel-perfect rendering
    pub fn round_vec_to_pixels(&self, vec: Vec2) -> Vec2 {
        self.ctx().round_vec_to_pixels(vec)
    }

    /// Useful for pixel-perfect rendering
    pub fn round_pos_to_pixels(&self, pos: Pos2) -> Pos2 {
        self.ctx().round_pos_to_pixels(pos)
    }
}

/// ## Low level
impl Painter {
    /// It is up to the caller to make sure there is room for this.
    /// Can be used for free painting.
    /// NOTE: all coordinates are screen coordinates!
    pub fn add(&self, shape: Shape) -> ShapeIdx {
        self.ctx
            .graphics()
            .list(self.layer_id)
            .add(self.clip_rect, shape)
    }

    pub fn extend(&self, shapes: Vec<Shape>) {
        self.ctx
            .graphics()
            .list(self.layer_id)
            .extend(self.clip_rect, shapes);
    }

    /// Modify an existing [`Shape`].
    pub fn set(&self, idx: ShapeIdx, shape: Shape) {
        self.ctx
            .graphics()
            .list(self.layer_id)
            .set(idx, self.clip_rect, shape)
    }
}

/// ## Debug painting
impl Painter {
    pub fn debug_rect(&mut self, rect: Rect, color: Color32, text: impl Into<String>) {
        self.rect_stroke(rect, 0.0, (1.0, color));
        let text_style = TextStyle::Monospace;
        self.text(rect.min, Align2::LEFT_TOP, text.into(), text_style, color);
    }

    pub fn error(&self, pos: Pos2, text: impl std::fmt::Display) -> Rect {
        let text_style = TextStyle::Monospace;
        let font = &self.fonts()[text_style];
        let galley = font.layout_multiline(format!("🔥 {}", text), f32::INFINITY);
        let rect = Align2::LEFT_TOP.anchor_rect(Rect::from_min_size(pos, galley.size));
        let frame_rect = rect.expand(2.0);
        self.add(Shape::Rect {
            rect: frame_rect,
            corner_radius: 0.0,
            fill: Color32::from_black_alpha(240),
            stroke: Stroke::new(1.0, Color32::RED),
        });
        self.galley(rect.min, galley, text_style, Color32::RED);
        frame_rect
    }
}

/// # Paint different primitives
impl Painter {
    pub fn line_segment(&self, points: [Pos2; 2], stroke: impl Into<Stroke>) {
        self.add(Shape::LineSegment {
            points,
            stroke: stroke.into(),
        });
    }

    pub fn circle(
        &self,
        center: Pos2,
        radius: f32,
        fill_color: impl Into<Color32>,
        stroke: impl Into<Stroke>,
    ) {
        self.add(Shape::Circle {
            center,
            radius,
            fill: fill_color.into(),
            stroke: stroke.into(),
        });
    }

    pub fn circle_filled(&self, center: Pos2, radius: f32, fill_color: impl Into<Color32>) {
        self.add(Shape::Circle {
            center,
            radius,
            fill: fill_color.into(),
            stroke: Default::default(),
        });
    }

    pub fn circle_stroke(&self, center: Pos2, radius: f32, stroke: impl Into<Stroke>) {
        self.add(Shape::Circle {
            center,
            radius,
            fill: Default::default(),
            stroke: stroke.into(),
        });
    }

    pub fn rect(
        &self,
        rect: Rect,
        corner_radius: f32,
        fill_color: impl Into<Color32>,
        stroke: impl Into<Stroke>,
    ) {
        self.add(Shape::Rect {
            rect,
            corner_radius,
            fill: fill_color.into(),
            stroke: stroke.into(),
        });
    }

    pub fn rect_filled(&self, rect: Rect, corner_radius: f32, fill_color: impl Into<Color32>) {
        self.add(Shape::Rect {
            rect,
            corner_radius,
            fill: fill_color.into(),
            stroke: Default::default(),
        });
    }

    pub fn rect_stroke(&self, rect: Rect, corner_radius: f32, stroke: impl Into<Stroke>) {
        self.add(Shape::Rect {
            rect,
            corner_radius,
            fill: Default::default(),
            stroke: stroke.into(),
        });
    }

    /// Show an arrow starting at `origin` and going in the direction of `vec`, with the length `vec.length()`.
    pub fn arrow(&self, origin: Pos2, vec: Vec2, stroke: Stroke) {
        use crate::math::*;
        let rot = Rot2::from_angle(std::f32::consts::TAU / 10.0);
        let tip_length = vec.length() / 4.0;
        let tip = origin + vec;
        let dir = vec.normalized();
        self.line_segment([origin, tip], stroke);
        self.line_segment([tip, tip - tip_length * (rot * dir)], stroke);
        self.line_segment([tip, tip - tip_length * (rot.inverse() * dir)], stroke);
    }
}

/// ## Text
impl Painter {
    /// Lay out and paint some text.
    ///
    /// To center the text at the given position, use `anchor: (Center, Center)`.
    ///
    /// Returns where the text ended up.
    pub fn text(
        &self,
        pos: Pos2,
        anchor: Align2,
        text: impl Into<String>,
        text_style: TextStyle,
        text_color: Color32,
    ) -> Rect {
        let font = &self.fonts()[text_style];
        let galley = font.layout_multiline(text.into(), f32::INFINITY);
        let rect = anchor.anchor_rect(Rect::from_min_size(pos, galley.size));
        self.galley(rect.min, galley, text_style, text_color);
        rect
    }

    /// Paint text that has already been layed out in a `Galley`.
    pub fn galley(&self, pos: Pos2, galley: Galley, text_style: TextStyle, color: Color32) {
        self.add(Shape::Text {
            pos,
            galley,
            text_style,
            color,
        });
    }
}
