use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    align::{anchor_rect, Align, LEFT_TOP},
    color,
    layers::PaintCmdIdx,
    math::{Pos2, Rect, Vec2},
    paint::fonts::GlyphLayout,
    paint::{Fonts, PaintCmd, Stroke, TextStyle},
    Context, Layer, Srgba,
};

/// Helper to paint shapes and text to a specific region on a specific layer.
#[derive(Clone)]
pub struct Painter {
    /// Source of fonts and destination of paint commands
    ctx: Arc<Context>,

    /// Where we paint
    layer: Layer,

    /// Everything painted in this `Painter` will be clipped against this.
    /// This means nothing outside of this rectangle will be visible on screen.
    clip_rect: Rect,
}

impl Painter {
    pub fn new(ctx: Arc<Context>, layer: Layer, clip_rect: Rect) -> Self {
        Self {
            ctx,
            layer,
            clip_rect,
        }
    }

    /// Create a painter for a sub-region of this `Painter`.
    ///
    /// The clip-rect of the returned `Painter` will be the intersection
    /// of the given rectangle and the `clip_rect()` of this `Painter`.
    pub fn sub_region(&self, rect: Rect) -> Self {
        Self::new(self.ctx.clone(), self.layer, rect.intersect(self.clip_rect))
    }
}

/// ## Accessors etc
impl Painter {
    pub(crate) fn ctx(&self) -> &Arc<Context> {
        &self.ctx
    }

    /// Available fonts
    pub(crate) fn fonts(&self) -> Arc<Mutex<Fonts>> {
        self.ctx.fonts()
    }

    /// Where we paint
    pub fn layer(&self) -> Layer {
        self.layer
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
    pub fn add(&self, paint_cmd: PaintCmd) -> PaintCmdIdx {
        self.ctx
            .graphics()
            .list(self.layer)
            .add(self.clip_rect, paint_cmd)
    }

    pub fn extend(&self, cmds: Vec<PaintCmd>) {
        self.ctx
            .graphics()
            .list(self.layer)
            .extend(self.clip_rect, cmds);
    }

    /// Modify an existing command.
    pub fn set(&self, idx: PaintCmdIdx, cmd: PaintCmd) {
        self.ctx
            .graphics()
            .list(self.layer)
            .set(idx, self.clip_rect, cmd)
    }
}

/// ## Debug painting
impl Painter {
    pub fn debug_rect(&mut self, rect: Rect, color: Srgba, text: &str) {
        self.rect_stroke(rect, 0.0, (1.0, color));
        let text_style = TextStyle::Monospace;
        self.text(rect.min, LEFT_TOP, text, text_style, color);
    }

    pub fn error(&self, pos: Pos2, text: &str) {
        let text_style = TextStyle::Monospace;
        let layout = self.fonts().lock().layout_multiline(text_style, text, None);
        let rect = anchor_rect(Rect::from_min_size(pos, layout.size), LEFT_TOP);
        self.add(PaintCmd::Rect {
            rect: rect.expand(2.0),
            corner_radius: 0.0,
            fill: Srgba::black_alpha(240),
            stroke: Stroke::new(1.0, color::RED),
        });
        self.layout(rect.min, layout, text_style, color::RED);
    }

    pub fn debug_arrow(&self, origin: Pos2, dir: Vec2, stroke: Stroke) {
        use crate::math::*;
        let full_length = dir.length().at_least(64.0);
        let tip_length = full_length / 3.0;
        let dir = dir.normalized();
        let tip = origin + dir * full_length;
        let angle = TAU / 10.0;
        self.line_segment([origin, tip], stroke);
        self.line_segment(
            [
                tip,
                tip - tip_length * Vec2::angled(angle).rotate_other(dir),
            ],
            stroke,
        );
        self.line_segment(
            [
                tip,
                tip - tip_length * Vec2::angled(-angle).rotate_other(dir),
            ],
            stroke,
        );
    }
}

/// # Paint different primitives
impl Painter {
    pub fn line_segment(&self, points: [Pos2; 2], stroke: impl Into<Stroke>) {
        self.add(PaintCmd::LineSegment {
            points,
            stroke: stroke.into(),
        });
    }

    pub fn circle(
        &self,
        center: Pos2,
        radius: f32,
        fill_color: impl Into<Srgba>,
        stroke: impl Into<Stroke>,
    ) {
        self.add(PaintCmd::Circle {
            center,
            radius,
            fill: fill_color.into(),
            stroke: stroke.into(),
        });
    }

    pub fn circle_filled(&self, center: Pos2, radius: f32, fill_color: impl Into<Srgba>) {
        self.add(PaintCmd::Circle {
            center,
            radius,
            fill: fill_color.into(),
            stroke: Default::default(),
        });
    }

    pub fn circle_stroke(&self, center: Pos2, radius: f32, stroke: impl Into<Stroke>) {
        self.add(PaintCmd::Circle {
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
        fill_color: impl Into<Srgba>,
        stroke: impl Into<Stroke>,
    ) {
        self.add(PaintCmd::Rect {
            rect,
            corner_radius,
            fill: fill_color.into(),
            stroke: stroke.into(),
        });
    }

    pub fn rect_filled(&self, rect: Rect, corner_radius: f32, fill_color: impl Into<Srgba>) {
        self.add(PaintCmd::Rect {
            rect,
            corner_radius,
            fill: fill_color.into(),
            stroke: Default::default(),
        });
    }

    pub fn rect_stroke(&self, rect: Rect, corner_radius: f32, stroke: impl Into<Stroke>) {
        self.add(PaintCmd::Rect {
            rect,
            corner_radius,
            fill: Default::default(),
            stroke: stroke.into(),
        });
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
        anchor: (Align, Align),
        text: &str,
        text_style: TextStyle,
        text_color: Srgba,
    ) -> Rect {
        let layout = self.fonts().lock().layout_multiline(text_style, text, None);
        let rect = anchor_rect(Rect::from_min_size(pos, layout.size), anchor);
        self.layout(rect.min, layout, text_style, text_color);
        rect
    }

    /// Paint text that has already been laid out in a `GlyphLayout`.
    pub fn layout(&self, pos: Pos2, layout: GlyphLayout, text_style: TextStyle, color: Srgba) {
        self.add(PaintCmd::Text {
            pos,
            layout,
            text_style,
            color,
        });
    }
}
