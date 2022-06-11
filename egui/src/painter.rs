use std::{ops::RangeInclusive, rc::Rc};

use crate::{
    emath::{pos2, Align2, Pos2, Rect, Vec2},
    layers::{LayerId, PaintList, ShapeIdx},
    Color32, Context, FontId,
};
use epaint::{text::Galley, CircleShape, RectShape, Rounding, Shape, Stroke};

/// Helper to paint shapes and text to a specific region on a specific layer.
///
/// All coordinates are screen coordinates in the unit points (one point can consist of many physical pixels).
#[derive(Clone)]
pub struct Painter {
    /// Where we paint
    layer_id: LayerId,

    /// Everything painted in this [`Painter`] will be clipped against this.
    /// This means nothing outside of this rectangle will be visible on screen.
    clip_rect: Rect,

    /// If set, all shapes will have their colors modified to be closer to this.
    /// This is used to implement grayed out interfaces.
    fade_to_color: Option<Color32>,
}

impl Painter {
    /// Create a painter to a specific layer within a certain clip rectangle.
    pub fn new(layer_id: LayerId, clip_rect: Rect) -> Self {
        Self {
            layer_id,
            clip_rect,
            fade_to_color: None,
        }
    }

    /// Redirect where you are painting.
    #[must_use]
    pub fn with_layer_id(self, layer_id: LayerId) -> Self {
        Self {
            layer_id,
            clip_rect: self.clip_rect,
            fade_to_color: None,
        }
    }

    /// Create a painter for a sub-region of this [`Painter`].
    ///
    /// The clip-rect of the returned [`Painter`] will be the intersection
    /// of the given rectangle and the `clip_rect()` of the parent [`Painter`].
    pub fn with_clip_rect(&self, rect: Rect) -> Self {
        Self {
            layer_id: self.layer_id,
            clip_rect: rect.intersect(self.clip_rect),
            fade_to_color: self.fade_to_color,
        }
    }

    /// Redirect where you are painting.
    pub fn set_layer_id(&mut self, layer_id: LayerId) {
        self.layer_id = layer_id;
    }

    /// If set, colors will be modified to look like this
    pub(crate) fn set_fade_to_color(&mut self, fade_to_color: Option<Color32>) {
        self.fade_to_color = fade_to_color;
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.fade_to_color != Some(Color32::TRANSPARENT)
    }

    /// If `false`, nothing added to the painter will be visible
    pub(crate) fn set_invisible(&mut self) {
        self.fade_to_color = Some(Color32::TRANSPARENT);
    }

    #[deprecated = "Use Painter::with_clip_rect"] // Deprecated in 2022-04-18, before egui 0.18
    pub fn sub_region(&self, rect: Rect) -> Self {
        Self {
            layer_id: self.layer_id,
            clip_rect: rect.intersect(self.clip_rect),
            fade_to_color: self.fade_to_color,
        }
    }
}

/// ## Accessors etc
impl Painter {
    /// Where we paint
    #[inline(always)]
    pub fn layer_id(&self) -> LayerId {
        self.layer_id
    }

    /// Everything painted in this [`Painter`] will be clipped against this.
    /// This means nothing outside of this rectangle will be visible on screen.
    #[inline(always)]
    pub fn clip_rect(&self) -> Rect {
        self.clip_rect
    }

    /// Everything painted in this [`Painter`] will be clipped against this.
    /// This means nothing outside of this rectangle will be visible on screen.
    #[inline(always)]
    pub fn set_clip_rect(&mut self, clip_rect: Rect) {
        self.clip_rect = clip_rect;
    }
}

