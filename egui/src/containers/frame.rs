//! Frame container

use crate::{layers::ShapeIdx, *};
use epaint::*;

/// Color and margin of a rectangular background of a [`Ui`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Frame {
    /// On each side
    pub margin: Vec2,
    pub corner_radius: f32,
    pub shadow: Shadow,
    pub fill: Color32,
    pub stroke: Stroke,
}

impl Frame {
    pub fn none() -> Self {
        Self::default()
    }

    /// For when you want to group a few widgets together within a frame.
    pub fn group(style: &Style) -> Self {
        Self {
            margin: Vec2::new(8.0, 6.0),
            corner_radius: 4.0,
            stroke: style.visuals.window_stroke(),
            ..Default::default()
        }
    }

    pub(crate) fn side_top_panel(style: &Style) -> Self {
        Self {
            margin: Vec2::new(8.0, 2.0),
            corner_radius: 0.0,
            fill: style.visuals.window_fill(),
            stroke: style.visuals.window_stroke(),
            ..Default::default()
        }
    }

    pub(crate) fn central_panel(style: &Style) -> Self {
        Self {
            margin: Vec2::new(8.0, 8.0),
            corner_radius: 0.0,
            fill: style.visuals.window_fill(),
            stroke: Default::default(),
            ..Default::default()
        }
    }

    pub fn window(style: &Style) -> Self {
        Self {
            margin: style.spacing.window_padding,
            corner_radius: style.visuals.window_corner_radius,
            shadow: style.visuals.window_shadow,
            fill: style.visuals.window_fill(),
            stroke: style.visuals.window_stroke(),
        }
    }

    pub fn menu(style: &Style) -> Self {
        Self {
            margin: Vec2::splat(1.0),
            corner_radius: 2.0,
            shadow: Shadow::small(),
            fill: style.visuals.window_fill(),
            stroke: style.visuals.window_stroke(),
        }
    }

    pub fn popup(style: &Style) -> Self {
        Self {
            margin: style.spacing.window_padding,
            corner_radius: 5.0,
            shadow: Shadow::small(),
            fill: style.visuals.window_fill(),
            stroke: style.visuals.window_stroke(),
        }
    }

    /// dark canvas to draw on
    pub fn dark_canvas(style: &Style) -> Self {
        Self {
            margin: Vec2::new(10.0, 10.0),
            corner_radius: 5.0,
            fill: Color32::from_black_alpha(250),
            stroke: style.visuals.window_stroke(),
            ..Default::default()
        }
    }
}

impl Frame {
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = fill;
        self
    }

    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
        self
    }

    pub fn multiply_with_opacity(mut self, opacity: f32) -> Self {
        self.fill = self.fill.linear_multiply(opacity);
        self.stroke.color = self.stroke.color.linear_multiply(opacity);
        self.shadow.color = self.shadow.color.linear_multiply(opacity);
        self
    }
}

pub struct Prepared {
    pub frame: Frame,
    outer_rect_bounds: Rect,
    where_to_put_background: ShapeIdx,
    pub content_ui: Ui,
}

impl Frame {
    pub fn begin(self, ui: &mut Ui) -> Prepared {
        let where_to_put_background = ui.painter().add(Shape::Noop);
        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = outer_rect_bounds.shrink2(self.margin);
        let content_ui = ui.child_ui(inner_rect, *ui.layout());

        // content_ui.set_clip_rect(outer_rect_bounds.shrink(self.stroke.width * 0.5)); // Can't do this since we don't know final size yet

        Prepared {
            frame: self,
            outer_rect_bounds,
            where_to_put_background,
            content_ui,
        }
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        let mut prepared = self.begin(ui);
        let ret = add_contents(&mut prepared.content_ui);
        prepared.end(ui);
        ret
    }

    pub fn paint(&self, outer_rect: Rect) -> Shape {
        let Self {
            margin: _,
            corner_radius,
            shadow,
            fill,
            stroke,
        } = *self;

        let frame_shape = Shape::Rect {
            rect: outer_rect,
            corner_radius,
            fill,
            stroke,
        };

        if shadow == Default::default() {
            frame_shape
        } else {
            let shadow = shadow.tessellate(outer_rect, corner_radius);
            let shadow = Shape::Mesh(shadow);
            Shape::Vec(vec![shadow, frame_shape])
        }
    }
}

impl Prepared {
    pub fn outer_rect(&self) -> Rect {
        Rect::from_min_max(
            self.outer_rect_bounds.min,
            self.content_ui.min_rect().max + self.frame.margin,
        )
    }

    pub fn end(self, ui: &mut Ui) -> Rect {
        let outer_rect = self.outer_rect();

        let Prepared {
            frame,
            where_to_put_background,
            ..
        } = self;

        let shape = frame.paint(outer_rect);
        ui.painter().set(where_to_put_background, shape);
        ui.advance_cursor_after_rect(outer_rect);
        outer_rect
    }
}
