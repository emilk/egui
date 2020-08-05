use std::sync::Arc;

use crate::{
    anchor_rect, color,
    layers::PaintCmdIdx,
    math::{Pos2, Rect, Vec2},
    paint::{font, Fonts, LineStyle, PaintCmd, TextStyle},
    Align, Color, Context, Layer,
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
    pub(crate) fn fonts(&self) -> &Fonts {
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
    pub fn debug_rect(&mut self, rect: Rect, color: Color, text: impl Into<String>) {
        self.add(PaintCmd::Rect {
            corner_radius: 0.0,
            fill: None,
            outline: Some(LineStyle::new(1.0, color)),
            rect,
        });
        let anchor = (Align::Min, Align::Min);
        let text_style = TextStyle::Monospace;
        self.text(rect.min, anchor, text.into(), text_style, color);
    }

    pub fn error(&self, pos: Pos2, text: impl Into<String>) {
        let text = text.into();
        let anchor = (Align::Min, Align::Min);
        let text_style = TextStyle::Monospace;
        let font = &self.fonts()[text_style];
        let galley = font.layout_multiline(text, f32::INFINITY);
        let rect = anchor_rect(Rect::from_min_size(pos, galley.size), anchor);
        self.add(PaintCmd::Rect {
            corner_radius: 0.0,
            fill: Some(color::gray(0, 240)),
            outline: Some(LineStyle::new(1.0, color::RED)),
            rect: rect.expand(2.0),
        });
        self.galley(rect.min, galley, text_style, color::RED);
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
        text: impl Into<String>,
        text_style: TextStyle,
        text_color: Color,
    ) -> Rect {
        let font = &self.fonts()[text_style];
        let galley = font.layout_multiline(text.into(), f32::INFINITY);
        let rect = anchor_rect(Rect::from_min_size(pos, galley.size), anchor);
        self.galley(rect.min, galley, text_style, text_color);
        rect
    }

    /// Paint text that has already been layed out in a `Galley`.
    pub fn galley(&self, pos: Pos2, galley: font::Galley, text_style: TextStyle, color: Color) {
        self.add(PaintCmd::Text {
            pos,
            galley,
            text_style,
            color,
        });
    }
}