/// ## Low level
impl Painter {
    fn paint_list<'a>(&self, ctx: &'a mut Context) -> &'a mut PaintList {
        ctx.graphics_mut().list(self.layer_id)
    }

    fn transform_shape(&self, shape: &mut Shape) {
        if let Some(fade_to_color) = self.fade_to_color {
            tint_shape_towards(shape, fade_to_color);
        }
    }

    /// It is up to the caller to make sure there is room for this.
    /// Can be used for free painting.
    /// NOTE: all coordinates are screen coordinates!
    pub fn add(&self, ctx: &mut Context, shape: impl Into<Shape>) -> ShapeIdx {
        if self.fade_to_color == Some(Color32::TRANSPARENT) {
            self.paint_list(ctx).add(self.clip_rect, Shape::Noop)
        } else {
            let mut shape = shape.into();
            self.transform_shape(&mut shape);
            self.paint_list(ctx).add(self.clip_rect, shape)
        }
    }

    /// Add many shapes at once.
    ///
    /// Calling this once is generally faster than calling [`Self::add`] multiple times.
    pub fn extend(&self, ctx: &mut Context, mut shapes: Vec<Shape>) {
        if self.fade_to_color == Some(Color32::TRANSPARENT) {
            return;
        }
        if !shapes.is_empty() {
            if self.fade_to_color.is_some() {
                for shape in &mut shapes {
                    self.transform_shape(shape);
                }
            }

            self.paint_list(ctx).extend(self.clip_rect, shapes);
        }
    }

    /// Modify an existing [`Shape`].
    pub fn set(&self, ctx: &mut Context, idx: ShapeIdx, shape: impl Into<Shape>) {
        if self.fade_to_color == Some(Color32::TRANSPARENT) {
            return;
        }
        let mut shape = shape.into();
        self.transform_shape(&mut shape);
        self.paint_list(ctx).set(idx, self.clip_rect, shape);
    }
}

/// ## Debug painting
impl Painter {
    pub fn debug_rect(
        &self,
        ctx: &mut Context,
        rect: Rect,
        color: Color32,
        text: impl Into<String>,
    ) {
        self.rect_stroke(ctx, rect, 0.0, (1.0, color));
        self.text(
            ctx,
            rect.min,
            Align2::LEFT_TOP,
            text.into(),
            FontId::monospace(14.0),
            color,
        );
    }

    pub fn error(&self, ctx: &mut Context, pos: Pos2, text: impl std::fmt::Display) -> Rect {
        self.debug_text(
            ctx,
            pos,
            Align2::LEFT_TOP,
            Color32::RED,
            format!("🔥 {}", text),
        )
    }

    /// text with a background
    pub fn debug_text(
        &self,
        ctx: &mut Context,
        pos: Pos2,
        anchor: Align2,
        color: Color32,
        text: impl Into<String>,
    ) -> Rect {
        let galley = ctx
            .fonts()
            .layout_no_wrap(text.into(), FontId::monospace(14.0), color);
        let rect = anchor.anchor_rect(Rect::from_min_size(pos, galley.size()));
        let frame_rect = rect.expand(2.0);
        self.add(
            ctx,
            Shape::rect_filled(frame_rect, 0.0, Color32::from_black_alpha(240)),
        );
        self.galley(ctx, rect.min, galley);
        frame_rect
    }
}

/// # Paint different primitives
impl Painter {
    /// Paints a line from the first point to the second.
    pub fn line_segment(&self, ctx: &mut Context, points: [Pos2; 2], stroke: impl Into<Stroke>) {
        self.add(
            ctx,
            Shape::LineSegment {
                points,
                stroke: stroke.into(),
            },
        );
    }

    /// Paints a horizontal line.
    pub fn hline(
        &self,
        ctx: &mut Context,
        x: RangeInclusive<f32>,
        y: f32,
        stroke: impl Into<Stroke>,
    ) {
        self.add(
            ctx,
            Shape::LineSegment {
                points: [pos2(*x.start(), y), pos2(*x.end(), y)],
                stroke: stroke.into(),
            },
        );
    }

    /// Paints a vertical line.
    pub fn vline(
        &self,
        ctx: &mut Context,
        x: f32,
        y: RangeInclusive<f32>,
        stroke: impl Into<Stroke>,
    ) {
        self.add(
            ctx,
            Shape::LineSegment {
                points: [pos2(x, *y.start()), pos2(x, *y.end())],
                stroke: stroke.into(),
            },
        );
    }

    pub fn circle(
        &self,
        ctx: &mut Context,
        center: Pos2,
        radius: f32,
        fill_color: impl Into<Color32>,
        stroke: impl Into<Stroke>,
    ) {
        self.add(
            ctx,
            CircleShape {
                center,
                radius,
                fill: fill_color.into(),
                stroke: stroke.into(),
            },
        );
    }

    pub fn circle_filled(
        &self,
        ctx: &mut Context,
        center: Pos2,
        radius: f32,
        fill_color: impl Into<Color32>,
    ) {
        self.add(
            ctx,
            CircleShape {
                center,
                radius,
                fill: fill_color.into(),
                stroke: Default::default(),
            },
        );
    }

    pub fn circle_stroke(
        &self,
        ctx: &mut Context,
        center: Pos2,
        radius: f32,
        stroke: impl Into<Stroke>,
    ) {
        self.add(
            ctx,
            CircleShape {
                center,
                radius,
                fill: Default::default(),
                stroke: stroke.into(),
            },
        );
    }

    pub fn rect(
        &self,
        ctx: &mut Context,
        rect: Rect,
        rounding: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
        stroke: impl Into<Stroke>,
    ) {
        self.add(
            ctx,
            RectShape {
                rect,
                rounding: rounding.into(),
                fill: fill_color.into(),
                stroke: stroke.into(),
            },
        );
    }

    pub fn rect_filled(
        &self,
        ctx: &mut Context,
        rect: Rect,
        rounding: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
    ) {
        self.add(
            ctx,
            RectShape {
                rect,
                rounding: rounding.into(),
                fill: fill_color.into(),
                stroke: Default::default(),
            },
        );
    }

    pub fn rect_stroke(
        &self,
        ctx: &mut Context,
        rect: Rect,
        rounding: impl Into<Rounding>,
        stroke: impl Into<Stroke>,
    ) {
        self.add(
            ctx,
            RectShape {
                rect,
                rounding: rounding.into(),
                fill: Default::default(),
                stroke: stroke.into(),
            },
        );
    }

    /// Show an arrow starting at `origin` and going in the direction of `vec`, with the length `vec.length()`.
    pub fn arrow(&self, ctx: &mut Context, origin: Pos2, vec: Vec2, stroke: Stroke) {
        use crate::emath::*;
        let rot = Rot2::from_angle(std::f32::consts::TAU / 10.0);
        let tip_length = vec.length() / 4.0;
        let tip = origin + vec;
        let dir = vec.normalized();
        self.line_segment(ctx, [origin, tip], stroke);
        self.line_segment(ctx, [tip, tip - tip_length * (rot * dir)], stroke);
        self.line_segment(ctx, [tip, tip - tip_length * (rot.inverse() * dir)], stroke);
    }
}

/// ## Text
impl Painter {
    /// Lay out and paint some text.
    ///
    /// To center the text at the given position, use `Align2::CENTER_CENTER`.
    ///
    /// To find out the size of text before painting it, use
    /// [`Self::layout`] or [`Self::layout_no_wrap`].
    ///
    /// Returns where the text ended up.
    #[allow(clippy::needless_pass_by_value)]
    pub fn text(
        &self,
        ctx: &mut Context,
        pos: Pos2,
        anchor: Align2,
        text: impl ToString,
        font_id: FontId,
        text_color: Color32,
    ) -> Rect {
        let galley = ctx
            .fonts()
            .layout_no_wrap(text.to_string(), font_id, text_color);
        let rect = anchor.anchor_rect(Rect::from_min_size(pos, galley.size()));
        self.galley(ctx, rect.min, galley);
        rect
    }

    /// Paint text that has already been layed out in a [`Galley`].
    ///
    /// You can create the [`Galley`] with [`Self::layout`].
    ///
    /// If you want to change the color of the text, use [`Self::galley_with_color`].
    #[inline(always)]
    pub fn galley(&self, ctx: &mut Context, pos: Pos2, galley: Rc<Galley>) {
        if !galley.is_empty() {
            self.add(ctx, Shape::galley(pos, galley));
        }
    }

    /// Paint text that has already been layed out in a [`Galley`].
    ///
    /// You can create the [`Galley`] with [`Self::layout`].
    ///
    /// The text color in the [`Galley`] will be replaced with the given color.
    #[inline(always)]
    pub fn galley_with_color(
        &self,
        ctx: &mut Context,
        pos: Pos2,
        galley: Rc<Galley>,
        text_color: Color32,
    ) {
        if !galley.is_empty() {
            self.add(ctx, Shape::galley_with_color(pos, galley, text_color));
        }
    }
}

fn tint_shape_towards(shape: &mut Shape, target: Color32) {
    epaint::shape_transform::adjust_colors(shape, &|color| {
        *color = crate::color::tint_color_towards(*color, target);
    });
}
